use anyhow::Result;

use crate::config::Config;
use crate::daemon::lifecycle;
use crate::graphiti;

/// Connect to the Graphiti backend, preferring the daemon.
///
/// 1. Try connecting to the daemon socket
/// 2. If not running, clean up stale files, start daemon, retry
/// 3. If daemon won't start, fall back to a direct shim connection
pub async fn connect() -> Result<graphiti::Client> {
    if let Ok(client) = graphiti::Client::connect_daemon().await {
        return Ok(client);
    }

    lifecycle::cleanup_stale();

    if lifecycle::start_background().is_ok()
        && lifecycle::wait_for_socket(std::time::Duration::from_secs(10))
            .await
            .is_ok()
        && let Ok(client) = graphiti::Client::connect_daemon().await
    {
        return Ok(client);
    }

    eprintln!("warning: daemon unavailable, using direct connection");
    let config = Config::load()?;
    graphiti::Client::new(&config).await
}
