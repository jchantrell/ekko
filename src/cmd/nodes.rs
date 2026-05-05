use anyhow::Result;

use crate::graphiti::SearchNodesRequest;

use super::client;

pub async fn run(
    query: Option<String>,
    entity_type: Option<String>,
    max: Option<u32>,
) -> Result<()> {
    let mut client = client::connect().await?;

    let query = query.unwrap_or_default();
    let entity_types = entity_type.map(|t| vec![t]);

    let resp = client
        .search_nodes(SearchNodesRequest {
            query,
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
