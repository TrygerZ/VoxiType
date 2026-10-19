//! Universal text injection into the active application.
//!
//! Default strategy: clipboard paste (set -> Ctrl+V).
//! Fallback: per-character keystroke simulation.

pub mod clipboard;
pub mod command;
pub mod hybrid;
pub mod keystroke;

pub use command::VoiceCommand;
pub use hybrid::HybridInjector;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

/// Open an enigo connection, turning the macOS permission failure into a
/// message the user can act on. enigo prompts for Accessibility itself; the
/// prompt is only useful if the failure that follows explains it.
pub(crate) fn new_enigo() -> Result<Enigo> {
    Enigo::new(&Settings::default()).map_err(|e| match e {
        enigo::NewConError::NoPermission => AppError::injection(
            "Input simulation is not permitted. Grant VoxiType Accessibility \
             permission in System Settings, Privacy and Security, Accessibility.",
        ),
        other => AppError::injection(format!("Enigo init failed: {other}")),
    })
}

/// A letter key for keyboard shortcuts, independent of the active layout.
///
/// enigo exposes named letter keys (`Key::V`) only on Windows, and its macOS
/// `Key::Unicode` lookup resolves through the active layout, returning keycode
/// 0 (the `A` key) when that layout cannot produce the letter. Both platforms
/// therefore use positional codes instead: Windows virtual keys (contiguous
/// from `VK_A`) and macOS ANSI virtual keycodes. Linux keeps `Key::Unicode`,
/// which maps to a keysym.
pub(crate) fn letter_key(letter: char) -> Key {
    #[cfg(windows)]
    {
        const VK_A: u32 = 0x41;
        let offset = u32::from(letter.to_ascii_uppercase()) - u32::from('A');
        Key::Other(VK_A + offset)
    }
    #[cfg(target_os = "macos")]
    {
        // kVK_ANSI_* for a-z, indexed by letter offset.
        const ANSI_KEYCODES: [u32; 26] = [
            0x00, 0x0B, 0x08, 0x02, 0x0E, 0x03, 0x05, 0x04, 0x22, 0x26, 0x28, 0x25, 0x2E, //
            0x2D, 0x1F, 0x23, 0x0C, 0x0F, 0x01, 0x11, 0x20, 0x09, 0x0D, 0x07, 0x10, 0x06,
        ];
        let offset = (u32::from(letter.to_ascii_lowercase()) - u32::from('a')) as usize;
        Key::Other(ANSI_KEYCODES[offset])
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        Key::Unicode(letter.to_ascii_lowercase())
    }
}

/// The platform-appropriate modifier: Command on macOS, Control elsewhere.
pub(crate) fn modifier_key() -> Key {
    #[cfg(target_os = "macos")]
    {
        Key::Meta
    }
    #[cfg(not(target_os = "macos"))]
    {
        Key::Control
    }
}

/// RAII guard that releases a key on drop, preventing keyboard state corruption
/// if an error occurs mid-sequence.
pub(crate) struct KeyGuard<'a> {
    pub(crate) enigo: &'a mut Enigo,
    pub(crate) key: Key,
}

impl<'a> Drop for KeyGuard<'a> {
    fn drop(&mut self) {
        let _ = self.enigo.key(self.key, Direction::Release);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectStrategy {
    Clipboard,
    Keystroke,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectResult {
    pub success: bool,
    pub strategy: InjectStrategy,
    pub chars_injected: u32,
    pub duration_ms: u64,
}

/// Injects formatted text into the focused application.
pub trait TextInjector: Send + Sync {
    fn inject(&self, text: &str) -> Result<InjectResult>;
    fn inject_keystroke(&self, text: &str) -> Result<InjectResult>;
    fn inject_clipboard(&self, text: &str) -> Result<InjectResult>;
}
