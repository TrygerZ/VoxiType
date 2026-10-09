//! Tauri command handlers (IPC surface).
//!
//! Sub-modules own their command groups; `runtime` holds shared config
//! builders and the core recording pipeline (used by both commands and
//! the global hotkey callbacks).

mod dictionary;
mod file_transcription;
mod history;
mod misc;
mod per_app;
mod recording;
pub mod runtime;
mod settings;
mod snippets;
mod stats;

pub use dictionary::*;
pub use file_transcription::*;
pub use history::*;
pub use misc::*;
pub use per_app::*;
pub use recording::*;
pub use settings::*;
pub use snippets::*;
pub use stats::*;

use crate::error::AppError;

/// Max length of an identifier-like IPC field (language, category, mode,
/// pronunciation, app process name, ...). Bounds what a compromised webview
/// can persist into SQLite or render into export documents.
pub const MAX_IDENTIFIER_LEN: usize = 4 * 1024;
/// Max length of a free-text blob crossing the IPC boundary (exported item
/// text, dictionary replacement, ...).
pub const MAX_TEXT_BLOB_LEN: usize = 1024 * 1024;

/// Reject a string field that exceeds `max` characters. Shared by every
/// command boundary so a single helper covers the whole IPC surface.
pub fn check_len(field: &str, value: &str, max: usize) -> std::result::Result<(), AppError> {
    if value.chars().count() > max {
        return Err(AppError::invalid_input(format!(
            "Field '{field}' exceeds {max} characters"
        )));
    }
    Ok(())
}

/// Reject an empty or whitespace-only string field.
pub fn check_non_empty(field: &str, value: &str) -> std::result::Result<(), AppError> {
    if value.trim().is_empty() {
        return Err(AppError::invalid_input(format!(
            "Field '{field}' must not be empty"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_len_accepts_at_limit_and_rejects_one_over() {
        let at_limit = "x".repeat(10);
        assert!(check_len("field", &at_limit, 10).is_ok());
        let over = "x".repeat(11);
        let err = check_len("field", &over, 10).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
    }

    #[test]
    fn check_len_counts_characters_not_bytes() {
        // Ten multi-byte characters are within a 10-char cap.
        let ten_chars = "é".repeat(10);
        assert!(check_len("field", &ten_chars, 10).is_ok());
    }

    #[test]
    fn check_non_empty_rejects_blank() {
        assert!(check_non_empty("field", "ok").is_ok());
        assert_eq!(
            check_non_empty("field", "  ").unwrap_err().code,
            crate::error::ErrorCode::InvalidInput
        );
    }
}
