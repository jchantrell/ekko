use anyhow::{bail, Context, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::types::*;
use crate::config::Config;

/// Graphiti client that talks to the ekko daemon over a Unix socket.
pub struct DaemonClient {
    writer: tokio::io::WriteHalf<UnixStream>,
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    next_id: AtomicU64,
}

#[derive(serde::Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(serde::Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

impl DaemonClient {
    /// Connect to the daemon socket.
    #[allow(dead_code)]
    pub async fn connect() -> Result<Self> {
        let socket_path = Config::socket_path()?;
        let stream = UnixStream::connect(&socket_path)
            .await
            .with_context(|| format!("failed to connect to daemon at {}", socket_path.display()))?;

        let (read_half, write_half) = tokio::io::split(stream);

        Ok(Self {
            writer: write_half,
            reader: BufReader::new(read_half),
            next_id: AtomicU64::new(1),
        })
    }

    /// Connect with retry and exponential backoff.
    #[allow(dead_code)]
    pub async fn connect_with_retry(max_attempts: u32) -> Result<Self> {
        let mut delay = std::time::Duration::from_millis(100);

        for attempt in 1..=max_attempts {
            match Self::connect().await {
                Ok(client) => return Ok(client),
                Err(e) if attempt == max_attempts => return Err(e),
                Err(_) => {
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(std::time::Duration::from_secs(1));
                }
            }
        }

        bail!("failed to connect to daemon after {max_attempts} attempts")
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    async fn call_raw(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let req = serde_json::json!({
            "id": self.next_id(),
            "method": method,
            "params": params,
        });

        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .await
            .context("failed to write to daemon socket")?;
        self.writer.flush().await?;

        let mut response_line = String::new();
        let n = self
            .reader
            .read_line(&mut response_line)
            .await
            .context("failed to read from daemon socket")?;

        if n == 0 {
            bail!("daemon connection closed unexpectedly");
        }

        let resp: JsonRpcResponse = serde_json::from_str(&response_line)
            .with_context(|| format!("failed to parse daemon response: {response_line}"))?;

        if let Some(err) = resp.error {
            bail!("{}", err.message);
        }

        resp.result.context("missing result in daemon response")
    }

    async fn call_json<T: serde::de::DeserializeOwned>(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T> {
        let val = self.call_raw(method, params).await?;

        if let Ok(err) = serde_json::from_value::<ErrorResponse>(val.clone()) {
            bail!("{}", err.error);
        }

        serde_json::from_value(val)
            .with_context(|| format!("failed to deserialize response from '{method}'"))
    }

    pub async fn add_memory(&mut self, req: AddMemoryRequest) -> Result<SuccessResponse> {
        self.call_json("add_memory", serde_json::to_value(req)?)
            .await
    }

    pub async fn search_nodes(&mut self, req: SearchNodesRequest) -> Result<NodeSearchResponse> {
        self.call_json("search_nodes", serde_json::to_value(req)?)
            .await
    }

    pub async fn search_facts(&mut self, req: SearchFactsRequest) -> Result<FactSearchResponse> {
        self.call_json("search_facts", serde_json::to_value(req)?)
            .await
    }

    pub async fn get_episodes(
        &mut self,
        req: GetEpisodesRequest,
    ) -> Result<EpisodeSearchResponse> {
        self.call_json("get_episodes", serde_json::to_value(req)?)
            .await
    }

    pub async fn get_edge(&mut self, uuid: &str) -> Result<Fact> {
        self.call_json("get_entity_edge", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn delete_edge(&mut self, uuid: &str) -> Result<SuccessResponse> {
        self.call_json("delete_entity_edge", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn delete_episode(&mut self, uuid: &str) -> Result<SuccessResponse> {
        self.call_json("delete_episode", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn clear_graph(&mut self, req: ClearGraphRequest) -> Result<SuccessResponse> {
        self.call_json("clear_graph", serde_json::to_value(req)?)
            .await
    }

    pub async fn list_origins(&mut self, req: ListOriginsRequest) -> Result<ListOriginsResponse> {
        self.call_json("list_groups", serde_json::to_value(req)?)
            .await
    }

    pub async fn status(&mut self) -> Result<StatusResponse> {
        self.call_json("status", serde_json::json!({})).await
    }

    pub async fn queue_status(&mut self) -> Result<QueueStatusResponse> {
        self.call_json("queue_status", serde_json::json!({})).await
    }

    pub async fn health(&mut self) -> Result<bool> {
        match self.call_raw("health", serde_json::json!({})).await {
            Ok(val) => {
                let status = val
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("error");
                Ok(status == "ok")
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        let _ = self.call_raw("shutdown", serde_json::json!({})).await;
        Ok(())
    }
}
