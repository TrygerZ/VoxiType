//! Dependency-free regex/heuristic text cleaner.
//!
//! Used as a fallback when no LLM backend is available. Removes filler words,
//! fixes basic punctuation spacing, and capitalizes sentences.

use async_trait::async_trait;

use super::types::{LlmMode, RuleBasedConfig};
use super::LlmFormatter;
use crate::error::Result;

pub struct RuleBasedFormatter {
    filler_words: Vec<String>,
}

impl RuleBasedFormatter {
    pub fn new(config: RuleBasedConfig) -> Self {
        Self {
            filler_words: config.filler_words,
        }
    }

    fn clean(&self, text: &str) -> String {
        // Pre-split each filler into its normalized token sequence so both
        // single-word ("um") and multi-word ("you know") fillers are matched
        // against the tokenized input by a sliding window.
        let filler_seqs: Vec<Vec<String>> = self
            .filler_words
            .iter()
            .map(|w| {
                w.split_whitespace()
                    .map(normalize_token)
                    .filter(|t| !t.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|seq| !seq.is_empty())
            .collect();

        // Original tokens (kept verbatim) paired with their normalized form.
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let normalized: Vec<String> = tokens.iter().map(|t| normalize_token(t)).collect();

        let mut kept: Vec<&str> = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            // Greedily match the longest filler phrase starting at `i`.
            let mut matched_len = 0;
            for seq in &filler_seqs {
                let len = seq.len();
                if len > matched_len
                    && i + len <= normalized.len()
                    && normalized[i..i + len] == seq[..]
                {
                    matched_len = len;
                }
            }
            if matched_len > 0 {
                i += matched_len;
            } else {
                kept.push(tokens[i]);
                i += 1;
            }
        }

        let mut joined = kept.join(" ");
        joined = fix_punctuation_spacing(&joined);
        joined = capitalize_sentences(&joined);
        joined = ensure_terminal_period(&joined);
        joined.trim().to_string()
    }
}

/// Normalize a token for filler matching: strip surrounding non-alphanumerics
/// and lowercase.
fn normalize_token(tok: &str) -> String {
    tok.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Whether punctuation at this position sits inside a single token and must
/// neither be split by a space nor trigger sentence capitalization: decimals
/// and dotted domains (`3.14`, `example.com`), decimal commas and times
/// (`3,5`, `12:30`).
///
/// Deliberate trade-off: any `alnum.alnum` counts as intra-token, so a missing
/// space after a sentence period before a word or digit (`halo.dunia`,
/// `end.2024`) is left unsplit. Whisper output normally includes the space, and
/// splitting the no-space form would corrupt domains and decimals. Preserving
/// those intact is preferred over repairing the rare missing space.
fn is_intra_token_punctuation(prev: Option<char>, c: char, next: Option<char>) -> bool {
    let (Some(p), Some(n)) = (prev, next) else {
        return false;
    };
    match c {
        '.' => p.is_alphanumeric() && n.is_alphanumeric(),
        ',' | ':' => p.is_ascii_digit() && n.is_ascii_digit(),
        _ => false,
    }
}

/// Ensure a single space after sentence punctuation, no space before it.
fn fix_punctuation_spacing(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if matches!(c, ',' | '.' | '!' | '?' | ';' | ':') {
            // Remove preceding space.
            while out.ends_with(' ') {
                out.pop();
            }
            out.push(c);
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            // Punctuation inside a token (decimals, times, domains) keeps the
            // token intact instead of being split by a space.
            if is_intra_token_punctuation(prev, c, next) {
                continue;
            }
            // Ensure following space if next char isn't space/end/punct.
            if let Some(next) = next {
                if next != ' ' && !next.is_ascii_punctuation() {
                    out.push(' ');
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Capitalize the first alphabetic char of each sentence.
fn capitalize_sentences(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = true;
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if capitalize_next && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            capitalize_next = false;
            continue;
        }
        out.push(c);
        let prev = i.checked_sub(1).map(|j| chars[j]);
        let next = chars.get(i + 1).copied();
        // `!` and `?` always end a sentence; `.` ends one only when it is not
        // part of an intra-token token (so "example.com" stays lowercase).
        if matches!(c, '!' | '?') || (c == '.' && !is_intra_token_punctuation(prev, c, next)) {
            capitalize_next = true;
        }
    }
    out
}

fn ensure_terminal_period(text: &str) -> String {
    let trimmed = text.trim_end();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    let last = trimmed.chars().last().unwrap();
    if matches!(last, '.' | '!' | '?') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

#[async_trait]
impl LlmFormatter for RuleBasedFormatter {
    async fn format(&self, text: &str, _mode: &LlmMode, _language: &str) -> Result<String> {
        Ok(self.clean(text))
    }

    async fn translate(&self, text: &str, _source: &str, _target: &str) -> Result<String> {
        tracing::warn!(
            "Rule-based formatter does not support translation. Returning original text."
        );
        Ok(text.to_string())
    }

    fn name(&self) -> &'static str {
        "rule_based"
    }

    fn supports_translation(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt() -> RuleBasedFormatter {
        RuleBasedFormatter::new(RuleBasedConfig::default())
    }

    #[test]
    fn removes_filler_words() {
        let out = fmt().clean("um halo uh saya anu mau makan");
        assert_eq!(out, "Halo saya mau makan.");
    }

    #[test]
    fn capitalizes_and_terminates() {
        let out = fmt().clean("halo dunia");
        assert_eq!(out, "Halo dunia.");
    }

    #[test]
    fn fixes_punctuation_spacing() {
        let out = fmt().clean("halo ,dunia");
        assert_eq!(out, "Halo, dunia.");
    }

    #[test]
    fn removes_multi_word_fillers() {
        // A two-token filler phrase must be dropped as a phrase, not left
        // behind because it isn't a single token. "you know" is no longer a
        // default (it is a legitimate phrase), so the config is supplied.
        let config = RuleBasedConfig {
            filler_words: vec!["you know".to_string()],
        };
        let out = RuleBasedFormatter::new(config).clean("i think you know this works");
        assert_eq!(out, "I think this works.");
    }

    #[test]
    fn default_config_keeps_legitimate_words() {
        // "like" and "you know" were removed from the default filler list
        // because they corrupt legitimate text.
        let out = fmt().clean("it works like a charm you know");
        assert_eq!(out, "It works like a charm you know.");
    }

    #[test]
    fn keeps_decimal_point_intact() {
        assert_eq!(
            fmt().clean("pi is 3.14 approximately"),
            "Pi is 3.14 approximately."
        );
    }

    #[test]
    fn keeps_time_colon_intact() {
        assert_eq!(fmt().clean("meet at 12:30 today"), "Meet at 12:30 today.");
    }

    #[test]
    fn keeps_decimal_comma_intact() {
        assert_eq!(fmt().clean("beratnya 3,5 kg"), "Beratnya 3,5 kg.");
    }

    #[test]
    fn keeps_domain_dot_intact_and_lowercase() {
        assert_eq!(
            fmt().clean("visit example.com now"),
            "Visit example.com now."
        );
    }

    #[test]
    fn normal_comma_spacing_still_regresses() {
        assert_eq!(fmt().clean("halo ,dunia"), "Halo, dunia.");
    }

    #[test]
    fn sentence_dot_still_capitalizes_next_word() {
        assert_eq!(fmt().clean("halo. dunia"), "Halo. Dunia.");
    }
}
