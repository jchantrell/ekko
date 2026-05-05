use serde::{Deserialize, Serialize};

/// Request to store a memory in the knowledge graph.
#[derive(Debug, Serialize)]
pub struct AddMemoryRequest {
    pub name: String,
    pub episode_body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub source: String,
    #[serde(default)]
    pub source_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    /// Process synchronously (blocks until done). Default false (async queue).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sync: bool,
    /// ISO-8601 timestamp for when this memory is from. Defaults to now if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchNodesRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_nodes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct SearchFactsRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_facts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub center_node_uuid: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetEpisodesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_episodes: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ClearGraphRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ListGroupsRequest {
    #[serde(default)]
    pub include_stats: bool,
}

// --- Response types ---

#[derive(Debug, Deserialize)]
pub struct Node {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub labels: Vec<String>,
    pub created_at: Option<String>,
    pub summary: Option<String>,
    pub group_id: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct Fact {
    pub uuid: String,
    pub name: String,
    pub fact: String,
    pub valid_at: Option<String>,
    pub invalid_at: Option<String>,
    pub created_at: Option<String>,
    pub expired_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Episode {
    pub uuid: String,
    pub name: String,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub source: Option<String>,
    pub source_description: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub message: String,
}

/// Graphiti MCP server wraps responses in tool result envelopes.
/// We need to handle both success and error cases.
#[derive(Debug, Deserialize)]
pub struct SuccessResponse {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct NodeSearchResponse {
    pub message: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
pub struct FactSearchResponse {
    pub message: String,
    #[serde(default)]
    pub facts: Vec<Fact>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeSearchResponse {
    pub message: String,
    #[serde(default)]
    pub episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
pub struct QueueStatusResponse {
    #[serde(default)]
    pub groups: Vec<QueueGroup>,
}

#[derive(Debug, Deserialize)]
pub struct QueueGroup {
    pub group_id: String,
    pub processing: Option<String>,
    pub pending: u32,
}

#[derive(Debug, Deserialize)]
pub struct ListGroupsResponse {
    #[serde(default)]
    pub groups: Vec<GroupInfo>,
}

#[derive(Debug, Deserialize)]
pub struct GroupInfo {
    pub group_id: String,
    pub entity_count: Option<u32>,
    pub episode_count: Option<u32>,
    pub last_activity: Option<String>,
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.labels.is_empty() {
            write!(f, " [{}]", self.labels.join(", "))?;
        }
        if let Some(ref summary) = self.summary {
            let first_line = summary.lines().next().unwrap_or(summary);
            write!(f, " — {first_line}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Fact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fact)?;
        if let Some(ref valid) = self.valid_at {
            write!(f, " (valid: {valid}")?;
            if let Some(ref invalid) = self.invalid_at {
                write!(f, ", invalid: {invalid}")?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Episode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(ref src) = self.source_description {
            write!(f, " — {src}")?;
        }
        if let Some(ref created) = self.created_at {
            write!(f, " ({created})")?;
        }
        Ok(())
    }
}
