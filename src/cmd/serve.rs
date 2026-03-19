use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};

use crate::mcp::EkkoServer;

pub async fn run() -> Result<()> {
    let server = EkkoServer::from_config().await?;
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
