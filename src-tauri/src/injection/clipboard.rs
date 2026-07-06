//! Clipboard access via arboard.

use arboard::Clipboard;

use crate::error::{AppError, Result};

/// Write text to the clipboard.
pub fn write_text(text: &str) -> Result<()> {
    let mut cb =
        Clipboard::new().map_err(|e| AppError::injection(format!("Clipboard open failed: {e}")))?;
    cb.set_text(text.to_string())
        .map_err(|e| AppError::injection(format!("Clipboard set failed: {e}")))?;
    Ok(())
}
