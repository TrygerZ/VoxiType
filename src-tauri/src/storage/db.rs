//! Database connection and schema migrations.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::error::Result;
use crate::util::MutexExt;

/// Embedded schema (v1). Baseline schema for new databases.
const SCHEMA_V1: &str = include_str!("schema.sql");

/// Current database schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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
        let mut conn = self.conn.lock_recover();
        let mut version = get_user_version(&conn)?;

        if version == 0 {
            // New database initializes with base schema; existing DB is recognized at v1.
            if !has_user_tables(&conn)? {
                conn.execute_batch(SCHEMA_V1)?;
            }
            version = 1;
            set_user_version(&conn, version)?;
        }

        while version < CURRENT_SCHEMA_VERSION {
            let next_version = version + 1;
            let tx = conn.transaction()?;
            apply_migration_step(&tx, next_version)?;
            tx.commit()?;
            set_user_version(&conn, next_version)?;
            version = next_version;
        }

        Ok(())
    }

    /// Read the current SQLite schema version (PRAGMA user_version).
    pub fn schema_version(&self) -> Result<u32> {
        let conn = self.conn.lock_recover();
        get_user_version(&conn)
    }

    /// Run a closure with locked access to the connection.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock_recover();
        f(&conn)
    }
}

fn get_user_version(conn: &Connection) -> Result<u32> {
    let version: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(version)
}

fn set_user_version(conn: &Connection, version: u32) -> Result<()> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn has_user_tables(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?",
        [table_name],
        |r| r.get(0),
    )?;
    Ok(count > 0)
}

fn apply_migration_step(conn: &Connection, target_version: u32) -> Result<()> {
    match target_version {
        2 => migrate_to_v2(conn),
        _ => Err(crate::error::AppError::storage(format!(
            "Unsupported database migration step: {target_version}"
        ))),
    }
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    // Migration v2: migrate legacy default model and drop redundant snippets index.
    // Explicit guard: settings table may be absent in isolated unit test databases.
    if table_exists(conn, "settings")? {
        conn.execute(
            "UPDATE settings SET value = '\"whisper-large-v3-turbo\"' WHERE key = 'stt_model' AND value = '\"small\"'",
            [],
        )?;
    }
    conn.execute("DROP INDEX IF EXISTS idx_snippets_trigger", [])?;
    Ok(())
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

    #[test]
    fn migration_drops_redundant_snippets_index() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE snippets (id TEXT PRIMARY KEY, trigger_phrase TEXT NOT NULL UNIQUE);
             CREATE INDEX idx_snippets_trigger ON snippets(trigger_phrase);",
        )
        .unwrap();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate().unwrap();
        let exists: i64 = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_snippets_trigger'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(exists, 0);
    }

    #[test]
    fn new_empty_db_gets_full_schema_and_latest_version() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

        let table_count: i64 = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN \
                     ('transcriptions','dictionary_entries','snippets','settings','usage_stats','per_app_modes')",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(table_count, 6);

        let stt_model: String = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT value FROM settings WHERE key = 'stt_model'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(stt_model, "\"whisper-large-v3-turbo\"");
    }

    #[test]
    fn existing_db_v1_migrates_without_data_loss() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.pragma_update(None, "user_version", 0).unwrap();

        conn.execute(
            "INSERT INTO transcriptions (id, text_raw, text_formatted) VALUES ('h-1', 'raw hello', 'Hello')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dictionary_entries (id, word, replacement) VALUES ('d-1', 'voxitype', 'VoxiType')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO snippets (id, name, trigger_phrase, content) VALUES ('s-1', 'sig', 'my sign', 'Best regards')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('stt_model', '\"small\"')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('user_custom_key', '\"custom_val\"')",
            [],
        )
        .unwrap();
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snippets_trigger ON snippets(trigger_phrase)",
            [],
        );

        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate().unwrap();

        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

        let (raw_text, fmt_text): (String, String) = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT text_raw, text_formatted FROM transcriptions WHERE id = 'h-1'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .unwrap();
        assert_eq!(raw_text, "raw hello");
        assert_eq!(fmt_text, "Hello");

        let dict_word: String = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT word FROM dictionary_entries WHERE id = 'd-1'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(dict_word, "voxitype");

        let snippet_content: String = db
            .with_conn(|c| {
                Ok(
                    c.query_row("SELECT content FROM snippets WHERE id = 's-1'", [], |r| {
                        r.get(0)
                    })?,
                )
            })
            .unwrap();
        assert_eq!(snippet_content, "Best regards");

        let custom_setting: String = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT value FROM settings WHERE key = 'user_custom_key'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(custom_setting, "\"custom_val\"");

        let stt_model: String = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT value FROM settings WHERE key = 'stt_model'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(stt_model, "\"whisper-large-v3-turbo\"");

        let idx_count: i64 = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_snippets_trigger'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(idx_count, 0);
    }

    #[test]
    fn migration_is_idempotent_on_reopen() {
        let temp_dir =
            std::env::temp_dir().join(format!("voxitype_idempotent_{}", uuid::Uuid::new_v4()));
        let db_path = temp_dir.join("test.db");

        {
            let db = Database::open(&db_path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
            db.with_conn(|c| {
                c.execute(
                    "INSERT INTO transcriptions (id, text_raw, text_formatted) VALUES ('t-1', 'hi', 'Hi')",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        }

        {
            let db = Database::open(&db_path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

            let count: i64 = db
                .with_conn(|c| {
                    Ok(c.query_row(
                        "SELECT COUNT(*) FROM transcriptions WHERE id = 't-1'",
                        [],
                        |r| r.get(0),
                    )?)
                })
                .unwrap();
            assert_eq!(count, 1);
        }

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn migration_failure_is_reported_not_swallowed() {
        let conn = Connection::open_in_memory().unwrap();
        let err = apply_migration_step(&conn, 999).unwrap_err();
        assert!(
            err.message.contains("999") || err.message.contains("Unsupported"),
            "Error must identify the failed migration step"
        );
    }

    #[test]
    fn migration_sql_error_rolls_back_and_surfaces_error() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT);
             INSERT INTO settings (key, value) VALUES ('stt_model', '\"small\"');
             CREATE TRIGGER prevent_update BEFORE UPDATE ON settings BEGIN
                 SELECT RAISE(FAIL, 'simulated migration error');
             END;",
        )
        .unwrap();

        let db = Database {
            conn: Mutex::new(conn),
        };
        let err = db.migrate().unwrap_err();
        assert!(
            err.message.contains("simulated migration error"),
            "Database migration failure must propagate the underlying SQL error: {err}"
        );
        assert_eq!(db.schema_version().unwrap(), 1);
    }
}
