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

use enigo::{Direction, Enigo, Key, Keyboard};
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// A letter key for keyboard shortcuts, independent of the active layout.
///
/// enigo 0.2.1 exposes named letter keys (`Key::V`) only on Windows. Windows
/// virtual-key codes for A-Z are contiguous from `VK_A`, so the same positional
/// key is built with `Key::Other` there. Other platforms accept a Unicode
/// character, which enigo maps to a keysym on Linux and a keycode on macOS.
pub(crate) fn letter_key(letter: char) -> Key {
    #[cfg(windows)]
    {
        const VK_A: u32 = 0x41;
        let offset = u32::from(letter.to_ascii_uppercase()) - u32::from('A');
        Key::Other(VK_A + offset)
    }
    #[cfg(not(windows))]
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
