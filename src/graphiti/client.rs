use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

use super::types::*;

const MCP_SESSION_HEADER: &str = "mcp-session-id";

pub struct Client {
    http: reqwest::Client,
    base_url: String,
    session_id: Option<String>,
    next_id: AtomicU64,
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct ToolResult {
    content: Vec<ToolContent>,
    #[serde(default)]
    #[allow(dead_code)]
    is_error: bool,
}

#[derive(Deserialize)]
struct ToolContent {
    #[allow(dead_code)]
    r#type: String,
    text: String,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            session_id: None,
            next_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn endpoint(&self) -> String {
        format!("{}/mcp/", self.base_url)
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        if let Some(ref sid) = self.session_id {
            if let Ok(val) = HeaderValue::from_str(sid) {
                headers.insert(MCP_SESSION_HEADER, val);
            }
        }
        headers
    }

    /// Parse a response that may be application/json or text/event-stream (SSE).
    async fn parse_response(&self, resp: reqwest::Response) -> Result<Option<JsonRpcResponse>> {
        let status = resp.status();
        if status == reqwest::StatusCode::ACCEPTED {
            return Ok(None);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            bail!("HTTP {status}: {body}");
        }

        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp.text().await.context("failed to read response body")?;

        if content_type.contains("text/event-stream") {
            // SSE: extract data lines
            let mut data = String::new();
            for line in body.lines() {
                if let Some(payload) = line.strip_prefix("data: ") {
                    data.push_str(payload);
                }
            }
            if data.is_empty() {
                bail!("empty SSE response");
            }
            let rpc: JsonRpcResponse =
                serde_json::from_str(&data).context("failed to parse SSE JSON-RPC response")?;
            Ok(Some(rpc))
        } else {
            let rpc: JsonRpcResponse =
                serde_json::from_str(&body).context("failed to parse JSON-RPC response")?;
            Ok(Some(rpc))
        }
    }

    /// Initialize the MCP session. Must be called before any tool calls.
    pub async fn initialize(&mut self) -> Result<()> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: Some(self.next_id()),
            method: "initialize",
            params: Some(serde_json::json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {
                    "name": "ekko",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
        };

        let resp = self
            .http
            .post(self.endpoint())
            .headers(self.headers())
            .json(&req)
            .send()
            .await
            .context("failed to connect to Graphiti")?;

        // Extract session ID from response headers
        if let Some(sid) = resp.headers().get(MCP_SESSION_HEADER) {
            self.session_id = Some(sid.to_str().unwrap_or_default().to_string());
        }

        let rpc = self
            .parse_response(resp)
            .await?
            .context("no response from initialize")?;

        if let Some(err) = rpc.error {
            bail!("initialize failed: {}", err.message);
        }

        // Send initialized notification
        let notif = JsonRpcRequest {
            jsonrpc: "2.0",
            id: None,
            method: "notifications/initialized",
            params: None,
        };

        self.http
            .post(self.endpoint())
            .headers(self.headers())
            .json(&notif)
            .send()
            .await
            .context("failed to send initialized notification")?;

        Ok(())
    }

    /// Call a tool and return the parsed text content.
    async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<String> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: Some(self.next_id()),
            method: "tools/call",
            params: Some(serde_json::json!({
                "name": name,
                "arguments": arguments
            })),
        };

        let resp = self
            .http
            .post(self.endpoint())
            .headers(self.headers())
            .json(&req)
            .send()
            .await
            .with_context(|| format!("failed to call tool '{name}'"))?;

        let rpc = self
            .parse_response(resp)
            .await?
            .with_context(|| format!("no response from tool '{name}'"))?;

        if let Some(err) = rpc.error {
            bail!("tool '{name}' error: {}", err.message);
        }

        let result_val = rpc.result.context("missing result in response")?;
        let tool_result: ToolResult =
            serde_json::from_value(result_val).context("failed to parse tool result")?;

        let text = tool_result
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }

    /// Call a tool and deserialize the text content as JSON.
    async fn call_tool_json<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<T> {
        let text = self.call_tool(name, arguments).await?;
        serde_json::from_str(&text)
            .with_context(|| format!("failed to parse response from '{name}': {text}"))
    }

    // --- Public API ---

    pub async fn add_memory(&self, req: AddMemoryRequest) -> Result<SuccessResponse> {
        self.call_tool_json("add_memory", serde_json::to_value(req)?).await
    }

    pub async fn search_nodes(&self, req: SearchNodesRequest) -> Result<NodeSearchResponse> {
        self.call_tool_json("search_nodes", serde_json::to_value(req)?).await
    }

    pub async fn search_facts(&self, req: SearchFactsRequest) -> Result<FactSearchResponse> {
        self.call_tool_json("search_memory_facts", serde_json::to_value(req)?).await
    }

    pub async fn get_episodes(&self, req: GetEpisodesRequest) -> Result<EpisodeSearchResponse> {
        self.call_tool_json("get_episodes", serde_json::to_value(req)?).await
    }

    pub async fn get_edge(&self, uuid: &str) -> Result<Fact> {
        self.call_tool_json("get_entity_edge", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn delete_edge(&self, uuid: &str) -> Result<SuccessResponse> {
        self.call_tool_json("delete_entity_edge", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn delete_episode(&self, uuid: &str) -> Result<SuccessResponse> {
        self.call_tool_json("delete_episode", serde_json::json!({ "uuid": uuid }))
            .await
    }

    pub async fn clear_graph(&self, req: ClearGraphRequest) -> Result<SuccessResponse> {
        self.call_tool_json("clear_graph", serde_json::to_value(req)?).await
    }

    pub async fn status(&self) -> Result<StatusResponse> {
        self.call_tool_json("get_status", serde_json::json!({})).await
    }

    /// Quick health check — can we reach Graphiti at all?
    pub async fn health(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
