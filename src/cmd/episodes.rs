use anyhow::Result;

use crate::graphiti::GetEpisodesRequest;
use crate::project;

use super::client;

pub async fn run(group: Option<String>, max: Option<u32>) -> Result<()> {
    let mut client = client::connect().await?;

    let group_ids = group
        .map(project::sanitize_group_id)
        .or_else(|| project::detect_group_id(&std::env::current_dir().ok()?))
        .map(|g| vec![g]);

    let resp = client
        .get_episodes(GetEpisodesRequest {
            group_ids,
            max_episodes: max,
        })
        .await?;

    if resp.episodes.is_empty() {
        println!("No episodes found.");
    } else {
        for ep in &resp.episodes {
            println!("{} {}", ep.uuid, ep);
        }
    }

    Ok(())
}
