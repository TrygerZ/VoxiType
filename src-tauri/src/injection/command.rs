//! Voice command mode: map spoken phrases to editing keystrokes.
//!
//! When command mode is active, the transcribed text is matched against a set
//! of known commands ("new line", "baris baru", "delete that", ...) and the
//! corresponding key action is simulated instead of injecting the literal text.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::{letter_key, modifier_key, KeyGuard};
use crate::error::{AppError, Result};

/// A recognized editing action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceCommand {
    NewLine,
    NewParagraph,
    Tab,
    Backspace,
    DeleteWord,
    SelectAll,
    Copy,
    Paste,
    Cut,
    Undo,
    Redo,
    Save,
    Escape,
}

impl VoiceCommand {
    /// Match a normalized phrase (lowercased, trimmed) to a command.
    pub fn from_phrase(phrase: &str) -> Option<Self> {
        let p = phrase
            .trim()
            .trim_end_matches(['.', '!', '?'])
            .trim()
            .to_lowercase();
        match p.as_str() {
            "new line" | "newline" | "baris baru" | "enter" => Some(Self::NewLine),
            "new paragraph" | "paragraf baru" => Some(Self::NewParagraph),
            "tab" | "indent" => Some(Self::Tab),
            "backspace" | "hapus" | "delete" => Some(Self::Backspace),
            "delete word" | "hapus kata" => Some(Self::DeleteWord),
            "select all" | "pilih semua" => Some(Self::SelectAll),
            "copy" | "salin" => Some(Self::Copy),
            "paste" | "tempel" => Some(Self::Paste),
            "cut" | "potong" => Some(Self::Cut),
            "undo" | "batal" => Some(Self::Undo),
            "redo" | "ulangi" => Some(Self::Redo),
            "save" | "simpan" => Some(Self::Save),
            "escape" | "batalkan" => Some(Self::Escape),
            _ => None,
        }
    }
}

/// Verify that the current foreground window matches the window that was active
/// when capture started. Fails closed: an unverifiable start (no window handle
/// was captured) aborts, and a changed window aborts, so editing macros never
/// run in the wrong application.
pub fn verify_target_window(expected: Option<isize>, current: Option<isize>) -> Result<()> {
    let Some(expected) = expected else {
        tracing::warn!("No target window captured at recording start; aborting command injection");
        return Err(AppError::injection(
            "Command aborted: target window could not be verified",
        ));
    };
    if current != Some(expected) {
        let current_display = current
            .map(|h| h.to_string())
            .unwrap_or_else(|| "none".to_string());
        tracing::warn!(
            "Foreground window changed from {expected} to {current_display}; aborting command injection"
        );
        return Err(AppError::injection(format!(
            "Command aborted: target window changed from {expected} to {current_display}"
        )));
    }
    Ok(())
}

/// Execute a voice command by simulating the corresponding keystrokes.
pub fn execute(command: VoiceCommand) -> Result<()> {
    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| AppError::injection(format!("Enigo init failed: {e}")))?;

    let map_err = |e| AppError::injection(format!("Keystroke failed: {e}"));

    match command {
        VoiceCommand::NewLine => {
            enigo.key(Key::Return, Direction::Click).map_err(map_err)?;
        }
        VoiceCommand::NewParagraph => {
            enigo.key(Key::Return, Direction::Click).map_err(map_err)?;
            enigo.key(Key::Return, Direction::Click).map_err(map_err)?;
        }
        VoiceCommand::Tab => {
            enigo.key(Key::Tab, Direction::Click).map_err(map_err)?;
        }
        VoiceCommand::Backspace => {
            enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(map_err)?;
        }
        VoiceCommand::DeleteWord => {
            // Ctrl+Backspace (Win/Linux) or Cmd+Backspace (macOS) deletes the previous word.
            let modifier = modifier_key();
            enigo.key(modifier, Direction::Press).map_err(map_err)?;
            let _guard = KeyGuard {
                enigo: &mut enigo,
                key: modifier,
            };
            _guard
                .enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(map_err)?;
        }
        VoiceCommand::SelectAll
        | VoiceCommand::Copy
        | VoiceCommand::Paste
        | VoiceCommand::Cut
        | VoiceCommand::Undo
        | VoiceCommand::Redo
        | VoiceCommand::Save => combo(&mut enigo, command, map_err)?,
        VoiceCommand::Escape => {
            enigo.key(Key::Escape, Direction::Click).map_err(map_err)?;
        }
    }
    Ok(())
}

/// Layout-independent letter key for a Ctrl/Cmd combo command.
///
/// `Key::Unicode(letter)` resolves through `VkKeyScanW` on Windows, which fails
/// on non-Latin keyboard layouts; `letter_key` uses the positional virtual key
/// there instead. Returns `None` for commands that are not combos.
fn combo_key(command: VoiceCommand) -> Option<Key> {
    match command {
        VoiceCommand::SelectAll => Some(letter_key('a')),
        VoiceCommand::Copy => Some(letter_key('c')),
        VoiceCommand::Paste => Some(letter_key('v')),
        VoiceCommand::Cut => Some(letter_key('x')),
        VoiceCommand::Undo => Some(letter_key('z')),
        VoiceCommand::Redo => Some(letter_key('y')),
        VoiceCommand::Save => Some(letter_key('s')),
        _ => None,
    }
}

/// Simulate Ctrl+<key> on Windows/Linux or Cmd+<key> on macOS.
fn combo(
    enigo: &mut Enigo,
    command: VoiceCommand,
    map_err: impl Fn(enigo::InputError) -> AppError,
) -> Result<()> {
    let key = combo_key(command)
        .ok_or_else(|| AppError::injection(format!("{command:?} is not a combo command")))?;
    let modifier = modifier_key();
    enigo.key(modifier, Direction::Press).map_err(&map_err)?;
    let _guard = KeyGuard {
        enigo,
        key: modifier,
    };
    _guard.enigo.key(key, Direction::Click).map_err(&map_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_known_phrases_both_languages() {
        assert_eq!(
            VoiceCommand::from_phrase("new line"),
            Some(VoiceCommand::NewLine)
        );
        assert_eq!(
            VoiceCommand::from_phrase("Baris Baru"),
            Some(VoiceCommand::NewLine)
        );
        assert_eq!(
            VoiceCommand::from_phrase("select all"),
            Some(VoiceCommand::SelectAll)
        );
        assert_eq!(
            VoiceCommand::from_phrase("simpan."),
            Some(VoiceCommand::Save)
        );
    }

    #[test]
    fn rejects_unknown_phrases() {
        assert_eq!(VoiceCommand::from_phrase("halo dunia"), None);
        assert_eq!(VoiceCommand::from_phrase(""), None);
    }

    #[test]
    fn combo_commands_map_to_layout_independent_letter_keys() {
        assert_eq!(combo_key(VoiceCommand::SelectAll), Some(letter_key('a')));
        assert_eq!(combo_key(VoiceCommand::Copy), Some(letter_key('c')));
        assert_eq!(combo_key(VoiceCommand::Paste), Some(letter_key('v')));
        assert_eq!(combo_key(VoiceCommand::Cut), Some(letter_key('x')));
        assert_eq!(combo_key(VoiceCommand::Undo), Some(letter_key('z')));
        assert_eq!(combo_key(VoiceCommand::Redo), Some(letter_key('y')));
        assert_eq!(combo_key(VoiceCommand::Save), Some(letter_key('s')));
    }

    #[test]
    fn non_combo_commands_have_no_combo_key() {
        assert_eq!(combo_key(VoiceCommand::NewLine), None);
        assert_eq!(combo_key(VoiceCommand::Tab), None);
        assert_eq!(combo_key(VoiceCommand::Escape), None);
    }

    #[test]
    fn verifies_target_window_matches() {
        assert!(verify_target_window(Some(100), Some(100)).is_ok());

        let err_diff = verify_target_window(Some(100), Some(200)).unwrap_err();
        assert_eq!(err_diff.code, crate::error::ErrorCode::InjectionFailed);

        let err_lost = verify_target_window(Some(100), None).unwrap_err();
        assert_eq!(err_lost.code, crate::error::ErrorCode::InjectionFailed);
    }

    #[test]
    fn verifies_target_window_fails_closed_when_unverifiable() {
        // No window handle captured at start: verification is impossible, so abort.
        let err_unknown = verify_target_window(None, Some(100)).unwrap_err();
        assert_eq!(err_unknown.code, crate::error::ErrorCode::InjectionFailed);

        let err_both_none = verify_target_window(None, None).unwrap_err();
        assert_eq!(err_both_none.code, crate::error::ErrorCode::InjectionFailed);
    }
}
