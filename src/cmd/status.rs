use anyhow::Result;

use crate::config::Config;
use crate::graphiti;

pub async fn run() -> Result<()> {
    let config = Config::load()?;
    let client = graphiti::Client::new(&config.graphiti.url);

    let healthy = client.health().await.unwrap_or(false);
    if !healthy {
        println!("Graphiti is not reachable at {}", config.graphiti.url);
        println!("Run `ekko init` to set up, or `ekko doctor` to diagnose.");
        return Ok(());
    }

    let mut client = graphiti::Client::new(&config.graphiti.url);
    client.initialize().await?;

    match client.status().await {
        Ok(status) => {
            println!("Graphiti: {} — {}", status.status, status.message);
        }
        Err(e) => {
            println!("Graphiti status check failed: {e}");
        }
    }

    Ok(())
}
