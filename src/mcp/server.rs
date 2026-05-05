use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::graphiti::{self, *};
use crate::groups::GroupsDb;
use crate::project;

/// MCP server exposing ekko's memory tools over STDIO.
#[derive(Clone)]
pub struct EkkoServer {
    client: Arc<Mutex<graphiti::Client>>,
    origin: Option<String>,
    tool_router: ToolRouter<Self>,
}

// -- Tool parameter types --

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberParams {
    /// The memory content to store (freeform text).
    pub content: String,
    /// Optional origin label (project name). Auto-detected from cwd if omitted.
    pub origin: Option<String>,
    /// Optional name for this memory episode.
    pub name: Option<String>,
    /// Optional source description (e.g. "claude-code session").
    pub source_description: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RecallParams {
    /// Search query — what do you want to remember?
    pub query: String,
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
    /// Maximum number of entities to return (default: 10).
    pub max_nodes: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EpisodesParams {
    /// Maximum number of episodes to return (default: 10).
    pub max_episodes: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct OriginsParams {
    /// Optional substring filter — only return origins matching this.
    pub filter: Option<String>,
    /// If true, include entity/episode counts per origin (slower).
    pub include_stats: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SetOriginParams {
    /// The origin to set metadata for.
    pub origin: String,
    /// Human-friendly display name for the origin.
    pub name: Option<String>,
    /// Short description of what this project/origin contains.
    pub description: Option<String>,
}

#[tool_router]
impl EkkoServer {
    pub fn new(client: graphiti::Client, origin: Option<String>) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            origin,
            tool_router: Self::tool_router(),
        }
    }

    fn resolve_origin(&self, explicit: Option<String>) -> Option<String> {
        explicit.or_else(|| self.origin.clone())
    }

    #[tool(
        name = "remember",
        description = "Store a memory in ekko's knowledge graph. Graphiti will extract entities, facts, and relationships automatically. The memory is processed asynchronously (5-15s) but this call returns immediately."
    )]
    async fn remember(
        &self,
        Parameters(params): Parameters<RememberParams>,
    ) -> Result<CallToolResult, McpError> {
        let origin = self.resolve_origin(params.origin);
        let name = params.name.unwrap_or_else(|| {
            format!(
                "agent-memory-{}",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            )
        });

        if let Some(ref o) = origin
            && let Ok(db) = GroupsDb::open()
        {
            let _ = db.ensure_exists(o);
        }

        let mut client = self.client.lock().await;
        let result = client
            .add_memory(AddMemoryRequest {
                name,
                episode_body: params.content,
                origin,
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
        description = "Search ekko's knowledge graph for relevant memories. Returns both facts (relationships with temporal metadata) and entities. Searches across all projects. Use this to recall preferences, decisions, patterns, or anything previously stored."
    )]
    async fn recall(
        &self,
        Parameters(params): Parameters<RecallParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;

        let facts_req = SearchFactsRequest {
            query: params.query.clone(),
            max_facts: Some(params.max_facts.unwrap_or(10)),
            center_node_uuid: None,
        };
        let nodes_req = SearchNodesRequest {
            query: params.query,
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
        description = "Search for entities (people, technologies, concepts, projects) in the knowledge graph. Returns entity names, labels, and summaries. Searches across all projects."
    )]
    async fn entities(
        &self,
        Parameters(params): Parameters<EntitiesParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;
        let resp = client
            .search_nodes(SearchNodesRequest {
                query: params.query,
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
                "  uuid: {} | origin: {}\n",
                node.uuid, node.origin
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "episodes",
        description = "List recent episodes (memory ingestion events) from the knowledge graph. Each episode represents a stored memory or indexed session. Lists across all projects."
    )]
    async fn episodes(
        &self,
        Parameters(params): Parameters<EpisodesParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut client = self.client.lock().await;
        let resp = client
            .get_episodes(GetEpisodesRequest {
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
        description = "Show the memory processing queue. Returns which episodes are currently being processed and how many are pending per origin."
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
                group.origin, group.pending
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "origins",
        description = "List all known origins (project sources) in the knowledge graph. Each origin represents where memories came from. Returns origin IDs with optional name, description, and stats."
    )]
    async fn origins(
        &self,
        Parameters(params): Parameters<OriginsParams>,
    ) -> Result<CallToolResult, McpError> {
        let include_stats = params.include_stats.unwrap_or(false);

        let mut client = self.client.lock().await;
        let graph_origins = client
            .list_origins(ListOriginsRequest { include_stats })
            .await
            .map_err(|e| McpError::internal_error(format!("list_origins failed: {e}"), None))?;

        let local_groups = GroupsDb::open()
            .and_then(|db| db.list())
            .unwrap_or_default();

        let local_map: std::collections::HashMap<&str, &crate::groups::GroupMeta> = local_groups
            .iter()
            .map(|g| (g.group_id.as_str(), g))
            .collect();

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut output = String::from("## Origins\n\n");
        let mut count = 0;

        for oi in &graph_origins.groups {
            if let Some(ref filter) = params.filter
                && !oi.origin.contains(filter.as_str())
            {
                continue;
            }
            seen.insert(oi.origin.clone());
            count += 1;

            let meta = local_map.get(oi.origin.as_str());
            let display_name = meta.and_then(|m| m.name.as_deref());
            let desc = meta.and_then(|m| m.description.as_deref());

            output.push_str(&format!("- **{}**", oi.origin));
            if let Some(name) = display_name
                && name != oi.origin
            {
                output.push_str(&format!(" ({name})"));
            }
            if let Some(d) = desc {
                output.push_str(&format!(" — {d}"));
            }
            if include_stats {
                output.push_str(&format!(
                    "\n  entities: {}, episodes: {}",
                    oi.entity_count.unwrap_or(0),
                    oi.episode_count.unwrap_or(0),
                ));
                if let Some(ref last) = oi.last_activity {
                    output.push_str(&format!(", last_activity: {last}"));
                }
            }
            output.push('\n');
        }

        for lg in &local_groups {
            if seen.contains(&lg.group_id) {
                continue;
            }
            if let Some(ref filter) = params.filter
                && !lg.group_id.contains(filter.as_str())
            {
                continue;
            }
            count += 1;
            output.push_str(&format!("- **{}**", lg.group_id));
            if let Some(ref name) = lg.name
                && name != &lg.group_id
            {
                output.push_str(&format!(" ({name})"));
            }
            if let Some(ref d) = lg.description {
                output.push_str(&format!(" — {d}"));
            }
            output.push_str(" *(no graph data)*\n");
        }

        if count == 0 {
            output.push_str("No origins found.\n");
        }

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tool(
        name = "set_origin",
        description = "Set the display name and/or description for a project origin. This metadata is shown when listing origins."
    )]
    async fn set_origin(
        &self,
        Parameters(params): Parameters<SetOriginParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = GroupsDb::open()
            .map_err(|e| McpError::internal_error(format!("failed to open groups db: {e}"), None))?;

        db.upsert(
            &params.origin,
            params.name.as_deref(),
            params.description.as_deref(),
        )
        .map_err(|e| McpError::internal_error(format!("failed to upsert origin: {e}"), None))?;

        let mut msg = format!("Origin '{}' updated.", params.origin);
        if let Some(ref name) = params.name {
            msg.push_str(&format!(" name={name}"));
        }
        if let Some(ref desc) = params.description {
            msg.push_str(&format!(" description=\"{desc}\""));
        }

        Ok(CallToolResult::success(vec![Content::text(msg)]))
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
                "Graphiti: {} — {}\nOrigin: {}",
                s.status,
                s.message,
                self.origin.as_deref().unwrap_or("(none)"),
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
                 episodes to list ingestion history, origins to list known project sources, \
                 set_origin to add name/description to an origin, and status to check health.",
            )
    }
}

impl EkkoServer {
    /// Build a server from config, auto-detecting origin from cwd.
    pub async fn from_config() -> anyhow::Result<Self> {
        let client = crate::cmd::client::connect()
            .await
            .map_err(|e| anyhow::anyhow!("failed to connect to graphiti: {e}"))?;

        let origin = std::env::current_dir()
            .ok()
            .and_then(|cwd| project::detect_origin(&cwd));

        Ok(Self::new(client, origin))
    }
}
