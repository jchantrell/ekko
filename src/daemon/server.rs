use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::graphiti::direct::DirectClient;

use super::lifecycle;

pub struct DaemonState {
    client: Mutex<DirectClient>,
    busy: AtomicBool,
}

/// Run the daemon: own the shim, listen on socket, multiplex requests.
pub async fn run() -> Result<()> {
    if let Some(pid) = lifecycle::running_pid() {
        anyhow::bail!("daemon already running (PID {pid})");
    }

    lifecycle::remove_socket();
    lifecycle::write_pid()?;

    let config = Config::load()?;
    let client = DirectClient::new(&config)
        .await
        .context("failed to start graphiti shim")?;

    let state = std::sync::Arc::new(DaemonState {
        client: Mutex::new(client),
        busy: AtomicBool::new(false),
    });

    let socket_path = Config::socket_path()?;
    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("failed to bind {}", socket_path.display()))?;

    eprintln!("ekko daemon listening on {}", socket_path.display());

    tokio::spawn(super::sync::run_with_state(state.clone()));

    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => {
                        let state = state.clone();
                        tokio::spawn(handle_connection(stream, state));
                    }
                    Err(e) => {
                        eprintln!("accept error: {e}");
                    }
                }
            }
            _ = &mut shutdown => {
                eprintln!("shutting down...");
                break;
            }
        }
    }

    let mut c = state.client.lock().await;
    let _ = c.shutdown().await;
    lifecycle::remove_socket();
    lifecycle::remove_pid();

    Ok(())
}

async fn respawn_shim(state: &DaemonState) -> bool {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config for shim respawn: {e}");
            return false;
        }
    };

    match DirectClient::new(&config).await {
        Ok(new_client) => {
            let mut c = state.client.lock().await;
            *c = new_client;
            eprintln!("shim respawned successfully");
            true
        }
        Err(e) => {
            eprintln!("failed to respawn shim: {e}");
            false
        }
    }
}

/// Methods that are fast and can use try_lock instead of blocking.
fn is_lightweight(method: &str) -> bool {
    matches!(
        method,
        "queue_status" | "health" | "status" | "list_groups"
    )
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    state: std::sync::Arc<DaemonState>,
) {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let n = match reader.read_line(&mut line).await {
            Ok(n) => n,
            Err(_) => break,
        };

        if n == 0 {
            break;
        }

        let req: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                let err = serde_json::json!({
                    "id": null,
                    "error": {"code": -32700, "message": "parse error"}
                });
                let _ = write_response(&mut writer, &err).await;
                continue;
            }
        };

        let req_id = req.get("id").cloned();
        let method = req
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(serde_json::json!({}));

        if method == "daemon_shutdown" {
            let resp = serde_json::json!({
                "id": req_id,
                "result": {"message": "shutdown requested"}
            });
            let _ = write_response(&mut writer, &resp).await;
            std::process::exit(0);
        }

        if method == "daemon_status" {
            let resp = serde_json::json!({
                "id": req_id,
                "result": {
                    "status": "running",
                    "pid": std::process::id(),
                    "busy": state.busy.load(Ordering::Relaxed),
                }
            });
            let _ = write_response(&mut writer, &resp).await;
            continue;
        }

        let result = if is_lightweight(method) {
            // Non-blocking: try to get the lock, return shim-busy if held
            match state.client.try_lock() {
                Ok(mut c) => c.call_raw(method, params).await,
                Err(_) => {
                    // Shim is busy with another request — return a synthetic response
                    let resp = match method {
                        "queue_status" => serde_json::json!({
                            "id": req_id,
                            "result": {"groups": [], "daemon_busy": true}
                        }),
                        "health" => serde_json::json!({
                            "id": req_id,
                            "result": {"status": "busy"}
                        }),
                        _ => serde_json::json!({
                            "id": req_id,
                            "result": {"status": "busy", "message": "shim is processing another request"}
                        }),
                    };
                    let _ = write_response(&mut writer, &resp).await;
                    continue;
                }
            }
        } else {
            // Blocking: wait for the lock
            state.busy.store(true, Ordering::Relaxed);
            let result = {
                let mut c = state.client.lock().await;
                c.call_raw(method, params.clone()).await
            };
            state.busy.store(false, Ordering::Relaxed);

            match result {
                Ok(val) => Ok(val),
                Err(e) if e.to_string().contains("shim") || e.to_string().contains("stdin") => {
                    eprintln!("shim appears dead ({e}), attempting respawn...");
                    if respawn_shim(&state).await {
                        let mut c = state.client.lock().await;
                        c.call_raw(method, params).await
                    } else {
                        Err(e)
                    }
                }
                Err(e) => Err(e),
            }
        };

        let resp = match result {
            Ok(val) => serde_json::json!({"id": req_id, "result": val}),
            Err(e) => serde_json::json!({"id": req_id, "error": {"code": -1, "message": e.to_string()}}),
        };

        if write_response(&mut writer, &resp).await.is_err() {
            break;
        }
    }
}

async fn write_response(
    writer: &mut tokio::io::WriteHalf<tokio::net::UnixStream>,
    resp: &serde_json::Value,
) -> Result<()> {
    let mut line = serde_json::to_string(resp)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}
