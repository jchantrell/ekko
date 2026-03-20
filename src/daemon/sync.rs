use std::time::Duration;

use tokio::sync::Mutex;

use crate::graphiti::direct::DirectClient;
use crate::session;
use crate::session::normalize::SyncOptions;

const INITIAL_DELAY: Duration = Duration::from_secs(5);
const SYNC_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes
const SYNC_LIMIT: usize = 50;

/// Run periodic background sync using the daemon's direct client.
pub async fn run(client: std::sync::Arc<Mutex<DirectClient>>) {
    tokio::time::sleep(INITIAL_DELAY).await;

    loop {
        run_once(&client).await;
        tokio::time::sleep(SYNC_INTERVAL).await;
    }
}

async fn run_once(_client: &std::sync::Arc<Mutex<DirectClient>>) {
    let opts = SyncOptions {
        full: false,
        agent_filter: None,
        group_filter: None,
        since: None,
        no_llm: false,
        dry_run: false,
        limit: Some(SYNC_LIMIT),
    };

    match session::sync(opts).await {
        Ok(report) if report.indexed > 0 => {
            eprintln!("[daemon sync] {report}");
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("[daemon sync] failed: {e}");
        }
    }
}
