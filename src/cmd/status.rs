use anyhow::Result;

use super::client;

pub async fn run() -> Result<()> {
    let mut client = match client::connect().await {
        Ok(c) => c,
        Err(e) => {
            println!("Could not connect: {e}");
            println!("Run `ekko init` to set up, or `ekko doctor` to diagnose.");
            return Ok(());
        }
    };

    match client.status().await {
        Ok(status) => {
            println!("Graphiti: {} — {}", status.status, status.message);
        }
        Err(e) => {
            println!("Status check failed: {e}");
        }
    }

    Ok(())
}
