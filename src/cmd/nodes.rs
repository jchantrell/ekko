use anyhow::Result;

use crate::graphiti::SearchNodesRequest;
use crate::project;

use super::client;

pub async fn run(
    query: Option<String>,
    group: Option<String>,
    entity_type: Option<String>,
    max: Option<u32>,
) -> Result<()> {
    let client = client::connect().await?;

    let group_ids = group
        .map(project::sanitize_group_id)
        .or_else(|| project::detect_group_id(&std::env::current_dir().ok()?))
        .map(|g| vec![g]);

    let query = query.unwrap_or_default();
    let entity_types = entity_type.map(|t| vec![t]);

    let resp = client
        .search_nodes(SearchNodesRequest {
            query,
            group_ids,
            max_nodes: max,
            entity_types,
        })
        .await?;

    if resp.nodes.is_empty() {
        println!("No entities found.");
    } else {
        for node in &resp.nodes {
            println!("{} {}", node.uuid, node);
        }
    }

    Ok(())
}
