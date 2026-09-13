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
