use anyhow::{bail, Context, Result};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

use super::types::*;
use crate::config::Config;

/// Graphiti client that owns a Python shim child process directly.
/// Used by the daemon process. Other consumers should go through DaemonClient.
pub struct DirectClient {
    child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
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

impl DirectClient {
    /// Spawn the Python shim and send the `init` request.
    pub async fn new(config: &Config) -> Result<Self> {
        let python = Config::python_path()?;
        let shim_dir = Config::shim_dir()?;

        if !python.exists() {
            bail!(
                "Python venv not found at {}. Run `ekko init` to set up.",
                python.display()
            );
        }
        if !shim_dir.join("__main__.py").exists() {
            bail!(
                "Shim not found at {}. Run `ekko init` to set up.",
                shim_dir.display()
            );
        }

        let mut child = tokio::process::Command::new(&python)
            .arg(&shim_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .with_context(|| format!("failed to spawn {}", python.display()))?;

        let stdin = child.stdin.take().context("no stdin on child")?;
        let stdout = child.stdout.take().context("no stdout on child")?;
        let reader = BufReader::new(stdout);

        let mut client = Self {
            child,
            stdin,
            reader,
            next_id: AtomicU64::new(1),
        };

        let init_params = serde_json::json!({
            "llm": {
                "provider": config.graphiti.llm.provider,
                "model": config.graphiti.llm.model,
                "api_url": config.graphiti.llm.api_url,
                "api_key": config.graphiti.llm.api_key,
            },
            "embedder": {
                "provider": config.graphiti.embedder.provider,
                "model": config.graphiti.embedder.model,
                "dimensions": config.graphiti.embedder.dimensions,
                "api_url": config.graphiti.embedder.api_url,
                "api_key": config.graphiti.embedder.api_key,
            },
            "database": {
                "provider": config.graphiti.database.provider,
                "uri": config.graphiti.database.uri,
                "user": config.graphiti.database.user,
                "password": config.graphiti.database.password,
                "database": config.graphiti.database.database,
            },
        });

        client
            .call_raw("init", init_params)
            .await
            .context("failed to initialize graphiti shim")?;

        Ok(client)
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    pub async fn call_raw(
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
        self.stdin
            .write_all(line.as_bytes())
            .await
            .context("failed to write to shim stdin")?;
        self.stdin.flush().await?;

        let mut response_line = String::new();
        let n = self
            .reader
            .read_line(&mut response_line)
            .await
            .context("failed to read from shim stdout")?;

        if n == 0 {
            bail!("shim process exited unexpectedly");
        }

        let resp: JsonRpcResponse = serde_json::from_str(&response_line)
            .with_context(|| format!("failed to parse shim response: {response_line}"))?;

        if let Some(err) = resp.error {
            bail!("{}", err.message);
        }

        resp.result.context("missing result in shim response")
    }

    pub async fn call_json<T: serde::de::DeserializeOwned>(
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

    pub async fn list_groups(&mut self, req: ListGroupsRequest) -> Result<ListGroupsResponse> {
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
        let _ = self.child.wait().await;
        Ok(())
    }
}

impl Drop for DirectClient {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}
