//! Keyboard simulation via enigo.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::KeyGuard;
use crate::error::{AppError, Result};

/// Type text character-by-character (Unicode-aware).
pub fn type_text(text: &str) -> Result<u32> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;
    enigo
        .text(text)
        .map_err(|e| AppError::injection(format!("Keystroke text failed: {e}")))?;
    Ok(text.chars().count() as u32)
}

/// Simulate pressing Ctrl+V to paste.
pub fn paste() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| AppError::injection(format!("Ctrl press failed: {e}")))?;
    let _guard = KeyGuard {
        enigo: &mut enigo,
        key: Key::Control,
    };
    _guard
        .enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| AppError::injection(format!("V click failed: {e}")))?;
    Ok(())
}
