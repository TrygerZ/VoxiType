//! Transcription history CRUD + full-text search.

use serde::{Deserialize, Serialize};

use super::db::Database;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionEntry {
    pub id: String,
    #[serde(default)]
    pub created_at: String,
    pub text_raw: String,
    pub text_formatted: String,
    #[serde(default = "default_lang")]
    pub source_lang: String,
    #[serde(default)]
    pub target_lang: Option<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub stt_engine: String,
    #[serde(default)]
    pub stt_confidence: Option<f32>,
    #[serde(default)]
    pub llm_engine: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<i64>,
    #[serde(default)]
    pub word_count: i64,
    #[serde(default)]
    pub character_count: i64,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub app_context: Option<String>,
}

fn default_lang() -> String {
    "id".to_string()
}
fn default_mode() -> String {
    "dictation".to_string()
}

/// Filter for listing history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryFilter {
    pub mode: Option<String>,
    pub source_lang: Option<String>,
    pub pinned_only: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Lifetime usage totals aggregated over the whole transcriptions table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryTotals {
    pub total_sessions: i64,
    pub total_words: i64,
    pub total_duration_ms: i64,
}

pub struct HistoryRepository<'a> {
    db: &'a Database,
}

impl<'a> HistoryRepository<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn insert(&self, e: &TranscriptionEntry) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO transcriptions
                 (id, text_raw, text_formatted, source_lang, target_lang, mode,
                  stt_engine, stt_confidence, llm_engine, duration_ms,
                  word_count, character_count, is_pinned, app_context)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
                rusqlite::params![
                    e.id,
                    e.text_raw,
                    e.text_formatted,
                    e.source_lang,
                    e.target_lang,
                    e.mode,
                    e.stt_engine,
                    e.stt_confidence,
                    e.llm_engine,
                    e.duration_ms,
                    e.word_count,
                    e.character_count,
                    e.is_pinned as i32,
                    e.app_context,
                ],
            )?;
            Ok(())
        })
    }

    pub fn list(&self, filter: &HistoryFilter) -> Result<Vec<TranscriptionEntry>> {
        self.db.with_conn(|c| {
            let mut sql = String::from(
                "SELECT id, created_at, text_raw, text_formatted, source_lang, target_lang,
                        mode, stt_engine, stt_confidence, llm_engine, duration_ms,
                        word_count, character_count, is_pinned, app_context
                 FROM transcriptions WHERE 1=1",
            );
            if filter.mode.is_some() {
                sql.push_str(" AND mode = :mode");
            }
            if filter.source_lang.is_some() {
                sql.push_str(" AND source_lang = :lang");
            }
            if filter.pinned_only == Some(true) {
                sql.push_str(" AND is_pinned = 1");
            }
            sql.push_str(" ORDER BY created_at DESC");
            let limit = filter.limit.unwrap_or(100);
            let offset = filter.offset.unwrap_or(0);
            sql.push_str(&format!(" LIMIT {limit} OFFSET {offset}"));

            let mut stmt = c.prepare(&sql)?;
            let mode = filter.mode.clone().unwrap_or_default();
            let lang = filter.source_lang.clone().unwrap_or_default();
            let mut params: Vec<(&str, &dyn rusqlite::ToSql)> = Vec::new();
            if filter.mode.is_some() {
                params.push((":mode", &mode));
            }
            if filter.source_lang.is_some() {
                params.push((":lang", &lang));
            }
            let rows = stmt.query_map(params.as_slice(), row_to_entry)?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    /// Lifetime totals aggregated over the whole table — independent of the
    /// list `LIMIT` used for the paginated history view. Powers the usage
    /// dashboard so totals never silently cap or shrink as old rows scroll
    /// past the 100-row list window.
    pub fn totals(&self) -> Result<HistoryTotals> {
        self.db.with_conn(|c| {
            let row = c.query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(word_count), 0),
                        COALESCE(SUM(duration_ms), 0)
                 FROM transcriptions",
                [],
                |r| {
                    Ok(HistoryTotals {
                        total_sessions: r.get(0)?,
                        total_words: r.get(1)?,
                        total_duration_ms: r.get(2)?,
                    })
                },
            )?;
            Ok(row)
        })
    }

    pub fn search(&self, query: &str) -> Result<Vec<TranscriptionEntry>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT t.id, t.created_at, t.text_raw, t.text_formatted, t.source_lang,
                        t.target_lang, t.mode, t.stt_engine, t.stt_confidence, t.llm_engine,
                        t.duration_ms, t.word_count, t.character_count, t.is_pinned, t.app_context
                 FROM transcriptions t
                 JOIN transcriptions_fts f ON f.rowid = t.rowid
                 WHERE transcriptions_fts MATCH ?1
                 ORDER BY t.created_at DESC LIMIT 100",
            )?;
            let safe = sanitize_fts5(query);
            let rows = stmt.query_map([&safe], row_to_entry)?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<TranscriptionEntry>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, text_raw, text_formatted, source_lang, target_lang,
                        mode, stt_engine, stt_confidence, llm_engine, duration_ms,
                        word_count, character_count, is_pinned, app_context
                 FROM transcriptions WHERE id = ?1",
            )?;
            let mut rows = stmt.query_map([id], row_to_entry)?;
            match rows.next() {
                Some(r) => Ok(Some(r?)),
                None => Ok(None),
            }
        })
    }

    pub fn set_pinned(&self, id: &str, pinned: bool) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "UPDATE transcriptions SET is_pinned = ?1 WHERE id = ?2",
                rusqlite::params![pinned as i32, id],
            )?;
            Ok(())
        })
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM transcriptions WHERE id = ?1", [id])?;
            Ok(())
        })
    }

    /// Delete all history entries. When `keep_pinned` is true, pinned entries
    /// are preserved. Returns the number of rows removed.
    pub fn clear(&self, keep_pinned: bool) -> Result<usize> {
        self.db.with_conn(|c| {
            let n = if keep_pinned {
                c.execute("DELETE FROM transcriptions WHERE is_pinned = 0", [])?
            } else {
                c.execute("DELETE FROM transcriptions", [])?
            };
            Ok(n)
        })
    }
}

/// Escape FTS5 special characters and operators from user input so queries
/// containing AND/OR/NOT/NEAR/*/etc. are treated as literal text.
/// Escape FTS5 special characters from user input and append `*` (prefix
/// wildcard) to each word so that partial input matches completed words.
///
/// `"menye"` → `"menye"*` which matches `menyesuaikan`, `menyebutkan`, etc.
fn sanitize_fts5(query: &str) -> String {
    // Characters that carry special meaning in FTS5 syntax.
    const SPECIAL: &[char] = &['"', '*', '^', '(', ')', ':', '~', ','];
    let cleaned: String = query.chars().filter(|c| !SPECIAL.contains(c)).collect();
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    if words.is_empty() {
        "\"\"".to_string()
    } else {
        words
            .iter()
            .map(|w| format!("\"{}\"*", w.replace('"', "\"\"")))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

fn row_to_entry(r: &rusqlite::Row) -> rusqlite::Result<TranscriptionEntry> {
    Ok(TranscriptionEntry {
        id: r.get(0)?,
        created_at: r.get(1)?,
        text_raw: r.get(2)?,
        text_formatted: r.get(3)?,
        source_lang: r.get(4)?,
        target_lang: r.get(5)?,
        mode: r.get(6)?,
        stt_engine: r.get(7)?,
        stt_confidence: r.get(8)?,
        llm_engine: r.get(9)?,
        duration_ms: r.get(10)?,
        word_count: r.get(11)?,
        character_count: r.get(12)?,
        is_pinned: r.get::<_, i32>(13)? != 0,
        app_context: r.get(14)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, text: &str) -> TranscriptionEntry {
        TranscriptionEntry {
            id: id.to_string(),
            created_at: String::new(),
            text_raw: text.to_string(),
            text_formatted: text.to_string(),
            source_lang: "id".to_string(),
            target_lang: None,
            mode: "dictation".to_string(),
            stt_engine: "groq".to_string(),
            stt_confidence: Some(0.9),
            llm_engine: Some("rule_based".to_string()),
            duration_ms: Some(1000),
            word_count: text.split_whitespace().count() as i64,
            character_count: text.len() as i64,
            is_pinned: false,
            app_context: None,
        }
    }

    #[test]
    fn insert_list_search_delete() {
        let db = Database::open_in_memory().unwrap();
        let repo = HistoryRepository::new(&db);
        repo.insert(&sample("a", "halo dunia voxitype")).unwrap();
        repo.insert(&sample("b", "selamat pagi")).unwrap();

        let all = repo.list(&HistoryFilter::default()).unwrap();
        assert_eq!(all.len(), 2);

        let found = repo.search("voxitype").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "a");

        // Multi-word search (non-consecutive) should find the matching entry
        let found_multi = repo.search("halo voxitype").unwrap();
        assert_eq!(found_multi.len(), 1);
        assert_eq!(found_multi[0].id, "a");

        // Prefix search — partial word matches completed words
        let found_prefix = repo.search("voxi").unwrap();
        assert_eq!(found_prefix.len(), 1);
        assert_eq!(found_prefix[0].id, "a");

        let found_prefix = repo.search("hal").unwrap();
        assert_eq!(found_prefix.len(), 1);
        assert_eq!(found_prefix[0].id, "a");

        repo.set_pinned("a", true).unwrap();
        let pinned = repo
            .list(&HistoryFilter {
                pinned_only: Some(true),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(pinned.len(), 1);

        repo.delete("b").unwrap();
        assert_eq!(repo.list(&HistoryFilter::default()).unwrap().len(), 1);
    }

    #[test]
    fn totals_aggregate_over_full_table() {
        let db = Database::open_in_memory().unwrap();
        let repo = HistoryRepository::new(&db);

        // Empty table → all zeros (COALESCE guards the NULL SUM).
        let empty = repo.totals().unwrap();
        assert_eq!(empty.total_sessions, 0);
        assert_eq!(empty.total_words, 0);
        assert_eq!(empty.total_duration_ms, 0);

        // sample() has word_count = split_whitespace().count(), duration_ms = 1000.
        repo.insert(&sample("a", "satu dua tiga")).unwrap(); // 3 words
        repo.insert(&sample("b", "empat lima")).unwrap(); // 2 words

        let t = repo.totals().unwrap();
        assert_eq!(t.total_sessions, 2);
        assert_eq!(t.total_words, 5);
        assert_eq!(t.total_duration_ms, 2000);
    }

    #[test]
    fn clear_respects_pinned() {
        let db = Database::open_in_memory().unwrap();
        let repo = HistoryRepository::new(&db);
        repo.insert(&sample("a", "satu")).unwrap();
        repo.insert(&sample("b", "dua")).unwrap();
        repo.insert(&sample("c", "tiga")).unwrap();
        repo.set_pinned("b", true).unwrap();

        // keep_pinned=true removes only unpinned entries.
        let removed = repo.clear(true).unwrap();
        assert_eq!(removed, 2);
        let remaining = repo.list(&HistoryFilter::default()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, "b");

        // FTS stays in sync after the bulk delete.
        assert_eq!(repo.search("satu").unwrap().len(), 0);

        // keep_pinned=false wipes everything.
        let removed = repo.clear(false).unwrap();
        assert_eq!(removed, 1);
        assert_eq!(repo.list(&HistoryFilter::default()).unwrap().len(), 0);
    }
}
