//! Voice command mode: map spoken phrases to editing keystrokes.
//!
//! When command mode is active, the transcribed text is matched against a set
//! of known commands ("new line", "baris baru", "delete that", ...) and the
//! corresponding key action is simulated instead of injecting the literal text.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::KeyGuard;
use crate::error::{AppError, Result};

/// Get the platform-appropriate modifier key (Cmd on macOS, Ctrl elsewhere).
#[cfg(target_os = "macos")]
fn modifier_key() -> Key {
    Key::Meta
}

#[cfg(not(target_os = "macos"))]
fn modifier_key() -> Key {
    Key::Control
}

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
/// when capture started. If the window changed, abort to prevent executing
/// editing macros in the wrong application.
pub fn verify_target_window(expected: Option<&str>, current: Option<&str>) -> Result<()> {
    if let Some(initial) = expected {
        if current != Some(initial) {
            let current_display = current.unwrap_or("none");
            tracing::warn!(
                "Foreground window changed from {initial} to {current_display}; aborting command injection"
            );
            return Err(AppError::injection(format!(
                "Command aborted: target window changed from {initial} to {current_display}"
            )));
        }
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
        VoiceCommand::SelectAll => combo(&mut enigo, 'a', map_err)?,
        VoiceCommand::Copy => combo(&mut enigo, 'c', map_err)?,
        VoiceCommand::Paste => combo(&mut enigo, 'v', map_err)?,
        VoiceCommand::Cut => combo(&mut enigo, 'x', map_err)?,
        VoiceCommand::Undo => combo(&mut enigo, 'z', map_err)?,
        VoiceCommand::Redo => combo(&mut enigo, 'y', map_err)?,
        VoiceCommand::Save => combo(&mut enigo, 's', map_err)?,
        VoiceCommand::Escape => {
            enigo.key(Key::Escape, Direction::Click).map_err(map_err)?;
        }
    }
    Ok(())
}

/// Simulate Ctrl+<key> on Windows/Linux or Cmd+<key> on macOS.
fn combo(
    enigo: &mut Enigo,
    key: char,
    map_err: impl Fn(enigo::InputError) -> AppError,
) -> Result<()> {
    let modifier = modifier_key();
    enigo.key(modifier, Direction::Press).map_err(&map_err)?;
    let _guard = KeyGuard {
        enigo,
        key: modifier,
    };
    _guard
        .enigo
        .key(Key::Unicode(key), Direction::Click)
        .map_err(&map_err)?;
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
    fn verifies_target_window_matches() {
        assert!(verify_target_window(Some("code"), Some("code")).is_ok());
        assert!(verify_target_window(None, Some("code")).is_ok());
        assert!(verify_target_window(None, None).is_ok());

        let err_diff = verify_target_window(Some("code"), Some("chrome")).unwrap_err();
        assert_eq!(err_diff.code, crate::error::ErrorCode::InjectionFailed);

        let err_lost = verify_target_window(Some("code"), None).unwrap_err();
        assert_eq!(err_lost.code, crate::error::ErrorCode::InjectionFailed);
    }
}
