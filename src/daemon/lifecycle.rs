use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::config::Config;

/// Write the current process PID to the PID file.
pub fn write_pid() -> Result<()> {
    let path = Config::pid_path()?;
    let pid = std::process::id();
    std::fs::write(&path, pid.to_string())
        .with_context(|| format!("failed to write PID file {}", path.display()))?;
    Ok(())
}

/// Remove the PID file.
pub fn remove_pid() {
    if let Ok(path) = Config::pid_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// Remove a stale socket file.
pub fn remove_socket() {
    if let Ok(path) = Config::socket_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// Read the PID from the PID file, if it exists and the process is alive.
pub fn running_pid() -> Option<u32> {
    let path = Config::pid_path().ok()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let pid: u32 = contents.trim().parse().ok()?;

    if is_alive(pid) {
        Some(pid)
    } else {
        // Stale PID file — clean up
        let _ = std::fs::remove_file(&path);
        remove_socket();
        None
    }
}

/// Check if a process is alive.
fn is_alive(pid: u32) -> bool {
    // kill(pid, 0) checks existence without sending a signal
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Start the daemon as a detached background process.
#[allow(dead_code)]
pub fn start_background() -> Result<()> {
    let exe = std::env::current_exe().context("failed to get current executable path")?;

    let child = std::process::Command::new(exe)
        .args(["daemon", "start"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn daemon process")?;

    // Detach — we don't wait for it
    drop(child);
    Ok(())
}

/// Ensure the daemon is running. Returns Ok if it's already running or was started.
#[allow(dead_code)]
pub fn ensure_running() -> Result<()> {
    if running_pid().is_some() {
        return Ok(());
    }

    // Check socket is connectable (PID might be stale)
    let socket_path = Config::socket_path()?;
    if socket_path.exists() && running_pid().is_none() {
        // Stale socket, clean up
        remove_socket();
    }

    start_background()?;
    Ok(())
}

/// Wait for the daemon socket to become available.
#[allow(dead_code)]
pub async fn wait_for_socket(timeout: std::time::Duration) -> Result<()> {
    let socket_path = Config::socket_path()?;
    let start = std::time::Instant::now();
    let mut delay = std::time::Duration::from_millis(50);

    while start.elapsed() < timeout {
        if socket_connectable(&socket_path).await {
            return Ok(());
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(std::time::Duration::from_millis(500));
    }

    bail!("daemon did not start within {}s", timeout.as_secs());
}

#[allow(dead_code)]
async fn socket_connectable(path: &Path) -> bool {
    tokio::net::UnixStream::connect(path).await.is_ok()
}
