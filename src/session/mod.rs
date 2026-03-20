pub mod db;
pub mod normalize;
pub mod parsers;
pub mod summarize;

use anyhow::Result;
use chrono::{Duration, Utc};

use crate::config::Config;
use crate::graphiti::{self, AddMemoryRequest};
use normalize::*;
use parsers::SessionParser;

const DEFAULT_WINDOW_DAYS: i64 = 30;

/// Collected info needed to ingest a session (after parsing, before async work).
struct IndexJob {
    session_ref: SessionRef,
    parsed: ParsedSession,
}

pub async fn sync(opts: SyncOptions) -> Result<SyncReport> {
    let db = db::IndexDb::open()?;
    let parsers = build_parsers(&opts);

    let mut all_sessions = discover(&parsers)?;

    let cutoff = if opts.full {
        None
    } else {
        Some(
            opts.since
                .unwrap_or_else(|| Utc::now() - Duration::days(DEFAULT_WINDOW_DAYS)),
        )
    };

    if let Some(cutoff) = cutoff {
        all_sessions.retain(|s| s.modified_at.is_none_or(|t| t >= cutoff));
    }

    if let Some(ref group) = opts.group_filter {
        let normalized = group.to_ascii_lowercase();
        all_sessions.retain(|s| {
            s.project_hint
                .as_deref()
                .is_some_and(|h| h.to_ascii_lowercase() == normalized)
        });
    }

    let discovered = all_sessions.len();
    let mut skipped = 0usize;
    let mut to_index = Vec::new();

    for session in &all_sessions {
        if opts.full || db.needs_indexing(session)? {
            to_index.push(session);
        } else {
            skipped += 1;
        }
    }

    if let Some(limit) = opts.limit {
        to_index.truncate(limit);
    }

    let new = to_index.len();

    if opts.dry_run {
        eprintln!("Would index {} sessions:", to_index.len());
        for s in &to_index {
            eprintln!(
                "  {}/{} — {}",
                s.agent,
                s.session_id,
                s.project_hint.as_deref().unwrap_or("?")
            );
        }
        return Ok(SyncReport {
            discovered,
            new,
            modified: 0,
            skipped,
            indexed: 0,
            errors: 0,
        });
    }

    if to_index.is_empty() {
        eprintln!("Nothing to index ({discovered} sessions already up-to-date).");
        return Ok(SyncReport {
            discovered,
            new: 0,
            modified: 0,
            skipped,
            indexed: 0,
            errors: 0,
        });
    }

    eprintln!("Indexing {} sessions...", to_index.len());

    // Phase 1: Parse all sessions synchronously (file I/O, no async needed).
    let mut jobs: Vec<IndexJob> = Vec::new();
    let mut parse_errors = 0usize;

    for session_ref in to_index {
        let parsed = match session_ref.agent {
            Agent::ClaudeCode => parsers::claude_code::ClaudeCodeParser.parse(session_ref),
            Agent::OpenCode => parsers::opencode::OpenCodeParser.parse(session_ref),
        };

        match parsed {
            Ok(parsed) => jobs.push(IndexJob {
                session_ref: session_ref.clone(),
                parsed,
            }),
            Err(e) => {
                eprintln!(
                    "  {}/{} — parse error: {e}",
                    session_ref.agent,
                    truncate_id(&session_ref.session_id),
                );
                parse_errors += 1;
            }
        }
    }

    // Phase 2: Summarize + ingest (async, needs Graphiti client).
    let config = Config::load()?;
    let mut client = graphiti::Client::new(&config).await?;
    let mut indexed = 0usize;
    let mut ingest_errors = 0usize;

    let total = jobs.len();
    for (i, job) in jobs.iter().enumerate() {
        eprint!(
            "  [{}/{}] {}/{} — {} ",
            i + 1,
            total,
            job.session_ref.agent,
            truncate_id(&job.session_ref.session_id),
            job.parsed.group_id,
        );

        if job.parsed.turns.is_empty() {
            eprintln!("(0 turns) ✓");
            continue;
        }

        match ingest(job, &mut client, opts.no_llm).await {
            Ok(()) => {
                eprintln!("({} turns) ✓", job.parsed.turns.len());
                indexed += 1;
            }
            Err(e) => {
                eprintln!("✗ {e}");
                ingest_errors += 1;
            }
        }
    }

    let _ = client.shutdown().await;

    // Phase 3: Record all successfully processed sessions in the index DB.
    // Done after ingestion so we only mark sessions that were actually sent to Graphiti.
    // We re-open the DB here to keep it out of the async section.
    let db = db::IndexDb::open()?;
    for job in &jobs {
        let _ = db.mark_indexed(
            job.session_ref.agent,
            &job.session_ref.session_id,
            &job.session_ref.source_path.to_string_lossy(),
            &job.session_ref.fingerprint,
            &job.parsed.group_id,
            job.parsed.turns.len(),
        );
    }

    let errors = parse_errors + ingest_errors;
    eprintln!("\nDone. {indexed} indexed, {errors} errors, {skipped} skipped.");

    Ok(SyncReport {
        discovered,
        new,
        modified: 0,
        skipped,
        indexed,
        errors,
    })
}

fn discover(parsers: &[Box<dyn SessionParser>]) -> Result<Vec<SessionRef>> {
    let mut all = Vec::new();

    for parser in parsers {
        match parser.discover() {
            Ok(sessions) => all.extend(sessions),
            Err(e) => eprintln!("Discovery error: {e}"),
        }
    }

    all.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    Ok(all)
}

fn build_parsers(opts: &SyncOptions) -> Vec<Box<dyn SessionParser>> {
    let mut parsers: Vec<Box<dyn SessionParser>> = Vec::new();

    let include_cc = opts
        .agent_filter
        .is_none_or(|a| a == Agent::ClaudeCode);
    let include_oc = opts
        .agent_filter
        .is_none_or(|a| a == Agent::OpenCode);

    if include_cc {
        parsers.push(Box::new(parsers::claude_code::ClaudeCodeParser));
    }
    if include_oc {
        parsers.push(Box::new(parsers::opencode::OpenCodeParser));
    }

    parsers
}

async fn ingest(
    job: &IndexJob,
    client: &mut graphiti::Client,
    no_llm: bool,
) -> Result<()> {
    let summary = summarize::summarize(&job.parsed, no_llm).await?;

    let req = AddMemoryRequest {
        name: format!(
            "{}-session-{}",
            job.session_ref.agent,
            truncate_id(&job.session_ref.session_id)
        ),
        episode_body: summary,
        group_id: Some(job.parsed.group_id.clone()),
        source: "text".into(),
        source_description: format!(
            "{} session {}, {} to {}",
            job.session_ref.agent,
            job.session_ref.session_id,
            job.parsed.started_at.format("%Y-%m-%d %H:%M"),
            job.parsed.ended_at.format("%Y-%m-%d %H:%M"),
        ),
        uuid: None,
        sync: false,
        reference_time: Some(job.parsed.started_at.to_rfc3339()),
    };

    client.add_memory(req).await?;
    Ok(())
}

fn truncate_id(id: &str) -> &str {
    if id.len() > 12 {
        &id[..12]
    } else {
        id
    }
}

/// Run a background sync scoped to the current project. Intended for `ekko serve` startup.
pub async fn background_sync() {
    let group = std::env::current_dir()
        .ok()
        .and_then(|cwd| crate::project::detect_group_id(&cwd));

    let opts = SyncOptions {
        full: false,
        agent_filter: None,
        group_filter: group,
        since: None,
        no_llm: false,
        dry_run: false,
        limit: Some(50),
    };

    match sync(opts).await {
        Ok(report) if report.indexed > 0 => {
            eprintln!("[background sync] {report}");
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("[background sync] failed: {e}");
        }
    }
}
