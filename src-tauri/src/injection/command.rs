//! Voice command mode: map spoken phrases to editing keystrokes.
//!
//! When command mode is active, the transcribed text is matched against a set
//! of known commands ("new line", "baris baru", "delete that", ...) and the
//! corresponding key action is simulated instead of injecting the literal text.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::keystroke::primary_modifier;
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
            // macOS: Option+Backspace. Windows/Linux: Ctrl+Backspace.
            #[cfg(target_os = "macos")]
            let word_mod = Key::Alt;
            #[cfg(not(target_os = "macos"))]
            let word_mod = Key::Control;
            enigo.key(word_mod, Direction::Press).map_err(map_err)?;
            enigo
                .key(Key::Backspace, Direction::Click)
                .map_err(map_err)?;
            enigo.key(word_mod, Direction::Release).map_err(map_err)?;
        }
        VoiceCommand::SelectAll => combo(&mut enigo, 'a', map_err)?,
        VoiceCommand::Copy => combo(&mut enigo, 'c', map_err)?,
        VoiceCommand::Paste => combo(&mut enigo, 'v', map_err)?,
        VoiceCommand::Cut => combo(&mut enigo, 'x', map_err)?,
        VoiceCommand::Undo => combo(&mut enigo, 'z', map_err)?,
        VoiceCommand::Redo => {
            // macOS: Cmd+Shift+Z. Windows/Linux: Ctrl+Y.
            #[cfg(target_os = "macos")]
            {
                let modifier = primary_modifier();
                enigo.key(modifier, Direction::Press).map_err(map_err)?;
                enigo.key(Key::Shift, Direction::Press).map_err(map_err)?;
                enigo
                    .key(Key::Unicode('z'), Direction::Click)
                    .map_err(map_err)?;
                enigo
                    .key(Key::Shift, Direction::Release)
                    .map_err(map_err)?;
                enigo.key(modifier, Direction::Release).map_err(map_err)?;
            }
            #[cfg(not(target_os = "macos"))]
            {
                combo(&mut enigo, 'y', map_err)?;
            }
        }
        VoiceCommand::Save => combo(&mut enigo, 's', map_err)?,
        VoiceCommand::Escape => {
            enigo.key(Key::Escape, Direction::Click).map_err(map_err)?;
        }
    }
    Ok(())
}

/// Simulate primary-modifier+<key> (Cmd on macOS, Ctrl elsewhere).
fn combo(
    enigo: &mut Enigo,
    key: char,
    map_err: impl Fn(enigo::InputError) -> AppError,
) -> Result<()> {
    let modifier = primary_modifier();
    enigo.key(modifier, Direction::Press).map_err(&map_err)?;
    enigo
        .key(Key::Unicode(key), Direction::Click)
        .map_err(&map_err)?;
    enigo.key(modifier, Direction::Release).map_err(&map_err)?;
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
}
