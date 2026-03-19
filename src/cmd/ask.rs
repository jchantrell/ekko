use anyhow::Result;

use crate::graphiti::{SearchFactsRequest, SearchNodesRequest};
use crate::project;

use super::client;

pub async fn run(
    query: String,
    group: Option<String>,
    nodes: bool,
    max_facts: Option<u32>,
    max_nodes: Option<u32>,
) -> Result<()> {
    let client = client::connect().await?;

    let group_ids = group
        .map(project::sanitize_group_id)
        .or_else(|| project::detect_group_id(&std::env::current_dir().ok()?))
        .map(|g| vec![g]);

    let facts_resp = client
        .search_facts(SearchFactsRequest {
            query: query.clone(),
            group_ids: group_ids.clone(),
            max_facts,
            center_node_uuid: None,
        })
        .await?;

    if facts_resp.facts.is_empty() {
        println!("No facts found.");
    } else {
        println!("Facts:");
        for fact in &facts_resp.facts {
            println!("  {} {}", fact.uuid, fact);
        }
    }

    if nodes {
        let nodes_resp = client
            .search_nodes(SearchNodesRequest {
                query,
                group_ids,
                max_nodes,
                entity_types: None,
            })
            .await?;

        if nodes_resp.nodes.is_empty() {
            println!("\nNo entities found.");
        } else {
            println!("\nEntities:");
            for node in &nodes_resp.nodes {
                println!("  {} {}", node.uuid, node);
            }
        }
    }

    Ok(())
}
