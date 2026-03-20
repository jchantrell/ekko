use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Agent {
    ClaudeCode,
    OpenCode,
}

impl Agent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::OpenCode => "opencode",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude-code" | "claudecode" => Some(Self::ClaudeCode),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Lightweight reference to a session found on disk. No parsing yet.
#[derive(Debug, Clone)]
pub struct SessionRef {
    pub agent: Agent,
    pub session_id: String,
    pub project_hint: Option<String>,
    pub source_path: PathBuf,
    pub fingerprint: String,
    pub modified_at: Option<DateTime<Utc>>,
}

/// A parsed, normalized session ready for summarization.
#[derive(Debug)]
#[allow(dead_code)]
pub struct ParsedSession {
    pub session_id: String,
    pub agent: Agent,
    pub group_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub turns: Vec<Turn>,
    pub files_touched: Vec<String>,
    pub tools_used: Vec<String>,
    pub model: Option<String>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Turn {
    pub user_message: Option<String>,
    pub assistant_message: String,
    pub tool_calls: Vec<ToolCall>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ToolCall {
    pub name: String,
    pub input_summary: String,
    pub target_file: Option<String>,
}

/// Options controlling what and how to sync.
pub struct SyncOptions {
    pub full: bool,
    pub agent_filter: Option<Agent>,
    pub group_filter: Option<String>,
    pub since: Option<DateTime<Utc>>,
    pub no_llm: bool,
    pub dry_run: bool,
    pub limit: Option<usize>,
}

/// Report of what sync did.
#[allow(dead_code)]
pub struct SyncReport {
    pub discovered: usize,
    pub new: usize,
    pub modified: usize,
    pub skipped: usize,
    pub indexed: usize,
    pub errors: usize,
}

impl fmt::Display for SyncReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} indexed, {} skipped, {} errors (of {} discovered)",
            self.indexed, self.skipped, self.errors, self.discovered
        )
    }
}
