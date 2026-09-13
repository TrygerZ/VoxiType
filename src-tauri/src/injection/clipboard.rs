//! Clipboard access via arboard.

use arboard::{Clipboard, Error};

use crate::error::{AppError, Result};

/// Snapshot of clipboard content before text injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardSnapshot {
    /// Clipboard holds valid UTF-8 text.
    Text(String),
    /// Clipboard holds non-text content (such as images, HTML, or files).
    NonText,
    /// Clipboard could not be accessed or contains no recognized data.
    Unavailable,
}

/// Read a snapshot of the current clipboard content.
pub fn read_snapshot() -> ClipboardSnapshot {
    let mut cb = match Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            tracing::debug!("Clipboard open failed for snapshot: {e}");
            return ClipboardSnapshot::Unavailable;
        }
    };

    categorize_snapshot(cb.get_text(), || probe_has_non_text(&mut cb))
}

/// Check if non-text payload (image, HTML, files) exists on clipboard.
fn probe_has_non_text(cb: &mut Clipboard) -> bool {
    if cb.get_image().is_ok() {
        return true;
    }
    if cb.get().html().is_ok() {
        return true;
    }
    if let Ok(files) = cb.get().file_list() {
        if !files.is_empty() {
            return true;
        }
    }
    false
}

/// Categorize clipboard result into snapshot enum.
pub(crate) fn categorize_snapshot(
    text_result: std::result::Result<String, Error>,
    has_non_text: impl FnOnce() -> bool,
) -> ClipboardSnapshot {
    match text_result {
        Ok(text) => ClipboardSnapshot::Text(text),
        Err(Error::ConversionFailure) => ClipboardSnapshot::NonText,
        Err(Error::ContentNotAvailable) => {
            if has_non_text() {
                ClipboardSnapshot::NonText
            } else {
                ClipboardSnapshot::Unavailable
            }
        }
        Err(_) => ClipboardSnapshot::Unavailable,
    }
}

/// Read text from the clipboard.
pub fn read_text() -> Option<String> {
    match read_snapshot() {
        ClipboardSnapshot::Text(text) => Some(text),
        _ => None,
    }
}

/// Write text to the clipboard.
pub fn write_text(text: &str) -> Result<()> {
    let mut cb =
        Clipboard::new().map_err(|e| AppError::injection(format!("Clipboard open failed: {e}")))?;
    cb.set_text(text.to_string())
        .map_err(|e| AppError::injection(format!("Clipboard set failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize_text_success() {
        let snap = categorize_snapshot(Ok("sample".into()), || false);
        assert_eq!(snap, ClipboardSnapshot::Text("sample".into()));
    }

    #[test]
    fn categorize_conversion_failure_yields_non_text() {
        let snap = categorize_snapshot(Err(Error::ConversionFailure), || false);
        assert_eq!(snap, ClipboardSnapshot::NonText);
    }

    #[test]
    fn categorize_content_not_available_with_non_text() {
        let snap = categorize_snapshot(Err(Error::ContentNotAvailable), || true);
        assert_eq!(snap, ClipboardSnapshot::NonText);
    }

    #[test]
    fn categorize_content_not_available_without_non_text() {
        let snap = categorize_snapshot(Err(Error::ContentNotAvailable), || false);
        assert_eq!(snap, ClipboardSnapshot::Unavailable);
    }

    #[test]
    fn categorize_clipboard_occupied_yields_unavailable() {
        let snap = categorize_snapshot(Err(Error::ClipboardOccupied), || true);
        assert_eq!(snap, ClipboardSnapshot::Unavailable);
    }
}
