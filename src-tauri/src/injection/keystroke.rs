//! Keyboard simulation via enigo.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use crate::error::{AppError, Result};

/// Primary shortcut modifier: Cmd (Meta) on macOS, Ctrl elsewhere.
pub fn primary_modifier() -> Key {
    #[cfg(target_os = "macos")]
    {
        Key::Meta
    }
    #[cfg(not(target_os = "macos"))]
    {
        Key::Control
    }
}

/// Type text character-by-character (Unicode-aware).
pub fn type_text(text: &str) -> Result<u32> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;
    enigo
        .text(text)
        .map_err(|e| AppError::injection(format!("Keystroke text failed: {e}")))?;
    Ok(text.chars().count() as u32)
}

/// Simulate primary-modifier+V (Cmd+V on macOS, Ctrl+V elsewhere).
pub fn paste() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;
    let modifier = primary_modifier();
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| AppError::injection(format!("modifier press failed: {e}")))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| AppError::injection(format!("V click failed: {e}")))?;
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| AppError::injection(format!("modifier release failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_modifier_is_platform_correct() {
        #[cfg(target_os = "macos")]
        assert_eq!(primary_modifier(), Key::Meta);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(primary_modifier(), Key::Control);
    }
}
