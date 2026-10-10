//! Database connection and schema migrations.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::util::MutexExt;

/// Embedded schema (v1). Baseline schema for new databases.
const SCHEMA_V1: &str = include_str!("schema.sql");

/// Current database schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 3;

/// How long a writer waits for a locked database before returning
/// `SQLITE_BUSY`. A second process or future nested access waits briefly
/// instead of failing immediately.
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Schema version that introduced the case-insensitive dictionary index.
/// Databases below this version are backed up before the v3 migration runs.
const DICTIONARY_NOCASE_SCHEMA_VERSION: u32 = 3;

/// Unique dictionary index definition. The collation must match the upsert
/// conflict target in `dictionary.rs` and the baseline in `schema.sql`.
///
/// SQLite `NOCASE` folds only ASCII A-Z, so it does not consider non-ASCII
/// casing variants equal. The v3 migration's `lower()` shares that ASCII-only
/// limitation. In-Rust replacement matching uses Unicode-aware `to_lowercase`,
/// so a word and its non-ASCII case variant (for example E-acute) are neither
/// deduped here nor rejected by the index.
const DICTIONARY_UNIQUE_INDEX_SQL: &str = "CREATE UNIQUE INDEX IF NOT EXISTS idx_dictionary_word
     ON dictionary_entries(word COLLATE NOCASE, language)";

/// Suffix appended to the database file name for the pre-v3 backup.
const PRE_V3_BACKUP_SUFFIX: &str = ".bak-v2";

/// WAL and shared-memory sidecar suffixes. In WAL mode committed rows may live
/// only in `-wal` until a checkpoint, so a main-file-only copy can be stale or
/// even table-less (L-9, QA Important-1).
const WAL_SIDECAR_SUFFIX: &str = "-wal";
const SHM_SIDECAR_SUFFIX: &str = "-shm";

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
        let existed = path.exists();
        let conn = Connection::open(path)?;
        Self::configure(&conn)?;
        if existed {
            back_up_before_v3_migration(&conn, path)?;
        }
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
        conn.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let mut conn = self.conn.lock_recover();
        let mut version = get_user_version(&conn)?;

        if version == 0 {
            // New database initializes with base schema; existing DB is recognized at v1.
            // This bootstrap step writes the base schema idempotently and carries no
            // data change, so it stays outside the per-step transaction (L-7).
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
            // Record the version inside the same transaction so a crash between
            // the data change and the version write can never re-run the step (L-7).
            set_user_version(&tx, next_version)?;
            tx.commit()?;
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
    ///
    /// The connection mutex is NOT reentrant: never call `with_conn` (or any
    /// repository method, which locks the same mutex) from inside a `with_conn`
    /// closure, or the call deadlocks.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
        let conn = self.conn.lock_recover();
        f(&conn)
    }
}

/// Copy the database file to a sibling backup before the v3 migration runs.
///
/// Runs outside the migration transaction: the backup is written as a temp
/// file and renamed into place, so a crash mid-copy leaves no partial backup.
/// An existing backup is never overwritten, and databases already at v3 (or
/// with no file, as in `open_in_memory`) are skipped.
///
/// The WAL is checkpointed into the main file first, so the backup holds every
/// committed row even when the previous process left data only in the `-wal`
/// sidecar (QA Important-1). Any remaining sidecars are copied alongside it.
fn back_up_before_v3_migration(conn: &Connection, path: &Path) -> Result<()> {
    if !path.exists() || get_user_version(conn)? >= DICTIONARY_NOCASE_SCHEMA_VERSION {
        return Ok(());
    }
    let backup_path = backup_path_for(path);
    if backup_path.exists() {
        tracing::info!(
            "Pre-v3 database backup already present at {}; keeping existing backup",
            backup_path.display()
        );
        return Ok(());
    }
    checkpoint_wal(conn)?;
    copy_verified(path, &backup_path)?;
    copy_sidecar(path, &backup_path, WAL_SIDECAR_SUFFIX);
    copy_sidecar(path, &backup_path, SHM_SIDECAR_SUFFIX);
    tracing::info!(
        "Backed up database to {} before v3 migration",
        backup_path.display()
    );
    Ok(())
}

/// Fold the WAL into the main database file so a subsequent file copy captures
/// every committed row. `TRUNCATE` leaves the `-wal` sidecar empty.
fn checkpoint_wal(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    Ok(())
}

/// Copy one sidecar file next to the backup. Best-effort: a missing sidecar is
/// skipped and a copy failure is logged rather than aborting the backup, since
/// the main file (already checkpointed) carries the committed data.
fn copy_sidecar(source_db: &Path, backup_db: &Path, suffix: &str) {
    let source = sidecar_path(source_db, suffix);
    if !source.is_file() {
        return;
    }
    let destination = sidecar_path(backup_db, suffix);
    if let Err(error) = copy_verified(&source, &destination) {
        tracing::warn!(
            "Could not copy database sidecar {} to backup: {error}",
            source.display()
        );
    }
}

fn sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = db_path.file_name().unwrap_or_default().to_os_string();
    file_name.push(suffix);
    db_path.with_file_name(file_name)
}

fn backup_path_for(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "voxitype.db".to_string());
    name.push_str(PRE_V3_BACKUP_SUFFIX);
    path.with_file_name(name)
}

/// Copy `source` to `destination` through a temp file plus rename, verifying
/// the byte length and SHA-256 so a truncated or corrupted copy is never
/// renamed into place. Matches the verification strength of `data_dir`'s copy.
fn copy_verified(source: &Path, destination: &Path) -> Result<()> {
    let temporary = destination.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| -> Result<()> {
        std::fs::copy(source, &temporary)?;
        let same_length = std::fs::metadata(source)?.len() == std::fs::metadata(&temporary)?.len();
        if !same_length || hash_file(source)? != hash_file(&temporary)? {
            return Err(crate::error::AppError::storage(format!(
                "database backup verification failed for {}",
                source.display()
            )));
        }
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn hash_file(path: &Path) -> std::io::Result<[u8; 32]> {
    let bytes = std::fs::read(path)?;
    Ok(Sha256::digest(bytes).into())
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
        3 => migrate_to_v3(conn),
        _ => Err(crate::error::AppError::storage(format!(
            "Unsupported database migration step: {target_version}"
        ))),
    }
}

/// Migration v3: deduplicate dictionary words case-insensitively, then
/// recreate the unique index with `COLLATE NOCASE` so casing variants can no
/// longer coexist (L-9). The caller backs up the database before this runs.
fn migrate_to_v3(conn: &Connection) -> Result<()> {
    // Explicit guard: the dictionary table may be absent in isolated unit tests.
    if !table_exists(conn, "dictionary_entries")? {
        return Ok(());
    }
    conn.execute("DROP INDEX IF EXISTS idx_dictionary_word", [])?;
    deduplicate_dictionary_words(conn)?;
    conn.execute(DICTIONARY_UNIQUE_INDEX_SQL, [])?;
    Ok(())
}

/// Keep one row per `(lower(word), language)`, preferring the highest
/// `usage_count`, then the newest `updated_at`, then the lowest `id` for
/// determinism. Every dropped row is logged; a summary count follows.
fn deduplicate_dictionary_words(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, word, language FROM dictionary_entries
         WHERE EXISTS (
             SELECT 1 FROM dictionary_entries AS other
             WHERE other.id != dictionary_entries.id
               AND lower(other.word) = lower(dictionary_entries.word)
               AND other.language = dictionary_entries.language
               AND (
                   other.usage_count > dictionary_entries.usage_count
                   OR (other.usage_count = dictionary_entries.usage_count
                       AND other.updated_at > dictionary_entries.updated_at)
                   OR (other.usage_count = dictionary_entries.usage_count
                       AND other.updated_at = dictionary_entries.updated_at
                       AND other.id < dictionary_entries.id)
               )
         )",
    )?;
    let dropped: Vec<(String, String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);

    for (id, word, language) in &dropped {
        conn.execute("DELETE FROM dictionary_entries WHERE id = ?1", [id])?;
        tracing::info!("Dropped duplicate dictionary word {id} ({word}, {language})");
    }
    if !dropped.is_empty() {
        tracing::info!(
            "Dropped {} duplicate dictionary rows in v3 migration",
            dropped.len()
        );
    }
    Ok(())
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
    fn configure_sets_busy_timeout() {
        let db = Database::open_in_memory().unwrap();
        let timeout: i64 = db
            .with_conn(|c| Ok(c.query_row("PRAGMA busy_timeout", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(timeout, 5000);
    }

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

    /// Build a v2-shaped dictionary table with the old BINARY unique index,
    /// insert two rows that differ only by casing, and return the raw handle.
    fn v2_dictionary_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE dictionary_entries (
                 id TEXT PRIMARY KEY,
                 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
                 updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                 word TEXT NOT NULL,
                 pronunciation TEXT,
                 category TEXT NOT NULL DEFAULT 'custom',
                 replacement TEXT,
                 language TEXT NOT NULL DEFAULT 'id',
                 usage_count INTEGER NOT NULL DEFAULT 0,
                 is_active INTEGER NOT NULL DEFAULT 1,
                 metadata TEXT
             );
             CREATE UNIQUE INDEX idx_dictionary_word ON dictionary_entries(word, language);
             INSERT INTO dictionary_entries (id, word, language, usage_count)
                 VALUES ('low', 'js', 'id', 1);
             INSERT INTO dictionary_entries (id, word, language, usage_count)
                 VALUES ('high', 'JS', 'id', 5);
             PRAGMA user_version = 2;",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migration_v3_dedupes_case_insensitive_words() {
        let conn = v2_dictionary_conn();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate().unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

        let rows: Vec<(String, String)> = db
            .with_conn(|c| {
                let mut stmt = c.prepare("SELECT id, word FROM dictionary_entries ORDER BY id")?;
                let rows = stmt
                    .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(rows)
            })
            .unwrap();
        // The higher usage_count row survives with its original casing.
        assert_eq!(rows, vec![("high".to_string(), "JS".to_string())]);

        // The recreated index is NOCASE: inserting the other casing conflicts.
        let conflict = db.with_conn(|c| {
            c.execute(
                "INSERT INTO dictionary_entries (id, word, language) VALUES ('x', 'js', 'id')",
                [],
            )
            .map_err(Into::into)
        });
        assert!(conflict.is_err(), "NOCASE index must reject 'js' vs 'JS'");
    }

    #[test]
    fn migration_v3_is_idempotent() {
        let conn = v2_dictionary_conn();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.migrate().unwrap();
        db.migrate().unwrap();

        let count: i64 = db
            .with_conn(|c| {
                Ok(c.query_row("SELECT COUNT(*) FROM dictionary_entries", [], |r| r.get(0))?)
            })
            .unwrap();
        assert_eq!(count, 1);

        let sql: String = db
            .with_conn(|c| {
                Ok(c.query_row(
                    "SELECT sql FROM sqlite_master WHERE type='index' AND name='idx_dictionary_word'",
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert!(sql.contains("NOCASE"), "index must remain NOCASE: {sql}");
    }

    #[test]
    fn open_backs_up_v2_database_once_and_never_overwrites() {
        let temp_dir =
            std::env::temp_dir().join(format!("voxitype_backup_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("voxitype.db");
        let backup_path = temp_dir.join("voxitype.db.bak-v2");

        {
            let conn = Connection::open(&db_path).unwrap();
            conn.execute_batch(SCHEMA_V1).unwrap();
            conn.pragma_update(None, "user_version", 2).unwrap();
        }

        {
            let db = Database::open(&db_path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        }
        assert!(backup_path.is_file(), "open must leave a pre-v3 backup");
        // The backup is the pre-migration database, not the migrated schema.
        let backup_version: u32 = Connection::open(&backup_path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            backup_version, 2,
            "backup must hold the pre-migration database"
        );

        // A second open must not overwrite the existing backup.
        std::fs::write(&backup_path, b"sentinel").unwrap();
        {
            let db = Database::open(&db_path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        }
        assert_eq!(std::fs::read(&backup_path).unwrap(), b"sentinel");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn backup_captures_committed_rows_from_uncheckpointed_wal() {
        let temp_dir = std::env::temp_dir().join(format!("voxitype_wal_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let db_path = temp_dir.join("voxitype.db");
        let backup_path = temp_dir.join("voxitype.db.bak-v2");

        // Seed a v2 database whose committed rows live only in the WAL, as they
        // would after a crashed prior process. Leaving the connection open (not
        // dropped) skips the clean-close checkpoint, so the main file is stale.
        let crashed = Connection::open(&db_path).unwrap();
        crashed.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
        crashed.execute_batch(SCHEMA_V1).unwrap();
        crashed
            .execute(
                "INSERT INTO transcriptions (id, text_raw, text_formatted) VALUES ('w-1', 'wal', 'WAL')",
                [],
            )
            .unwrap();
        crashed.pragma_update(None, "user_version", 2).unwrap();

        {
            let db = Database::open(&db_path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        }
        drop(crashed);

        // The backup must hold the row that only the WAL carried.
        let backup = Connection::open(&backup_path).unwrap();
        let count: i64 = backup
            .query_row(
                "SELECT COUNT(*) FROM transcriptions WHERE id = 'w-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "backup must include WAL-resident committed rows");

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
