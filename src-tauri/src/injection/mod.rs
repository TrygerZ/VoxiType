//! Universal text injection into the active application.
//!
//! Default strategy: clipboard paste (save -> set -> Ctrl+V -> restore).
//! Fallback: per-character keystroke simulation.

pub mod clipboard;
pub mod command;
pub mod hybrid;
pub mod keystroke;

pub use command::VoiceCommand;
pub use hybrid::HybridInjector;

use serde::{Deserialize, Serialize};

use crate::error::Result;

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
