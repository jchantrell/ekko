use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct GroupMeta {
    pub group_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

pub struct GroupsDb {
    conn: Connection,
}

impl GroupsDb {
    pub fn open() -> Result<Self> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let conn = Connection::open(&path)
            .with_context(|| format!("failed to open groups db at {}", path.display()))?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn db_path() -> Result<PathBuf> {
        Ok(Config::data_dir()?.join("groups.db"))
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS groups (
                group_id    TEXT PRIMARY KEY,
                name        TEXT,
                description TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            )",
        )?;
        Ok(())
    }

    pub fn upsert(&self, group_id: &str, name: Option<&str>, description: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO groups (group_id, name, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(group_id) DO UPDATE SET
                name = COALESCE(?2, groups.name),
                description = COALESCE(?3, groups.description),
                updated_at = ?4",
            (group_id, name, description, &now),
        )?;
        Ok(())
    }

    pub fn ensure_exists(&self, group_id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT OR IGNORE INTO groups (group_id, name, description, created_at, updated_at)
             VALUES (?1, NULL, NULL, ?2, ?2)",
            (group_id, &now),
        )?;
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<GroupMeta>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT group_id, name, description FROM groups ORDER BY group_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(GroupMeta {
                group_id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
            })
        })?;
        let mut groups = Vec::new();
        for row in rows {
            groups.push(row?);
        }
        Ok(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn test_db() -> GroupsDb {
        let conn = Connection::open_in_memory().unwrap();
        let db = GroupsDb { conn };
        db.migrate().unwrap();
        db
    }

    #[test]
    fn ensure_exists_creates_row() {
        let db = test_db();
        db.ensure_exists("myproject").unwrap();

        let groups = db.list().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group_id, "myproject");
        assert_eq!(groups[0].name, None);
        assert_eq!(groups[0].description, None);
    }

    #[test]
    fn ensure_exists_is_idempotent() {
        let db = test_db();
        db.ensure_exists("myproject").unwrap();
        db.ensure_exists("myproject").unwrap();

        let groups = db.list().unwrap();
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn upsert_sets_name_and_description() {
        let db = test_db();
        db.upsert("ekko", Some("ekko"), Some("Agent memory system")).unwrap();

        let groups = db.list().unwrap();
        assert_eq!(groups[0].name.as_deref(), Some("ekko"));
        assert_eq!(groups[0].description.as_deref(), Some("Agent memory system"));
    }

    #[test]
    fn upsert_preserves_existing_fields_when_null() {
        let db = test_db();
        db.upsert("ekko", Some("ekko"), Some("Agent memory")).unwrap();
        db.upsert("ekko", None, None).unwrap();

        let groups = db.list().unwrap();
        assert_eq!(groups[0].name.as_deref(), Some("ekko"));
        assert_eq!(groups[0].description.as_deref(), Some("Agent memory"));
    }

    #[test]
    fn upsert_updates_existing_fields() {
        let db = test_db();
        db.upsert("ekko", Some("ekko"), Some("v1")).unwrap();
        db.upsert("ekko", Some("Ekko"), Some("v2")).unwrap();

        let groups = db.list().unwrap();
        assert_eq!(groups[0].name.as_deref(), Some("Ekko"));
        assert_eq!(groups[0].description.as_deref(), Some("v2"));
    }

    #[test]
    fn list_returns_sorted() {
        let db = test_db();
        db.ensure_exists("zebra").unwrap();
        db.ensure_exists("alpha").unwrap();
        db.ensure_exists("middle").unwrap();

        let groups = db.list().unwrap();
        let ids: Vec<&str> = groups.iter().map(|g| g.group_id.as_str()).collect();
        assert_eq!(ids, vec!["alpha", "middle", "zebra"]);
    }
}
