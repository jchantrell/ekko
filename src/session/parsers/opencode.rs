use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::PathBuf;

use super::SessionParser;
use crate::project;
use crate::session::normalize::*;

pub struct OpenCodeParser;

impl SessionParser for OpenCodeParser {
    fn discover(&self) -> Result<Vec<SessionRef>> {
        let db_path = db_path();
        if !db_path.exists() {
            return Ok(Vec::new());
        }

        let conn = Connection::open_with_flags(
            &db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_context(|| format!("failed to open opencode db at {}", db_path.display()))?;

        let mut stmt = conn.prepare(
            "SELECT s.id, s.directory, s.time_created, s.time_updated, p.worktree
             FROM session s
             LEFT JOIN project p ON s.project_id = p.id
             ORDER BY s.time_updated DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(OpenCodeSession {
                    id: row.get(0)?,
                    directory: row.get(1)?,
                    time_created: row.get(2)?,
                    time_updated: row.get(3)?,
                    worktree: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .map(|s| {
                let project_hint = s
                    .worktree
                    .as_deref()
                    .or(Some(&s.directory))
                    .and_then(|p| {
                        std::path::Path::new(p)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                    });

                let modified_at = DateTime::from_timestamp_millis(s.time_updated);
                let fingerprint = s.time_updated.to_string();

                SessionRef {
                    agent: Agent::OpenCode,
                    session_id: s.id,
                    project_hint,
                    source_path: db_path.clone(),
                    fingerprint,
                    modified_at,
                }
            })
            .collect();

        Ok(sessions)
    }

    fn parse(&self, session: &SessionRef) -> Result<ParsedSession> {
        let conn = Connection::open_with_flags(
            &session.source_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        let (directory, time_created, time_updated, worktree): (
            String,
            i64,
            i64,
            Option<String>,
        ) = conn.query_row(
            "SELECT s.directory, s.time_created, s.time_updated, p.worktree
             FROM session s
             LEFT JOIN project p ON s.project_id = p.id
             WHERE s.id = ?",
            [&session.session_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let mut msg_stmt = conn.prepare(
            "SELECT m.id, m.data, m.time_created
             FROM message m
             WHERE m.session_id = ?
             ORDER BY m.time_created",
        )?;

        let mut part_stmt = conn.prepare(
            "SELECT p.data FROM part p WHERE p.message_id = ? ORDER BY p.time_created",
        )?;

        let mut turns: Vec<Turn> = Vec::new();
        let mut files_touched = BTreeSet::new();
        let mut tools_used = BTreeSet::new();
        let mut model: Option<String> = None;
        let mut current_user_msg: Option<String> = None;

        let messages: Vec<(String, String, i64)> = msg_stmt
            .query_map([&session.session_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        for (msg_id, msg_data_str, msg_time) in &messages {
            let msg_data: Value = match serde_json::from_str(msg_data_str) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let role = msg_data
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("");

            if model.is_none() {
                if let Some(m) = msg_data.get("modelID").and_then(|v| v.as_str()) {
                    model = Some(m.to_string());
                }
            }

            let parts: Vec<Value> = part_stmt
                .query_map([msg_id], |row| row.get::<_, String>(0))?
                .filter_map(|r| r.ok())
                .filter_map(|s| serde_json::from_str(&s).ok())
                .collect();

            match role {
                "user" => {
                    let mut text_parts = Vec::new();
                    for part in &parts {
                        let ptype = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        if ptype == "text" {
                            if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                                text_parts.push(t.to_string());
                            }
                        }
                    }
                    if !text_parts.is_empty() {
                        current_user_msg = Some(text_parts.join("\n"));
                    }
                }
                "assistant" => {
                    let mut assistant_text = String::new();
                    let mut turn_tools = Vec::new();

                    for part in &parts {
                        let ptype = part.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match ptype {
                            "text" => {
                                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                                    assistant_text.push_str(t);
                                    assistant_text.push('\n');
                                }
                            }
                            "tool" => {
                                let tool_name = part
                                    .get("tool")
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("unknown");
                                tools_used.insert(tool_name.to_string());

                                let state = &part["state"];
                                let input = &state["input"];

                                let (input_summary, target_file) =
                                    summarize_oc_tool(tool_name, input);
                                if let Some(ref f) = target_file {
                                    files_touched.insert(f.clone());
                                }

                                turn_tools.push(ToolCall {
                                    name: tool_name.to_string(),
                                    input_summary,
                                    target_file,
                                });
                            }
                            "patch" => {
                                if let Some(files) =
                                    part.get("files").and_then(|f| f.as_array())
                                {
                                    for f in files {
                                        if let Some(s) = f.as_str() {
                                            files_touched.insert(s.to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    let ts = DateTime::from_timestamp_millis(*msg_time).unwrap_or_else(Utc::now);

                    if !assistant_text.trim().is_empty() || !turn_tools.is_empty() {
                        turns.push(Turn {
                            user_message: current_user_msg.take(),
                            assistant_message: assistant_text.trim().to_string(),
                            tool_calls: turn_tools,
                            timestamp: ts,
                        });
                    }
                }
                _ => {}
            }
        }

        let project_dir = worktree.as_deref().unwrap_or(&directory);
        let group_id = project::detect_group_id(std::path::Path::new(project_dir))
            .or_else(|| session.project_hint.clone().map(project::sanitize_group_id))
            .unwrap_or_else(|| "unknown".into());

        let started_at = DateTime::from_timestamp_millis(time_created).unwrap_or_else(Utc::now);
        let ended_at = DateTime::from_timestamp_millis(time_updated).unwrap_or_else(Utc::now);

        Ok(ParsedSession {
            session_id: session.session_id.clone(),
            agent: Agent::OpenCode,
            group_id,
            started_at,
            ended_at,
            turns,
            files_touched: files_touched.into_iter().collect(),
            tools_used: tools_used.into_iter().collect(),
            model,
        })
    }
}

struct OpenCodeSession {
    id: String,
    directory: String,
    #[allow(dead_code)]
    time_created: i64,
    time_updated: i64,
    worktree: Option<String>,
}

fn db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("opencode")
        .join("opencode.db")
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

fn summarize_oc_tool(tool_name: &str, input: &Value) -> (String, Option<String>) {
    match tool_name {
        "read" => {
            let path = input
                .get("filePath")
                .or_else(|| input.get("file_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("read {path}"), Some(path.to_string()))
        }
        "edit" => {
            let path = input
                .get("filePath")
                .or_else(|| input.get("file_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("edit {path}"), Some(path.to_string()))
        }
        "write" => {
            let path = input
                .get("filePath")
                .or_else(|| input.get("file_path"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("write {path}"), Some(path.to_string()))
        }
        "bash" => {
            let cmd = input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            (format!("bash: {}", truncate(cmd, 120)), None)
        }
        _ => {
            let s = serde_json::to_string(input).unwrap_or_default();
            (truncate(&s, 80).to_string(), None)
        }
    }
}
