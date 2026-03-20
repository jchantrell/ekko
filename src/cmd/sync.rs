use anyhow::Result;
use chrono::NaiveDate;

use crate::session;
use crate::session::normalize::{Agent, SyncOptions};

pub async fn run(
    full: bool,
    agent: Option<String>,
    group: Option<String>,
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

    let opts = SyncOptions {
        full,
        agent_filter,
        group_filter: group,
        since,
        no_llm,
        dry_run,
        limit,
    };

    session::sync(opts).await?;

    Ok(())
}
