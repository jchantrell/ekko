use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::SessionParser;
use crate::project;
use crate::session::normalize::*;

pub struct ClaudeCodeParser;

impl SessionParser for ClaudeCodeParser {
    fn discover(&self) -> Result<Vec<SessionRef>> {
        let claude_dir = dirs::home_dir()
            .context("no home directory")?
            .join(".claude")
            .join("projects");

        if !claude_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        for project_entry in std::fs::read_dir(&claude_dir)? {
            let project_entry = project_entry?;
            let project_path = project_entry.path();
            if !project_path.is_dir() {
                continue;
            }

            let project_hint = decode_project_dir(&project_path);

            for entry in std::fs::read_dir(&project_path)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().is_some_and(|e| e == "jsonl") && path.is_file() {
                    let meta = entry.metadata()?;
                    let session_id = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();

                    let mtime = meta.modified().ok().and_then(|t| {
                        let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                        DateTime::from_timestamp(dur.as_secs() as i64, 0)
                    });

                    let fingerprint = format!(
                        "{}:{}",
                        meta.len(),
                        mtime.map(|t| t.timestamp()).unwrap_or(0)
                    );

                    sessions.push(SessionRef {
                        agent: Agent::ClaudeCode,
                        session_id,
                        project_hint: project_hint.clone(),
                        source_path: path,
                        fingerprint,
                        modified_at: mtime,
                    });
                }
            }
        }

        Ok(sessions)
    }

    fn parse(&self, session: &SessionRef) -> Result<ParsedSession> {
        let file = File::open(&session.source_path)
            .with_context(|| format!("failed to open {}", session.source_path.display()))?;
        let reader = BufReader::new(file);

        let mut turns_by_prompt: HashMap<String, TurnBuilder> = HashMap::new();
        let mut prompt_order: Vec<String> = Vec::new();
        let mut files_touched = BTreeSet::new();
        let mut tools_used = BTreeSet::new();
        let mut model: Option<String> = None;
        let mut cwd: Option<String> = None;
        let mut first_ts: Option<DateTime<Utc>> = None;
        let mut last_ts: Option<DateTime<Utc>> = None;

        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            let val: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let msg_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

            if let Some(ts) = parse_timestamp(&val) {
                if first_ts.is_none() || ts < first_ts.unwrap() {
                    first_ts = Some(ts);
                }
                if last_ts.is_none() || ts > last_ts.unwrap() {
                    last_ts = Some(ts);
                }
            }

            if cwd.is_none()
                && let Some(c) = val.get("cwd").and_then(|v| v.as_str())
            {
                cwd = Some(c.to_string());
            }

            match msg_type {
                "user" => {
                    let prompt_id = val
                        .get("promptId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    if prompt_id.is_empty() {
                        continue;
                    }

                    let message = &val["message"];
                    let content = &message["content"];

                    if content.is_string() {
                        let text = content.as_str().unwrap_or("").to_string();
                        let ts = parse_timestamp(&val).unwrap_or_else(Utc::now);

                        if !turns_by_prompt.contains_key(&prompt_id) {
                            prompt_order.push(prompt_id.clone());
                        }
                        let turn = turns_by_prompt.entry(prompt_id).or_default();
                        turn.user_message = Some(text);
                        if turn.timestamp.is_none() {
                            turn.timestamp = Some(ts);
                        }
                    }
                }
                "assistant" => {
                    let prompt_id = val
                        .get("promptId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let message = &val["message"];

                    if model.is_none()
                        && let Some(m) = message.get("model").and_then(|v| v.as_str())
                    {
                        model = Some(m.to_string());
                    }

                    if let Some(content_arr) = message.get("content").and_then(|c| c.as_array()) {
                        let ts = parse_timestamp(&val).unwrap_or_else(Utc::now);

                        if !prompt_id.is_empty() && !turns_by_prompt.contains_key(&prompt_id) {
                            prompt_order.push(prompt_id.clone());
                        }
                        let turn = if prompt_id.is_empty() {
                            turns_by_prompt.entry(String::new()).or_default()
                        } else {
                            turns_by_prompt.entry(prompt_id).or_default()
                        };

                        if turn.timestamp.is_none() {
                            turn.timestamp = Some(ts);
                        }

                        for block in content_arr {
                            let block_type =
                                block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                            match block_type {
                                "text" => {
                                    if let Some(text) = block.get("text").and_then(|t| t.as_str())
                                    {
                                        turn.assistant_text.push_str(text);
                                        turn.assistant_text.push('\n');
                                    }
                                }
                                "tool_use" => {
                                    let tool_name = block
                                        .get("name")
                                        .and_then(|n| n.as_str())
                                        .unwrap_or("unknown");
                                    tools_used.insert(tool_name.to_string());

                                    let (input_summary, target_file) =
                                        summarize_tool_input(tool_name, &block["input"]);
                                    if let Some(ref f) = target_file {
                                        files_touched.insert(f.clone());
                                    }

                                    turn.tool_calls.push(ToolCall {
                                        name: tool_name.to_string(),
                                        input_summary,
                                        target_file,
                                    });
                                }
                                _ => {}
                            }
                        }
                    }
                }
                "file-history-snapshot" => {
                    if let Some(snapshot) = val.get("snapshot")
                        && let Some(backups) = snapshot.get("trackedFileBackups")
                        && let Some(obj) = backups.as_object()
                    {
                        for key in obj.keys() {
                            files_touched.insert(key.clone());
                        }
                    }
                }
                _ => {}
            }
        }

        let origin = cwd
            .as_deref()
            .map(Path::new)
            .and_then(project::detect_origin)
            .or_else(|| session.project_hint.clone())
            .unwrap_or_else(|| "unknown".into());

        let now = Utc::now();
        let started = first_ts.unwrap_or(now);
        let ended = last_ts.unwrap_or(now);

        let turns: Vec<Turn> = prompt_order
            .into_iter()
            .filter_map(|pid| {
                let tb = turns_by_prompt.remove(&pid)?;
                let assistant_text = tb.assistant_text.trim().to_string();
                if assistant_text.is_empty() && tb.user_message.is_none() {
                    return None;
                }
                Some(Turn {
                    user_message: tb.user_message,
                    assistant_message: assistant_text,
                    tool_calls: tb.tool_calls,
                    timestamp: tb.timestamp.unwrap_or(started),
                })
            })
            .collect();

        Ok(ParsedSession {
            session_id: session.session_id.clone(),
            agent: Agent::ClaudeCode,
            origin,
            started_at: started,
            ended_at: ended,
            turns,
            files_touched: files_touched.into_iter().collect(),
            tools_used: tools_used.into_iter().collect(),
            model,
        })
    }
}

#[derive(Default)]
struct TurnBuilder {
    user_message: Option<String>,
    assistant_text: String,
    tool_calls: Vec<ToolCall>,
    timestamp: Option<DateTime<Utc>>,
}

fn parse_timestamp(val: &Value) -> Option<DateTime<Utc>> {
    val.get("timestamp")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.to_utc())
}

/// Decode Claude Code's project directory name back to an actual path.
/// `-home-joel-Workspace-ekko` → `home/joel/Workspace/ekko`
/// We don't reconstruct the leading `/` — we just need the leaf for group_id.
fn decode_project_dir(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let decoded = name.replace('-', "/");
    let decoded = decoded.trim_start_matches('/');

    // Extract just the last path component as a hint
    decoded.rsplit('/').next().map(|s| s.to_string())
}

fn summarize_tool_input(tool_name: &str, input: &Value) -> (String, Option<String>) {
    match tool_name {
        "Read" => {
            let path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("read {path}"), Some(path.to_string()))
        }
        "Edit" => {
            let path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("edit {path}"), Some(path.to_string()))
        }
        "Write" => {
            let path = input
                .get("file_path")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("write {path}"), Some(path.to_string()))
        }
        "Bash" => {
            let cmd = input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let truncated = truncate(cmd, 120);
            (format!("bash: {truncated}"), None)
        }
        "Glob" => {
            let pattern = input
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("glob {pattern}"), None)
        }
        "Grep" => {
            let pattern = input
                .get("pattern")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("grep {pattern}"), None)
        }
        "Agent" => {
            let desc = input
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("agent: {desc}"), None)
        }
        _ => {
            let s = serde_json::to_string(input).unwrap_or_default();
            (truncate(&s, 80).to_string(), None)
        }
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let end = s
            .char_indices()
            .nth(max)
            .map(|(i, _)| i)
            .unwrap_or(s.len());
        &s[..end]
    }
}
