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
                Some(s) => {
                    let raw = s?;
                    let v: Value = serde_json::from_str(&raw)?;
                    Ok(Some(v))
                }
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
        let encoded =
            serde_json::to_string(value).map_err(|e| AppError::storage(e.to_string()))?;
        self.set_raw(key, &encoded)
    }

    /// Set a pre-encoded JSON string value.
    pub fn set_raw(&self, key: &str, json_value: &str) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO settings (key, value, updated_at)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
                rusqlite::params![key, json_value],
            )?;
            Ok(())
        })
    }

    /// Return all settings as a JSON object.
    pub fn all(&self) -> Result<Value> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare("SELECT key, value FROM settings")?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut map = serde_json::Map::new();
            for row in rows {
                let (k, v) = row?;
                let parsed: Value = serde_json::from_str(&v).unwrap_or(Value::String(v));
                map.insert(k, parsed);
            }
            Ok(Value::Object(map))
        })
    }
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
    }
}
