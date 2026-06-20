//! Custom dictionary CRUD + hotword listing.

use serde::{Deserialize, Serialize};

use super::db::Database;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryEntry {
    pub id: String,
    pub word: String,
    #[serde(default)]
    pub pronunciation: Option<String>,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub replacement: Option<String>,
    #[serde(default = "default_lang")]
    pub language: String,
    #[serde(default)]
    pub usage_count: i64,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_category() -> String {
    "custom".to_string()
}
fn default_lang() -> String {
    "id".to_string()
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DictFilter {
    pub category: Option<String>,
    pub language: Option<String>,
}

pub struct DictionaryRepository<'a> {
    db: &'a Database,
}

impl<'a> DictionaryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn upsert(&self, e: &DictionaryEntry) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO dictionary_entries
                   (id, word, pronunciation, category, replacement, language, usage_count, is_active)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                 ON CONFLICT(word, language) DO UPDATE SET
                   pronunciation=excluded.pronunciation,
                   category=excluded.category,
                   replacement=excluded.replacement,
                   is_active=excluded.is_active,
                   updated_at=datetime('now')",
                rusqlite::params![
                    e.id,
                    e.word,
                    e.pronunciation,
                    e.category,
                    e.replacement,
                    e.language,
                    e.usage_count,
                    e.is_active as i32,
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_all(&self, filter: &DictFilter) -> Result<Vec<DictionaryEntry>> {
        self.db.with_conn(|c| {
            let mut sql = String::from(
                "SELECT id, word, pronunciation, category, replacement, language, usage_count, is_active
                 FROM dictionary_entries WHERE 1=1",
            );
            if filter.category.is_some() {
                sql.push_str(" AND category = :cat");
            }
            if filter.language.is_some() {
                sql.push_str(" AND language = :lang");
            }
            sql.push_str(" ORDER BY word COLLATE NOCASE ASC");

            let mut stmt = c.prepare(&sql)?;
            let cat = filter.category.clone().unwrap_or_default();
            let lang = filter.language.clone().unwrap_or_default();
            let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
            if filter.category.is_some() {
                params.push((":cat", &cat));
            }
            if filter.language.is_some() {
                params.push((":lang", &lang));
            }
            let rows = stmt.query_map(params.as_slice(), row_to_entry)?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM dictionary_entries WHERE id = ?1", [id])?;
            Ok(())
        })
    }

    /// Active words, used to bias STT (hotword prompt).
    pub fn get_hotwords(&self) -> Result<Vec<String>> {
        self.db.with_conn(|c| {
            let mut stmt =
                c.prepare("SELECT word FROM dictionary_entries WHERE is_active = 1")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    pub fn increment_usage(&self, id: &str) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "UPDATE dictionary_entries SET usage_count = usage_count + 1 WHERE id = ?1",
                [id],
            )?;
            Ok(())
        })
    }
}

fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<DictionaryEntry> {
    Ok(DictionaryEntry {
        id: r.get(0)?,
        word: r.get(1)?,
        pronunciation: r.get(2)?,
        category: r.get(3)?,
        replacement: r.get(4)?,
        language: r.get(5)?,
        usage_count: r.get(6)?,
        is_active: r.get::<_, i32>(7)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_and_hotwords() {
        let db = Database::open_in_memory().unwrap();
        let repo = DictionaryRepository::new(&db);
        repo.upsert(&DictionaryEntry {
            id: "1".into(),
            word: "VoxiType".into(),
            pronunciation: None,
            category: "custom".into(),
            replacement: None,
            language: "id".into(),
            usage_count: 0,
            is_active: true,
        })
        .unwrap();

        let hot = repo.get_hotwords().unwrap();
        assert_eq!(hot, vec!["VoxiType".to_string()]);

        assert_eq!(repo.get_all(&DictFilter::default()).unwrap().len(), 1);
        repo.delete("1").unwrap();
        assert!(repo.get_all(&DictFilter::default()).unwrap().is_empty());
    }
}
