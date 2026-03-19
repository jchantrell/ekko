use anyhow::Result;

use super::client;

pub async fn run(uuid: String) -> Result<()> {
    let client = client::connect().await?;
    let resp = client.delete_edge(&uuid).await?;
    println!("{}", resp.message);
    Ok(())
}
