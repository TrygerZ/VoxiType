//! Keyboard simulation via enigo.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::KeyGuard;
use crate::error::{AppError, Result};

/// Split text into the segments injected between explicit Return presses.
///
/// enigo 0.2.1's Windows `text()` returns early on `'\n'`, silently dropping
/// the rest of the string, so newlines are split out here and typed as
/// explicit Return key presses. `'\r'` is stripped because it is never
/// injected. Empty lines are preserved so consecutive newlines still produce
/// the right number of Return presses.
pub fn newline_segments(text: &str) -> Vec<String> {
    text.split('\n')
        .map(|segment| segment.replace('\r', ""))
        .collect()
}

/// Type text character-by-character (Unicode-aware).
///
/// The returned count reflects only characters actually injected: one per
/// segment character plus one per Return between segments.
pub fn type_text(text: &str) -> Result<u32> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;

    let segments = newline_segments(text);
    let mut injected = 0u32;
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            enigo
                .key(Key::Return, Direction::Click)
                .map_err(|e| AppError::injection(format!("Return click failed: {e}")))?;
            injected += 1;
        }
        if segment.is_empty() {
            continue;
        }
        enigo
            .text(segment)
            .map_err(|e| AppError::injection(format!("Keystroke text failed: {e}")))?;
        injected += segment.chars().count() as u32;
    }
    Ok(injected)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_line_is_one_segment() {
        assert_eq!(newline_segments("hello world"), vec!["hello world"]);
    }

    #[test]
    fn newlines_split_into_segments() {
        assert_eq!(
            newline_segments("line one\nline two\nline three"),
            vec!["line one", "line two", "line three"]
        );
    }

    #[test]
    fn carriage_returns_are_stripped() {
        assert_eq!(newline_segments("a\r\nb"), vec!["a", "b"]);
        assert_eq!(newline_segments("lone\rcr"), vec!["lonecr"]);
    }

    #[test]
    fn blank_lines_are_preserved() {
        assert_eq!(newline_segments("a\n\nb"), vec!["a", "", "b"]);
    }
}
