use anyhow::Result;

use crate::daemon;
use crate::graphiti::daemon_client::DaemonClient;

pub async fn start() -> Result<()> {
    daemon::server::run().await
}

pub async fn stop() -> Result<()> {
    let mut client = DaemonClient::connect().await?;
    let _ = client.shutdown().await;
    println!("Daemon stopped.");
    Ok(())
}

pub async fn status() -> Result<()> {
    match daemon::lifecycle::running_pid() {
        Some(pid) => println!("Daemon running (PID {pid})"),
        None => println!("Daemon not running"),
    }
    Ok(())
}
