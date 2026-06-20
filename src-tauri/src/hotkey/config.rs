//! Hotkey configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyMode {
    /// Press and hold to record, release to process.
    Ptt,
    /// Press once to start, again to stop.
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Accelerator string, e.g. "Ctrl+Space".
    pub key: String,
    pub mode: HotkeyMode,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            key: "Ctrl+Space".to_string(),
            mode: HotkeyMode::Ptt,
        }
    }
}
