use anyhow::{Context, Result};
use self_update::cargo_crate_version;

const REPO_OWNER: &str = "jchantrell";
const REPO_NAME: &str = "ekko";

pub async fn run(check_only: bool) -> Result<()> {
    // self_update uses blocking I/O, so run on the blocking thread pool
    tokio::task::spawn_blocking(move || {
        let current = cargo_crate_version!();

        if check_only {
            return check(current);
        }

        println!("Checking for updates...");

        let latest = latest_version()?;
        if latest.trim_start_matches('v') == current {
            println!("Already on latest (v{current}).");
            return Ok(());
        }

        let status = self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name("ekko")
            .show_download_progress(true)
            .current_version(current)
            .target_version_tag(&latest)
            .no_confirm(false)
            .build()
            .context("failed to configure updater")?
            .update()
            .context("failed to update")?;

        if status.updated() {
            println!("Updated to v{}.", status.version());
        } else {
            println!("Already on latest (v{current}).");
        }

        Ok(())
    })
    .await?
}

fn latest_version() -> Result<String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .context("failed to configure release list")?
        .fetch()
        .context("failed to fetch releases")?;

    releases
        .first()
        .map(|r| format!("v{}", r.version))
        .ok_or_else(|| anyhow::anyhow!("no releases found"))
}

fn check(current: &str) -> Result<()> {
    let latest = latest_version()?;
    let latest_ver = latest.trim_start_matches('v');
    if latest_ver != current {
        println!("Update available: v{current} -> v{latest_ver}");
        println!("Run `ekko update` to install.");
    } else {
        println!("Already on latest (v{current}).");
    }
    Ok(())
}
