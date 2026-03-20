use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};

use crate::mcp::EkkoServer;
use crate::session;

pub async fn run() -> Result<()> {
    let server = EkkoServer::from_config().await?;

    tokio::spawn(async {
        session::background_sync().await;
    });

    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
