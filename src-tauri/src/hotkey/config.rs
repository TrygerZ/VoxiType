//! Hotkey configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyMode {
    /// Press and hold to record, release to process.
    #[default]
    Ptt,
    /// Press once to start, again to stop.
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Accelerator string, e.g. "Ctrl+Space".
    pub key: String,
    /// A missing `mode` keeps the default (push-to-talk); an unknown mode
    /// string fails deserialization instead of silently coercing.
    #[serde(default)]
    pub mode: HotkeyMode,
}

/// The default accelerator. macOS uses Super+Shift+Space because Ctrl+Space
/// is reserved for input-source switching; Windows keeps Ctrl+Space.
fn default_accelerator() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Super+Shift+Space"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "Ctrl+Space"
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            key: default_accelerator().to_string(),
            mode: HotkeyMode::Ptt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkey_is_platform_aware() {
        let cfg = HotkeyConfig::default();
        #[cfg(target_os = "macos")]
        assert_eq!(cfg.key, "Super+Shift+Space");
        #[cfg(not(target_os = "macos"))]
        assert_eq!(cfg.key, "Ctrl+Space");
        assert_eq!(cfg.mode, HotkeyMode::Ptt);
    }
}
