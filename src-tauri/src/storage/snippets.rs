//! Snippet (voice shortcut) CRUD + trigger-phrase expansion.
//!
//! A snippet maps a spoken trigger phrase (e.g. "sign off") to a longer block
//! of text (e.g. an email signature). Expansion is applied to the transcribed
//! text before injection.

use serde::{Deserialize, Serialize};

use super::db::Database;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub trigger_phrase: String,
    pub content: String,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub usage_count: i64,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

pub struct SnippetRepository<'a> {
    db: &'a Database,
}

impl<'a> SnippetRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, s: &Snippet) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO snippets
                   (id, name, trigger_phrase, content, category, mode, usage_count, is_active)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(id) DO UPDATE SET
                   name=excluded.name,
                   trigger_phrase=excluded.trigger_phrase,
                   content=excluded.content,
                   category=excluded.category,
                   mode=excluded.mode,
                   is_active=excluded.is_active,
                   updated_at=datetime('now')",
                rusqlite::params![
                    s.id,
                    s.name,
                    s.trigger_phrase,
                    s.content,
                    s.category,
                    s.mode,
                    s.usage_count,
                    s.is_active as i32,
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_all(&self) -> Result<Vec<Snippet>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, name, trigger_phrase, content, category, mode, usage_count, is_active
                 FROM snippets ORDER BY trigger_phrase COLLATE NOCASE ASC",
            )?;
            let rows = stmt.query_map([], row_to_snippet)?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    /// Active `(trigger_phrase, content)` pairs used for expansion.
    pub fn get_active_expansions(&self) -> Result<Vec<(String, String)>> {
        self.db.with_conn(|c| {
            let mut stmt =
                c.prepare("SELECT trigger_phrase, content FROM snippets WHERE is_active = 1")?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM snippets WHERE id = ?1", [id])?;
            Ok(())
        })
    }
}

fn row_to_snippet(r: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: r.get(0)?,
        name: r.get(1)?,
        trigger_phrase: r.get(2)?,
        content: r.get(3)?,
        category: r.get(4)?,
        mode: r.get(5)?,
        usage_count: r.get(6)?,
        is_active: r.get::<_, i32>(7)? != 0,
    })
}

/// Expand snippet trigger phrases in `text`. Matching is case-insensitive and
/// bounded by non-alphanumeric characters (same rules as dictionary
/// replacements). Longer triggers are applied first so multi-word phrases win.
pub fn expand_snippets(text: &str, expansions: &[(String, String)]) -> String {
    let mut sorted: Vec<&(String, String)> = expansions.iter().collect();
    sorted.sort_by_key(|(trigger, _)| std::cmp::Reverse(trigger.len()));

    let mut out = text.to_string();
    for (trigger, content) in sorted {
        if trigger.is_empty() {
            continue;
        }
        out = super::dictionary::apply_replacements(&out, &[(trigger.clone(), content.clone())]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_get_delete_and_expand() {
        let db = Database::open_in_memory().unwrap();
        let repo = SnippetRepository::new(&db);
        repo.upsert(&Snippet {
            id: "1".into(),
            name: "Sign off".into(),
            trigger_phrase: "sign off".into(),
            content: "Best regards, Tryger".into(),
            category: None,
            mode: None,
            usage_count: 0,
            is_active: true,
        })
        .unwrap();

        assert_eq!(repo.get_all().unwrap().len(), 1);

        let exp = repo.get_active_expansions().unwrap();
        let out = expand_snippets("please sign off here", &exp);
        assert_eq!(out, "please Best regards, Tryger here");

        repo.delete("1").unwrap();
        assert!(repo.get_all().unwrap().is_empty());
    }

    #[test]
    fn longer_triggers_win() {
        let exp = vec![
            ("sign".to_string(), "X".to_string()),
            ("sign off".to_string(), "Y".to_string()),
        ];
        // "sign off" should expand to Y, not "X off".
        assert_eq!(expand_snippets("sign off", &exp), "Y");
    }
}
