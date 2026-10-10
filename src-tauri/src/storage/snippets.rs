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

/// Expand snippet trigger phrases in `text` in a single left-to-right pass.
///
/// At each source position the longest matching trigger is expanded; the scan
/// then continues *after* the inserted content, so text produced by one
/// expansion is never re-scanned by another (a trigger phrase occurring inside
/// an earlier expansion's content does not expand). Matching is case-insensitive
/// and bounded by non-alphanumeric characters, identical to dictionary
/// replacements. Empty triggers are ignored and unmatched text is unchanged.
pub fn expand_snippets(text: &str, expansions: &[(String, String)]) -> String {
    // Longest triggers first so a multi-word phrase wins over its own prefix.
    let mut sorted: Vec<&(String, String)> = expansions
        .iter()
        .filter(|(trigger, _)| !trigger.is_empty())
        .collect();
    sorted.sort_by_key(|(trigger, _)| std::cmp::Reverse(trigger.len()));
    let triggers: Vec<(String, &str)> = sorted
        .iter()
        .map(|(trigger, content)| (trigger.to_lowercase(), content.as_str()))
        .collect();

    let matcher = CaseInsensitiveMatcher::new(text);
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0usize;
    while cursor < text.len() {
        if let Some((match_len, content)) = matcher.longest_match_at(cursor, &triggers) {
            result.push_str(content);
            cursor += match_len;
        } else if let Some(ch) = text[cursor..].chars().next() {
            result.push(ch);
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    result
}

/// Case-insensitive view of a haystack that maps byte offsets between the
/// original string and its lowercased form. Lowercasing can change byte lengths
/// (e.g. 'İ' U+0130 -> "i̇", 2 -> 3 bytes), so offsets found in the lowercased
/// text cannot be used to slice the original directly; both directions are
/// recorded to stay panic-free.
struct CaseInsensitiveMatcher<'a> {
    original: &'a str,
    lower: String,
    /// Original byte offset for every byte offset in `lower`, plus an
    /// end-of-string sentinel.
    lower_to_orig: Vec<usize>,
    /// Lowercased byte offset for every original byte offset.
    orig_to_lower: Vec<usize>,
}

impl<'a> CaseInsensitiveMatcher<'a> {
    fn new(original: &'a str) -> Self {
        let mut lower = String::with_capacity(original.len());
        let mut lower_to_orig = Vec::with_capacity(original.len() + 1);
        let mut orig_to_lower = vec![0usize; original.len() + 1];
        for (orig_off, ch) in original.char_indices() {
            orig_to_lower[orig_off] = lower.len();
            for lc in ch.to_lowercase() {
                for _ in 0..lc.len_utf8() {
                    lower_to_orig.push(orig_off);
                }
                lower.push(lc);
            }
        }
        orig_to_lower[original.len()] = lower.len();
        lower_to_orig.push(original.len()); // sentinel for end-of-string
        Self {
            original,
            lower,
            lower_to_orig,
            orig_to_lower,
        }
    }

    /// Longest trigger matching at original byte offset `orig_start` (a char
    /// boundary). Triggers must be ordered longest-first; the first match wins.
    /// Returns `(match length in the original, content)`.
    fn longest_match_at<'t>(
        &self,
        orig_start: usize,
        triggers: &[(String, &'t str)],
    ) -> Option<(usize, &'t str)> {
        let lower_start = *self.orig_to_lower.get(orig_start)?;
        let before_ok = orig_start == 0
            || !self.original[..orig_start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric());

        for (trigger_lower, content) in triggers {
            if !self.lower[lower_start..].starts_with(trigger_lower.as_str()) {
                continue;
            }
            let orig_end = self.lower_to_orig[lower_start + trigger_lower.len()];
            let after_ok = orig_end == self.original.len()
                || !self.original[orig_end..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphanumeric());
            if before_ok && after_ok {
                return Some((orig_end - orig_start, content));
            }
        }
        None
    }
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

    #[test]
    fn inserted_content_is_not_re_expanded() {
        // Trigger "alpha" expands to content containing another trigger
        // ("beta"). The inserted "beta" must NOT be expanded by the "beta"
        // snippet: expansion is a single pass over the source text.
        let exp = vec![
            ("alpha".to_string(), "beta beta".to_string()),
            ("beta".to_string(), "ZZZ".to_string()),
        ];
        assert_eq!(expand_snippets("alpha", &exp), "beta beta");
    }

    #[test]
    fn two_source_triggers_both_expand_left_to_right() {
        let exp = vec![
            ("foo".to_string(), "1".to_string()),
            ("bar".to_string(), "2".to_string()),
        ];
        assert_eq!(expand_snippets("foo and bar", &exp), "1 and 2");
    }

    #[test]
    fn expansion_matching_is_case_insensitive_and_bounded() {
        let exp = vec![("sign off".to_string(), "Y".to_string())];
        assert_eq!(expand_snippets("Sign Off", &exp), "Y");
        // Bounded: not matched inside a larger alphanumeric token.
        assert_eq!(expand_snippets("signoff", &exp), "signoff");
    }

    #[test]
    fn duplicate_trigger_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let repo = SnippetRepository::new(&db);
        let first = Snippet {
            id: "1".into(),
            name: "Sign off".into(),
            trigger_phrase: "sign off".into(),
            content: "Best regards".into(),
            category: None,
            mode: None,
            usage_count: 0,
            is_active: true,
        };
        let second = Snippet {
            id: "2".into(),
            content: "Thanks".into(),
            ..first.clone()
        };

        repo.upsert(&first).unwrap();
        assert!(repo.upsert(&second).is_err());
        assert_eq!(repo.get_all().unwrap().len(), 1);
    }
}
