//! Hybrid injector: clipboard paste with keystroke fallback.

use std::time::Instant;

use super::{clipboard, keystroke, InjectResult, InjectStrategy, TextInjector};
use crate::error::Result;

pub struct HybridInjector;

impl HybridInjector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HybridInjector {
    fn default() -> Self {
        Self
    }
}

impl TextInjector for HybridInjector {
    fn inject(&self, text: &str) -> Result<InjectResult> {
        if text.is_empty() {
            return Ok(InjectResult {
                success: true,
                strategy: InjectStrategy::Manual,
                chars_injected: 0,
                duration_ms: 0,
            });
        }

        // Try clipboard first.
        match self.inject_clipboard(text) {
            Ok(res) if res.success => Ok(res),
            _ => self.inject_keystroke(text),
        }
    }

    fn inject_clipboard(&self, text: &str) -> Result<InjectResult> {
        let started = Instant::now();

        // Save current clipboard content
        let prev = clipboard::read_text();

        clipboard::write_text(text)?;

        // Brief pause so the clipboard write propagates before we paste.
        std::thread::sleep(std::time::Duration::from_millis(30));

        keystroke::paste()?;

        // CRITICAL: wait for the target app to consume the paste before
        // restoring the clipboard.  enigo keystrokes are posted to the
        // system input queue and processed asynchronously — if we restore
        // the old clipboard immediately, the app reads the OLD content
        // instead of our dictated text.
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Restore previous clipboard if we had one
        if let Some(prev_text) = prev {
            if let Err(e) = clipboard::write_text(&prev_text) {
                tracing::warn!("Failed to restore clipboard: {e}");
            }
        }

        Ok(InjectResult {
            success: true,
            strategy: InjectStrategy::Clipboard,
            chars_injected: text.chars().count() as u32,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    fn inject_keystroke(&self, text: &str) -> Result<InjectResult> {
        let started = Instant::now();
        let chars = keystroke::type_text(text)?;
        Ok(InjectResult {
            success: true,
            strategy: InjectStrategy::Keystroke,
            chars_injected: chars,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_manual_strategy() {
        let injector = HybridInjector::new();
        let result = injector.inject("").unwrap();
        assert!(result.success);
        assert_eq!(result.strategy, InjectStrategy::Manual);
        assert_eq!(result.chars_injected, 0);
    }

    #[test]
    fn empty_text_bypasses_clipboard_and_keystroke() {
        // Verify empty text doesn't touch system clipboard or enigo.
        // The Manual strategy confirms we short-circuit before any I/O.
        let result = HybridInjector::new().inject("").unwrap();
        assert_eq!(result.strategy, InjectStrategy::Manual);
        assert_eq!(result.duration_ms, 0);
    }

    #[test]
    fn clipboard_save_restore_preserves_original() {
        use crate::injection::clipboard;

        // Save whatever is currently on the clipboard.
        let original = clipboard::read_text();

        // Write a unique test string and verify it stuck.
        let test_text = "VOXITYPE_TEST_CLIPBOARD_SAVE_RESTORE";
        clipboard::write_text(test_text).unwrap();
        let readback = clipboard::read_text();
        assert_eq!(readback.as_deref(), Some(test_text));

        // Restore the original.
        if let Some(orig) = original.as_ref() {
            clipboard::write_text(orig).unwrap();
            let restored = clipboard::read_text();
            assert_eq!(restored.as_deref(), Some(orig.as_str()));
        }
    }
}
