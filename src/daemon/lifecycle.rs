use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Write;

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
        let _ = std::fs::remove_file(&path);
        remove_socket();
        None
    }
}

fn is_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

/// Start the daemon as a detached background process.
/// Uses a lock file to prevent concurrent starts.
pub fn start_background() -> Result<()> {
    if running_pid().is_some() {
        return Ok(());
    }

    let lock_path = Config::data_dir()?.join("daemon.lock");
    let lock_file = File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)
        .context("failed to open daemon lock file")?;

    // Try exclusive lock (non-blocking)
    use std::os::unix::io::AsRawFd;
    let ret = unsafe { libc::flock(lock_file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if ret != 0 {
        // Another process is already starting the daemon
        return Ok(());
    }

    // Double-check after acquiring lock
    if running_pid().is_some() {
        return Ok(());
    }

    let exe = std::env::current_exe().context("failed to get current executable path")?;

    // Write a marker so we know a start is in progress
    let mut lock = &lock_file;
    let _ = write!(lock, "{}", std::process::id());

    let child = std::process::Command::new(exe)
        .args(["daemon", "start"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn daemon process")?;

    drop(child);
    // Lock file released on drop
    Ok(())
}

/// Wait for the daemon socket to become available.
pub async fn wait_for_socket(timeout: std::time::Duration) -> Result<()> {
    let socket_path = Config::socket_path()?;
    let start = std::time::Instant::now();
    let mut delay = std::time::Duration::from_millis(50);

    while start.elapsed() < timeout {
        if tokio::net::UnixStream::connect(&socket_path).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(std::time::Duration::from_millis(500));
    }

    bail!("daemon did not start within {}s", timeout.as_secs());
}
