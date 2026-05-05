use anyhow::Result;
use chrono::NaiveDate;

use crate::project;
use crate::session;
use crate::session::normalize::{Agent, SyncOptions};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    full: bool,
    agent: Option<String>,
    origin: Option<String>,
    all: bool,
    since: Option<String>,
    no_llm: bool,
    dry_run: bool,
    limit: Option<usize>,
) -> Result<()> {
    let agent_filter = match agent.as_deref() {
        None => None,
        Some(s) => match Agent::from_str(s) {
            Some(a) => Some(a),
            None => anyhow::bail!("unknown agent {s:?}. Valid: claude-code, opencode"),
        },
    };

    let since = since
        .map(|s| {
            NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc())
        })
        .transpose()?;

    let origin_filter = if all {
        None
    } else {
        origin.or_else(|| project::detect_origin(&std::env::current_dir().ok()?))
    };

    let opts = SyncOptions {
        full,
        agent_filter,
        origin_filter,
        since,
        no_llm,
        dry_run,
        limit,
    };

    session::sync(opts).await?;

    Ok(())
}
