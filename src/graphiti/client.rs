use anyhow::Result;

use super::direct::DirectClient;
use super::types::*;
use crate::config::Config;

/// Unified Graphiti client. Currently only wraps DirectClient;
/// will gain a DaemonClient variant in a future phase.
pub enum Client {
    Direct(DirectClient),
}

impl Client {
    /// Create a new client by spawning a Python shim directly.
    pub async fn new(config: &Config) -> Result<Self> {
        Ok(Self::Direct(DirectClient::new(config).await?))
    }

    pub async fn add_memory(&mut self, req: AddMemoryRequest) -> Result<SuccessResponse> {
        match self {
            Self::Direct(c) => c.add_memory(req).await,
        }
    }

    pub async fn search_nodes(&mut self, req: SearchNodesRequest) -> Result<NodeSearchResponse> {
        match self {
            Self::Direct(c) => c.search_nodes(req).await,
        }
    }

    pub async fn search_facts(&mut self, req: SearchFactsRequest) -> Result<FactSearchResponse> {
        match self {
            Self::Direct(c) => c.search_facts(req).await,
        }
    }

    pub async fn get_episodes(
        &mut self,
        req: GetEpisodesRequest,
    ) -> Result<EpisodeSearchResponse> {
        match self {
            Self::Direct(c) => c.get_episodes(req).await,
        }
    }

    pub async fn get_edge(&mut self, uuid: &str) -> Result<Fact> {
        match self {
            Self::Direct(c) => c.get_edge(uuid).await,
        }
    }

    pub async fn delete_edge(&mut self, uuid: &str) -> Result<SuccessResponse> {
        match self {
            Self::Direct(c) => c.delete_edge(uuid).await,
        }
    }

    pub async fn delete_episode(&mut self, uuid: &str) -> Result<SuccessResponse> {
        match self {
            Self::Direct(c) => c.delete_episode(uuid).await,
        }
    }

    pub async fn clear_graph(&mut self, req: ClearGraphRequest) -> Result<SuccessResponse> {
        match self {
            Self::Direct(c) => c.clear_graph(req).await,
        }
    }

    pub async fn status(&mut self) -> Result<StatusResponse> {
        match self {
            Self::Direct(c) => c.status().await,
        }
    }

    pub async fn queue_status(&mut self) -> Result<QueueStatusResponse> {
        match self {
            Self::Direct(c) => c.queue_status().await,
        }
    }

    pub async fn health(&mut self) -> Result<bool> {
        match self {
            Self::Direct(c) => c.health().await,
        }
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        match self {
            Self::Direct(c) => c.shutdown().await,
        }
    }
}
