use anyhow::Result;

use crate::config::Config;
use crate::graphiti;

/// Create an initialized Graphiti client ready for tool calls.
pub async fn connect() -> Result<graphiti::Client> {
    let config = Config::load()?;
    graphiti::Client::new(&config).await
}
