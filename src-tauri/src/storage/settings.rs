//! Settings manager backed by the SQLite `settings` table.
//!
//! Values are stored JSON-encoded so any serializable type round-trips.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::db::Database;
use crate::error::{AppError, Result};

pub struct SettingsManager<'a> {
    db: &'a Database,
}

impl<'a> SettingsManager<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Get a raw JSON value for a key.
    pub fn get_raw(&self, key: &str) -> Result<Option<Value>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare("SELECT value FROM settings WHERE key = ?1")?;
            let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
            match rows.next() {
                Some(s) => Ok(Some(parse_stored_value(&s?))),
                None => Ok(None),
            }
        })
    }

    /// Get and deserialize a typed value.
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.get_raw(key)? {
            Some(v) => Ok(Some(serde_json::from_value(v)?)),
            None => Ok(None),
        }
    }

    /// Set a typed value (JSON-encoded).
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let encoded = serde_json::to_string(value).map_err(|e| AppError::storage(e.to_string()))?;
        self.set_raw(key, &encoded)
    }

    /// Set a pre-encoded JSON string value.
    pub fn set_raw(&self, key: &str, json_value: &str) -> Result<()> {
        self.set_raw_batch(&[(key, json_value)])
    }

    /// Set multiple pre-encoded JSON string values atomically in a single transaction.
    pub fn set_raw_batch(&self, entries: &[(&str, &str)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }
        self.db.with_conn(|c| {
            let tx = c.unchecked_transaction()?;
            {
                let mut stmt = tx.prepare(
                    "INSERT INTO settings (key, value, updated_at)
                     VALUES (?1, ?2, datetime('now'))
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
                )?;
                for (key, json_value) in entries {
                    stmt.execute(rusqlite::params![key, json_value])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Return all settings as a JSON object.
    pub fn all(&self) -> Result<Value> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare("SELECT key, value FROM settings")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            let mut map = serde_json::Map::new();
            for row in rows {
                let (k, v) = row?;
                map.insert(k, parse_stored_value(&v));
            }
            Ok(Value::Object(map))
        })
    }
}

/// Parse a stored JSON value, falling back to the raw string on corrupt JSON.
///
/// Shared by [`SettingsManager::get_raw`] and [`SettingsManager::all`] so the
/// same row surfaces identically through both readers. A typed
/// [`SettingsManager::get`] still errors via `from_value` on the coerced string.
fn parse_stored_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("Failed to parse setting value, storing as string: {e}");
        Value::String(raw.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_get_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mgr = SettingsManager::new(&db);
        mgr.set("language", &"en".to_string()).unwrap();
        let got: Option<String> = mgr.get("language").unwrap();
        assert_eq!(got, Some("en".to_string()));
    }

    #[test]
    fn seeded_defaults_present() {
        let db = Database::open_in_memory().unwrap();
        let mgr = SettingsManager::new(&db);
        let theme: Option<String> = mgr.get("theme").unwrap();
        assert_eq!(theme, Some("dark".to_string()));
        let auto_hide: Option<u64> = mgr.get("floating_widget_auto_hide_seconds").unwrap();
        assert_eq!(auto_hide, Some(0));
    }

    #[test]
    fn set_raw_batch_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let mgr = SettingsManager::new(&db);
        mgr.set_raw_batch(&[("key_a", "\"val_a\""), ("key_b", "\"val_b\"")])
            .unwrap();
        assert_eq!(
            mgr.get::<String>("key_a").unwrap(),
            Some("val_a".to_string())
        );
        assert_eq!(
            mgr.get::<String>("key_b").unwrap(),
            Some("val_b".to_string())
        );
    }

    #[test]
    fn corrupt_value_reads_back_identically_through_both_readers() {
        let db = Database::open_in_memory().unwrap();
        let mgr = SettingsManager::new(&db);
        mgr.set_raw("broken", "not-json").unwrap();

        let coerced = Value::String("not-json".to_string());
        assert_eq!(mgr.get_raw("broken").unwrap(), Some(coerced.clone()));
        assert_eq!(mgr.all().unwrap()["broken"], coerced);
    }

    #[test]
    fn corrupt_value_fails_typed_read() {
        let db = Database::open_in_memory().unwrap();
        let mgr = SettingsManager::new(&db);
        mgr.set_raw("broken", "not-json").unwrap();

        // The coerced string does not deserialize into the requested type.
        assert!(mgr.get::<u32>("broken").is_err());
    }
}
