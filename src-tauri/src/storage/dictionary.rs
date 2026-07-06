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
            let mut stmt = c.prepare("SELECT word FROM dictionary_entries WHERE is_active = 1")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    /// Active entries that define a non-empty replacement, as `(word, replacement)`.
    pub fn get_replacements(&self) -> Result<Vec<(String, String)>> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT word, replacement FROM dictionary_entries
                 WHERE is_active = 1 AND replacement IS NOT NULL AND replacement <> ''",
            )?;
            let rows =
                stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
            Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
        })
    }

    /// Toggle the active flag for an entry.
    pub fn set_active(&self, id: &str, active: bool) -> Result<()> {
        self.db.with_conn(|c| {
            c.execute(
                "UPDATE dictionary_entries SET is_active = ?2, updated_at = datetime('now')
                 WHERE id = ?1",
                rusqlite::params![id, active as i32],
            )?;
            Ok(())
        })
    }
}

/// Apply `(word -> replacement)` substitutions to `text`.
///
/// Matching is case-insensitive and bounded by non-alphanumeric characters so
/// "JS" replaces the standalone token but not the "js" inside "jsx".
pub fn apply_replacements(text: &str, replacements: &[(String, String)]) -> String {
    let mut out = text.to_string();
    for (word, replacement) in replacements {
        if word.is_empty() {
            continue;
        }
        out = replace_word_ci(&out, word, replacement);
    }
    out
}

fn replace_word_ci(haystack: &str, needle: &str, replacement: &str) -> String {
    let needle_lower = needle.to_lowercase();
    if needle_lower.is_empty() {
        return haystack.to_string();
    }

    // Lowercase the haystack while recording, for every byte offset in the
    // lowercased string, the corresponding byte offset in the original. We
    // search in the lowercased text but slice the original, so this mapping is
    // required to stay panic-free when lowercasing changes byte lengths (e.g.
    // 'İ' -> "i̇" is 2 -> 3 bytes). Without it, offsets from the lowercased
    // string can land out of bounds or mid-UTF-8-char in the original.
    let mut hay_lower = String::with_capacity(haystack.len());
    let mut offsets: Vec<usize> = Vec::with_capacity(haystack.len() + 1);
    for (orig_off, ch) in haystack.char_indices() {
        for lc in ch.to_lowercase() {
            for _ in 0..lc.len_utf8() {
                offsets.push(orig_off);
            }
            hay_lower.push(lc);
        }
    }
    offsets.push(haystack.len()); // sentinel for end-of-string

    let mut result = String::with_capacity(haystack.len());
    let mut cursor = 0usize; // byte offset into `hay_lower`

    while let Some(rel) = hay_lower[cursor..].find(&needle_lower) {
        let lstart = cursor + rel;
        let lend = lstart + needle_lower.len();
        let ostart = offsets[lstart];
        let oend = offsets[lend];

        let before_ok = ostart == 0
            || !haystack[..ostart]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric());
        let after_ok = oend == haystack.len()
            || !haystack[oend..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric());

        result.push_str(&haystack[offsets[cursor]..ostart]);
        if before_ok && after_ok {
            result.push_str(replacement);
        } else {
            result.push_str(&haystack[ostart..oend]);
        }
        cursor = lend;
    }
    result.push_str(&haystack[offsets[cursor]..]);
    result
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

    #[test]
    fn replacements_are_word_bounded_and_case_insensitive() {
        let reps = vec![
            ("js".to_string(), "JavaScript".to_string()),
            ("voxitype".to_string(), "VoxiType".to_string()),
        ];
        // "js" replaced, but "jsx" left intact.
        let out = apply_replacements("i love js and jsx in voxitype", &reps);
        assert_eq!(out, "i love JavaScript and jsx in VoxiType");
    }

    #[test]
    fn replacements_handle_punctuation_boundaries() {
        let reps = vec![("api".to_string(), "API".to_string())];
        assert_eq!(
            apply_replacements("the api, please", &reps),
            "the API, please"
        );
    }

    #[test]
    fn replacements_survive_length_changing_lowercase() {
        // 'İ' (U+0130) lowercases to "i̇" (2 bytes -> 3 bytes). A naive
        // implementation that derives offsets from the lowercased string but
        // slices the original panics here. Must not panic and must still
        // replace the surrounding word.
        let reps = vec![("hello".to_string(), "hi".to_string())];
        let out = apply_replacements("İ hello world", &reps);
        assert_eq!(out, "İ hi world");
    }

    #[test]
    fn replacement_and_active_toggle() {
        let db = Database::open_in_memory().unwrap();
        let repo = DictionaryRepository::new(&db);
        repo.upsert(&DictionaryEntry {
            id: "1".into(),
            word: "js".into(),
            pronunciation: None,
            category: "custom".into(),
            replacement: Some("JavaScript".into()),
            language: "id".into(),
            usage_count: 0,
            is_active: true,
        })
        .unwrap();
        assert_eq!(repo.get_replacements().unwrap().len(), 1);
        repo.set_active("1", false).unwrap();
        assert!(repo.get_replacements().unwrap().is_empty());
        assert!(repo.get_hotwords().unwrap().is_empty());
    }
}
