//! Database connection and schema migrations.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::Result;
use crate::util::MutexExt;

/// Embedded schema (v1). Idempotent — safe to run on every startup.
const SCHEMA_V1: &str = include_str!("schema.sql");

/// Thread-safe SQLite handle shared across repositories.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the database at `path` and run migrations.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    /// Open an in-memory database (used by tests).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure(&conn)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock_recover();
        conn.execute_batch(SCHEMA_V1)?;
        let _ = conn.execute(
            "UPDATE settings SET value = '\"whisper-large-v3-turbo\"' WHERE key = 'stt_model' AND value = '\"small\"'",
            [],
        );
        Ok(())
    }

    /// Run a closure with locked access to the connection.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock_recover();
        f(&conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_tables() {
        let db = Database::open_in_memory().unwrap();
        let count: i64 = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN \
                     ('transcriptions','dictionary_entries','snippets','settings')",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(count, 4);
    }
}
