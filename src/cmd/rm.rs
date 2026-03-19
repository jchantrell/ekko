use anyhow::Result;

use super::client;

pub async fn run_fact(uuid: String) -> Result<()> {
    let client = client::connect().await?;
    let resp = client.delete_edge(&uuid).await?;
    println!("{}", resp.message);
    Ok(())
}

pub async fn run_episode(uuid: String) -> Result<()> {
    let client = client::connect().await?;
    let resp = client.delete_episode(&uuid).await?;
    println!("{}", resp.message);
    Ok(())
}
