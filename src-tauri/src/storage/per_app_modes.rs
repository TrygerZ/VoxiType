//! Per-application mode mapping.
//!
//! Maps a foreground process name (e.g. "code", "chrome") to a formatting mode
//! id ("dictation" | "message" | "email" | custom). Used to auto-switch the
//! active mode based on the focused app.

use serde::{Deserialize, Serialize};

use super::db::Database;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerAppMode {
    #[serde(default)]
    pub id: i64,
    /// Lowercased process name without extension, e.g. "code".
    pub app_process_name: String,
    #[serde(default)]
    pub app_display_name: Option<String>,
    /// Mode id stored in settings form ("dictation", "message", "email", ...).
    pub mode_id: String,
}

pub struct PerAppModeRepository<'a> {
    db: &'a Database,
}

impl<'a> PerAppModeRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, m: &PerAppMode) -> Result<()> {
        let normalized = crate::active_window::normalize_process_name(&m.app_process_name);
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO per_app_modes (app_process_name, app_display_name, mode_id)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(app_process_name) DO UPDATE SET
                   app_display_name = excluded.app_display_name,
                   mode_id = excluded.mode_id",
                rusqlite::params![normalized, m.app_display_name, m.mode_id],
            )?;
            Ok(())
        })
    }

    pub fn get_all(&self) -> Result<Vec<PerAppMode>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, app_process_name, app_display_name, mode_id
                 FROM per_app_modes ORDER BY app_process_name ASC",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(PerAppMode {
                    id: r.get(0)?,
                    app_process_name: r.get(1)?,
                    app_display_name: r.get(2)?,
                    mode_id: r.get(3)?,
                })
            })?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    /// Look up the mode mapped to a process name, if any. The name is
    /// normalized to match the canonical form stored by [`Self::upsert`].
    pub fn mode_for(&self, process_name: &str) -> Result<Option<String>> {
        let normalized = crate::active_window::normalize_process_name(process_name);
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT mode_id FROM per_app_modes
                 WHERE app_process_name = ?1 AND is_active = 1",
            )?;
            let mut rows = stmt.query_map([&normalized], |r| r.get::<_, String>(0))?;
            match rows.next() {
                Some(v) => Ok(Some(v?)),
                None => Ok(None),
            }
        })
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM per_app_modes WHERE id = ?1", [id])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_lookup_delete() {
        let db = Database::open_in_memory().unwrap();
        let repo = PerAppModeRepository::new(&db);
        repo.upsert(&PerAppMode {
            id: 0,
            app_process_name: "code".into(),
            app_display_name: Some("VS Code".into()),
            mode_id: "message".into(),
        })
        .unwrap();

        assert_eq!(repo.mode_for("code").unwrap(), Some("message".to_string()));
        assert_eq!(repo.mode_for("unknown").unwrap(), None);

        let all = repo.get_all().unwrap();
        assert_eq!(all.len(), 1);
        repo.delete(all[0].id).unwrap();
        assert!(repo.get_all().unwrap().is_empty());
    }

    #[test]
    fn lookup_is_case_and_extension_insensitive() {
        let db = Database::open_in_memory().unwrap();
        let repo = PerAppModeRepository::new(&db);
        // Store using a mixed-case name with an uppercase extension.
        repo.upsert(&PerAppMode {
            id: 0,
            app_process_name: "Code.EXE".into(),
            app_display_name: None,
            mode_id: "email".into(),
        })
        .unwrap();

        // Lookups by any casing / with-or-without extension resolve the same row.
        assert_eq!(repo.mode_for("code").unwrap(), Some("email".to_string()));
        assert_eq!(
            repo.mode_for("Code.exe").unwrap(),
            Some("email".to_string())
        );
        assert_eq!(
            repo.mode_for("CODE.EXE").unwrap(),
            Some("email".to_string())
        );
        // A second upsert with different casing updates in place (no duplicate).
        repo.upsert(&PerAppMode {
            id: 0,
            app_process_name: "CODE".into(),
            app_display_name: None,
            mode_id: "message".into(),
        })
        .unwrap();
        assert_eq!(repo.get_all().unwrap().len(), 1);
        assert_eq!(repo.mode_for("code").unwrap(), Some("message".to_string()));
    }
}
