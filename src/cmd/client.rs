use anyhow::{bail, Result};

use crate::config::Config;
use crate::graphiti;

/// Create an initialized Graphiti client ready for tool calls.
pub async fn connect() -> Result<graphiti::Client> {
    let config = Config::load()?;
    let mut client = graphiti::Client::new(&config.graphiti.url);

    let healthy = client.health().await.unwrap_or(false);
    if !healthy {
        bail!(
            "Graphiti is not reachable at {}. Run `ekko doctor` to diagnose.",
            config.graphiti.url
        );
    }

    client.initialize().await?;
    Ok(client)
}
