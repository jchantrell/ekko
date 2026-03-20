use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::graphiti::{self, *};
use crate::project;

/// MCP server exposing ekko's memory tools over STDIO.
#[derive(Clone)]
pub struct EkkoServer {
    client: Arc<Mutex<graphiti::Client>>,
    group_id: Option<String>,
    tool_router: ToolRouter<Self>,
}

// -- Tool parameter types --

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberParams {
    /// The memory content to store (freeform text).
    pub content: String,
    /// Optional project scope. Auto-detected from cwd if omitted.
    pub group_id: Option<String>,
    /// Optional name for this memory episode.
    pub name: Option<String>,
    /// Optional source description (e.g. "claude-code session").
    pub source_description: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecallParams {
    /// Search query — what do you want to remember?
    pub query: String,
    /// Optional project scope. Auto-detected from cwd if omitted.
    pub group_id: Option<String>,
    /// Maximum number of facts to return (default: 10).
    pub max_facts: Option<u32>,
    /// Maximum number of entities to return (default: 5).
    pub max_nodes: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ForgetParams {
    /// UUID of the fact (edge) to delete.
    pub uuid: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EntitiesParams {
    /// Search query for entities.
    pub query: String,
    /// Optional project scope. Auto-detected from cwd if omitted.
    pub group_id: Option<String>,
    /// Maximum number of entities to return (default: 10).
    pub max_nodes: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EpisodesParams {
    /// Optional project scope. Auto-detected from cwd if omitted.
    pub group_id: Option<String>,
    /// Maximum number of episodes to return (default: 10).
    pub max_episodes: Option<u32>,
}

#[tool_router]
impl EkkoServer {
    pub fn new(client: graphiti::Client, group_id: Option<String>) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            group_id,
            tool_router: Self::tool_router(),
        }
    }

    fn resolve_group_id(&self, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| self.group_id.clone())
    }

    fn group_ids(&self, explicit: Option<String>) -> Option<Vec<String>> {
        self.resolve_group_id(explicit).map(|g| vec![g])
    }

    #[tool(
        name = "remember",
        description = "Store a memory in ekko's knowledge graph. Graphiti will extract entities, facts, and relationships automatically. The memory is processed asynchronously (5-15s) but this call returns immediately."
    )]
    async fn remember(
        &self,
        Parameters(params): Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        let group_id = self.resolve_group_id(params.group_id);
        let name = params.name.unwrap_or_else(|| {
            format!(
                "agent-memory-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            )
        });

        let mut client = self.client.lock().await;
        let result = client
            .add_memory(AddMemoryRequest {
                name,
                episode_body: params.content,
                group_id,
                source: "text".into(),
                source_description: params.source_description.unwrap_or_default(),
                uuid: None,
                sync: false,
                reference_time: None,
            })
            .await
            .map_err(|e| McpError::internal_error(format!("add_memory failed: {e}"), None))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Memory stored. {}",
            result.message
        ))]))
    }

    #[tool(
        name = "recall",
        description = "Search ekko's knowledge graph for relevant memories. Returns both facts (relationships with temporal metadata) and entities. Use this to recall preferences, decisions, patterns, or anything previously stored."
    )]
    async fn recall(
        &self,
        Parameters(params): Parameters<RecallParams>,
    ) -> Result<CallToolResult, McpError> {
        let group_ids = self.group_ids(params.group_id);

        let mut client = self.client.lock().await;

        let facts_req = SearchFactsRequest {
            query: params.query.clone(),
            group_ids: group_ids.clone(),
            max_facts: Some(params.max_facts.unwrap_or(10)),
            center_node_uuid: None,
        };
        let nodes_req = SearchNodesRequest {
            query: params.query,
            group_ids,
            max_nodes: Some(params.max_nodes.unwrap_or(5)),
            entity_types: None,
        };

        let facts_result = client.search_facts(facts_req).await;
        let nodes_result = client.search_nodes(nodes_req).await;

        let mut output = String::new();

        match facts_result {
            Ok(resp) if !resp.facts.is_empty() => {
                output.push_str("## Facts\n\n");
                for fact in &resp.facts {
                    output.push_str(&format!("- {fact}\n"));
                    output.push_str(&format!("  uuid: {}\n", fact.uuid));
                }
            }
            Ok(_) => output.push_str("No matching facts found.\n"),
            Err(e) => output.push_str(&format!("Fact search error: {e}\n")),
        }

        match nodes_result {
            Ok(resp) if !resp.nodes.is_empty() => {
                output.push_str("\n## Entities\n\n");
                for node in &resp.nodes {
                    output.push_str(&format!("- {node}\n"));
                }
            }
            Ok(_) => {}
            Err(e) => output.push_str(&format!("Node search error: {e}\n")),
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "forget",
        description = "Delete a specific fact from the knowledge graph by its UUID. Use recall first to find the UUID of the fact you want to remove."
    )]
    async fn forget(
        &self,
        Parameters(params): Parameters<ForgetParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;
        let result = client.delete_edge(&params.uuid).await.map_err(|e| {
            McpError::internal_error(format!("delete_entity_edge failed: {e}"), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Fact deleted. {}",
            result.message
        ))]))
    }

    #[tool(
        name = "entities",
        description = "Search for entities (people, technologies, concepts, projects) in the knowledge graph. Returns entity names, labels, and summaries."
    )]
    async fn entities(
        &self,
        Parameters(params): Parameters<EntitiesParams>,
    ) -> Result<CallToolResult, McpError> {
        let group_ids = self.group_ids(params.group_id);

        let mut client = self.client.lock().await;
        let resp = client
            .search_nodes(SearchNodesRequest {
                query: params.query,
                group_ids,
                max_nodes: Some(params.max_nodes.unwrap_or(10)),
                entity_types: None,
            })
            .await
            .map_err(|e| McpError::internal_error(format!("search_nodes failed: {e}"), None))?;

        if resp.nodes.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No matching entities found.",
            )]));
        }

        let mut output = String::new();
        for node in &resp.nodes {
            output.push_str(&format!("- {node}\n"));
            output.push_str(&format!(
                "  uuid: {} | group: {}\n",
                node.uuid, node.group_id
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "episodes",
        description = "List recent episodes (memory ingestion events) from the knowledge graph. Each episode represents a stored memory or indexed session."
    )]
    async fn episodes(
        &self,
        Parameters(params): Parameters<EpisodesParams>,
    ) -> Result<CallToolResult, McpError> {
        let group_ids = self.group_ids(params.group_id);

        let mut client = self.client.lock().await;
        let resp = client
            .get_episodes(GetEpisodesRequest {
                group_ids,
                max_episodes: Some(params.max_episodes.unwrap_or(10)),
            })
            .await
            .map_err(|e| McpError::internal_error(format!("get_episodes failed: {e}"), None))?;

        if resp.episodes.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No episodes found.",
            )]));
        }

        let mut output = String::new();
        for ep in &resp.episodes {
            output.push_str(&format!("- {ep}\n"));
            output.push_str(&format!("  uuid: {}\n", ep.uuid));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "queue",
        description = "Show the memory processing queue. Returns which episodes are currently being processed and how many are pending per group."
    )]
    async fn queue(&self) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;
        let resp = client.queue_status().await.map_err(|e| {
            McpError::internal_error(format!("queue_status failed: {e}"), None)
        })?;

        if resp.groups.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "Queue is empty.",
            )]));
        }

        let mut output = String::new();
        for group in &resp.groups {
            let status = match &group.processing {
                Some(name) => format!("processing: {name}"),
                None => "idle".into(),
            };
            output.push_str(&format!(
                "{} — {status}, {} pending\n",
                group.group_id, group.pending
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "status",
        description = "Check ekko's health — is Graphiti reachable? Is the MCP session active? Returns system status for diagnostics."
    )]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;

        let health = client.health().await.unwrap_or(false);
        if !health {
            return Ok(CallToolResult::success(vec![Content::text(
                "Graphiti is unreachable. Run `ekko doctor` to diagnose.",
            )]));
        }

        match client.status().await {
            Ok(s) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Graphiti: {} — {}\nProject scope: {}",
                s.status,
                s.message,
                self.group_id.as_deref().unwrap_or("(none)"),
            ))])),
            Err(e) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Graphiti reachable but status check failed: {e}",
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for EkkoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("ekko", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "ekko provides persistent memory for AI agents via a temporal knowledge graph. \
                 Use remember to store memories, recall to search them, \
                 forget to remove facts, entities to explore the graph, \
                 episodes to list ingestion history, and status to check health.",
            )
    }
}

impl EkkoServer {
    /// Build a server from config, auto-detecting project from cwd.
    pub async fn from_config() -> anyhow::Result<Self> {
        let config = Config::load()?;
        let client = graphiti::Client::new(&config)
            .await
            .map_err(|e| anyhow::anyhow!("failed to initialize graphiti: {e}"))?;

        let group_id = std::env::current_dir()
            .ok()
            .and_then(|cwd| project::detect_group_id(&cwd));

        Ok(Self::new(client, group_id))
    }
}
