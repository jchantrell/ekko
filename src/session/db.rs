use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use super::normalize::{Agent, SessionRef};
use crate::config::Config;

pub struct IndexDb {
    conn: Connection,
}

impl IndexDb {
    pub fn open() -> Result<Self> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("failed to open index db at {}", path.display()))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn db_path() -> Result<PathBuf> {
        Ok(Config::data_dir()?.join("index.db"))
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS indexed_sessions (
                agent        TEXT NOT NULL,
                session_id   TEXT NOT NULL,
                source_path  TEXT NOT NULL,
                fingerprint  TEXT NOT NULL,
                group_id     TEXT NOT NULL,
                turn_count   INTEGER NOT NULL,
                indexed_at   TEXT NOT NULL,
                PRIMARY KEY (agent, session_id)
            )",
        )?;
        Ok(())
    }

    /// Check if a session needs indexing. Returns true if new or fingerprint changed.
    pub fn needs_indexing(&self, session: &SessionRef) -> Result<bool> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT fingerprint FROM indexed_sessions WHERE agent = ? AND session_id = ?",
        )?;
        let agent = session.agent.as_str();
        let result: Option<String> = stmt
            .query_row((&agent, &session.session_id), |row| row.get(0))
            .ok();

        match result {
            None => Ok(true),
            Some(fp) => Ok(fp != session.fingerprint),
        }
    }

    /// Record a successfully indexed session.
    pub fn mark_indexed(
        &self,
        agent: Agent,
        session_id: &str,
        source_path: &str,
        fingerprint: &str,
        group_id: &str,
        turn_count: usize,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR REPLACE INTO indexed_sessions
                (agent, session_id, source_path, fingerprint, group_id, turn_count, indexed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            (
                agent.as_str(),
                session_id,
                source_path,
                fingerprint,
                group_id,
                turn_count as i64,
                &now,
            ),
        )?;
        Ok(())
    }

    /// Remove a session from the index (for re-indexing).
    #[allow(dead_code)]
    pub fn remove(&self, agent: Agent, session_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM indexed_sessions WHERE agent = ? AND session_id = ?",
            (agent.as_str(), session_id),
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM indexed_sessions", ())?;
        Ok(())
    }
}
