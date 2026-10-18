//! Keyboard simulation via enigo.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::{letter_key, modifier_key, KeyGuard};
use crate::error::{AppError, Result};

/// One step in a keystroke plan: a run of printable text, or a named key press.
#[derive(Debug, PartialEq, Eq)]
enum KeystrokeAction {
    Text(String),
    Press(Key),
}

/// Build the injection plan for `text`.
///
/// enigo 0.2.1's Windows `text()` returns early on `'\n'` and `'\t'`, silently
/// discarding the rest of its buffer, so text around either separator would be
/// lost and the injected count over-reported. The text is therefore split at
/// those separators and each is emitted as an explicit key press. The scan runs
/// left to right, drops `'\r'` (never injected), accumulates printable runs into
/// `Text`, and emits one key action per separator. Consecutive separators emit
/// consecutive key actions, and no empty `Text` entries are produced.
fn keystroke_sequence(text: &str) -> Vec<KeystrokeAction> {
    let mut actions = Vec::new();
    let mut run = String::new();
    for ch in text.chars() {
        match ch {
            '\n' => {
                flush_text_run(&mut actions, &mut run);
                actions.push(KeystrokeAction::Press(Key::Return));
            }
            '\t' => {
                flush_text_run(&mut actions, &mut run);
                actions.push(KeystrokeAction::Press(Key::Tab));
            }
            '\r' => {}
            _ => run.push(ch),
        }
    }
    flush_text_run(&mut actions, &mut run);
    actions
}

/// Move the accumulated text run into the plan, if any, leaving it empty.
fn flush_text_run(actions: &mut Vec<KeystrokeAction>, run: &mut String) {
    if !run.is_empty() {
        actions.push(KeystrokeAction::Text(std::mem::take(run)));
    }
}

/// Count characters actually injected: one per text character plus one per key
/// press (Return and Tab each count as one).
fn injected_count(actions: &[KeystrokeAction]) -> u32 {
    actions
        .iter()
        .map(|action| match action {
            KeystrokeAction::Text(segment) => segment.chars().count() as u32,
            KeystrokeAction::Press(_) => 1,
        })
        .sum()
}

/// Type text character-by-character (Unicode-aware).
///
/// The returned count reflects only characters actually injected: one per
/// segment character plus one per key press between segments.
pub fn type_text(text: &str) -> Result<u32> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;

    let actions = keystroke_sequence(text);
    for action in &actions {
        match action {
            KeystrokeAction::Text(segment) => {
                enigo
                    .text(segment)
                    .map_err(|e| AppError::injection(format!("Keystroke text failed: {e}")))?;
            }
            KeystrokeAction::Press(key) => {
                enigo
                    .key(*key, Direction::Click)
                    .map_err(|e| AppError::injection(format!("Key click failed: {e}")))?;
            }
        }
    }
    Ok(injected_count(&actions))
}

/// Simulate pressing Ctrl+V (Cmd+V on macOS) to paste.
pub fn paste() -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;
    let modifier = modifier_key();
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| AppError::injection(format!("Paste modifier press failed: {e}")))?;
    let _guard = KeyGuard {
        enigo: &mut enigo,
        key: modifier,
    };
    // Named key: layout-independent, unlike Key::Unicode('v') which fails on
    // non-Latin layouts.
    _guard
        .enigo
        .key(letter_key('v'), Direction::Click)
        .map_err(|e| AppError::injection(format!("V click failed: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> KeystrokeAction {
        KeystrokeAction::Text(value.to_string())
    }

    #[test]
    fn paste_modifier_is_command_on_macos_only() {
        #[cfg(target_os = "macos")]
        assert_eq!(super::modifier_key(), enigo::Key::Meta);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(super::modifier_key(), enigo::Key::Control);
    }

    #[test]
    fn single_line_is_one_text_run() {
        assert_eq!(keystroke_sequence("hello world"), vec![text("hello world")]);
    }

    #[test]
    fn tab_splits_into_key_press() {
        assert_eq!(
            keystroke_sequence("a\tb"),
            vec![text("a"), KeystrokeAction::Press(Key::Tab), text("b")]
        );
    }

    #[test]
    fn newline_splits_into_return() {
        assert_eq!(
            keystroke_sequence("a\nb"),
            vec![text("a"), KeystrokeAction::Press(Key::Return), text("b")]
        );
    }

    #[test]
    fn mixed_separators_emit_consecutive_keys() {
        assert_eq!(
            keystroke_sequence("a\t\nb"),
            vec![
                text("a"),
                KeystrokeAction::Press(Key::Tab),
                KeystrokeAction::Press(Key::Return),
                text("b"),
            ]
        );
    }

    #[test]
    fn leading_separator_has_no_empty_text() {
        assert_eq!(
            keystroke_sequence("\ta"),
            vec![KeystrokeAction::Press(Key::Tab), text("a")]
        );
    }

    #[test]
    fn trailing_separator_has_no_empty_text() {
        assert_eq!(
            keystroke_sequence("a\t"),
            vec![text("a"), KeystrokeAction::Press(Key::Tab)]
        );
    }

    #[test]
    fn carriage_returns_are_stripped() {
        assert_eq!(
            keystroke_sequence("a\r\tb"),
            vec![text("a"), KeystrokeAction::Press(Key::Tab), text("b")]
        );
        assert_eq!(keystroke_sequence("lone\rcr"), vec![text("lonecr")]);
    }

    #[test]
    fn consecutive_newlines_emit_consecutive_returns() {
        assert_eq!(
            keystroke_sequence("a\n\nb"),
            vec![
                text("a"),
                KeystrokeAction::Press(Key::Return),
                KeystrokeAction::Press(Key::Return),
                text("b"),
            ]
        );
    }

    #[test]
    fn count_includes_text_chars_and_key_presses() {
        assert_eq!(injected_count(&keystroke_sequence("a\tb")), 3);
        assert_eq!(injected_count(&keystroke_sequence("a\n\nb")), 4);
    }
}
