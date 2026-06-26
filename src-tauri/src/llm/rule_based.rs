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
        let lowered_fillers: Vec<String> =
            self.filler_words.iter().map(|w| w.to_lowercase()).collect();

        // Tokenize on whitespace, drop filler words (word-boundary, case-insensitive).
        let kept: Vec<&str> = text
            .split_whitespace()
            .filter(|tok| {
                let stripped = tok
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                !lowered_fillers.contains(&stripped)
            })
            .collect();

        let mut joined = kept.join(" ");
        joined = fix_punctuation_spacing(&joined);
        joined = capitalize_sentences(&joined);
        joined = ensure_terminal_period(&joined);
        joined.trim().to_string()
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
            // Ensure following space if next char isn't space/end/punct.
            if let Some(&next) = chars.get(i + 1) {
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
    for c in text.chars() {
        if capitalize_next && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            out.push(c);
            if matches!(c, '.' | '!' | '?') {
                capitalize_next = true;
            }
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
    async fn format(&self, text: &str, _mode: &LlmMode) -> Result<String> {
        Ok(self.clean(text))
    }

    async fn translate(&self, _text: &str, _source: &str, _target: &str) -> Result<String> {
        Err(crate::error::AppError::llm(
            "Translation requires a cloud LLM (Groq) or local Ollama. \
             Rule-based formatter supports text cleanup only.",
        ))
    }

    fn name(&self) -> &'static str {
        "rule_based"
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
}
