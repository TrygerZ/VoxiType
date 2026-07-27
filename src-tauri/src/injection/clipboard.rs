//! Clipboard access via arboard.

use arboard::Clipboard;

use crate::error::{AppError, Result};

/// Read text from the clipboard.
pub fn read_text() -> Option<String> {
    Clipboard::new().ok().and_then(|mut cb| cb.get_text().ok())
}

/// Write text to the clipboard.
pub fn write_text(text: &str) -> Result<()> {
    let mut cb =
        Clipboard::new().map_err(|e| AppError::injection(format!("Clipboard open failed: {e}")))?;
    cb.set_text(text.to_string())
        .map_err(|e| AppError::injection(format!("Clipboard set failed: {e}")))?;
    Ok(())
}
