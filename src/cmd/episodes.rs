use anyhow::Result;

use crate::graphiti::GetEpisodesRequest;

use super::client;

pub async fn run(max: Option<u32>) -> Result<()> {
    let mut client = client::connect().await?;

    let resp = client
        .get_episodes(GetEpisodesRequest {
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
