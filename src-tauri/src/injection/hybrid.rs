//! Hybrid injector: clipboard paste with keystroke fallback.

use std::sync::Mutex;
use std::time::Instant;

use super::{clipboard, keystroke, InjectResult, InjectStrategy, TextInjector};
use crate::error::Result;
use crate::util::MutexExt;

static CLIPBOARD_INJECTION_LOCK: Mutex<()> = Mutex::new(());

/// Pause after writing to the clipboard before issuing the paste keystroke.
///
/// Why: the clipboard write must propagate through the OS clipboard service
/// before the target app reads it. A value too small risks pasting stale
/// content; a value too large adds latency on every injection.
const CLIPBOARD_PROPAGATION_DELAY_MS: u64 = 30;

/// Wait for the target app to CONSUME the paste before restoring the
/// clipboard.
///
/// Why not shorter: `keystroke::paste()` only posts Ctrl+V to the system
/// input queue — the target app processes it asynchronously. Slow apps
/// (Electron, heavy IDEs) can take hundreds of milliseconds; restoring too
/// early makes them paste the OLD clipboard content instead of the dictated
/// text (bug B-03). It also keeps the dictated text out of clipboard
/// managers for this window.
///
/// Trade-off: total injection latency grows by this amount (~480 ms end-to-end),
/// but a wrong paste is far worse than a slower one. If paste races are still
/// reported in the field, prefer raising this over adding global state or
/// lazy restore.
const PASTE_CONSUME_DELAY_MS: u64 = 450;

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
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        let started = Instant::now();

        // Save current clipboard content
        let prev = clipboard::read_text();

        clipboard::write_text(text)?;

        // Brief pause so the clipboard write propagates before we paste.
        std::thread::sleep(std::time::Duration::from_millis(
            CLIPBOARD_PROPAGATION_DELAY_MS,
        ));

        keystroke::paste()?;

        // Wait for the target app to consume the paste before restoring.
        // Keystrokes are processed asynchronously by the target app.
        std::thread::sleep(std::time::Duration::from_millis(PASTE_CONSUME_DELAY_MS));

        restore_clipboard(prev.as_deref());

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

fn restore_clipboard(prev: Option<&str>) {
    match prev {
        Some(prev_text) => {
            if let Err(e) = clipboard::write_text(prev_text) {
                tracing::warn!(
                    "Failed to restore clipboard ({e}); wiping dictated text from clipboard"
                );
                wipe_clipboard_fail_safe();
            }
        }
        None => {
            tracing::debug!("Clipboard was empty before injection; wiping dictated text");
            wipe_clipboard_fail_safe();
        }
    }
}

/// Privacy fail-safe: overwrite the clipboard with an empty string so
/// dictated text does not linger where other apps can read it.
fn wipe_clipboard_fail_safe() {
    if let Err(wipe_err) = clipboard::write_text("") {
        tracing::error!("Failed to wipe clipboard: {wipe_err}");
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

    #[test]
    fn clipboard_lock_serializes_concurrent_access() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let active_count = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let mut handles = Vec::new();
        for _ in 0..4 {
            let active = active_count.clone();
            let max = max_active.clone();
            handles.push(std::thread::spawn(move || {
                let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
                let cur = active.fetch_add(1, Ordering::SeqCst) + 1;
                max.fetch_max(cur, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(15));
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }
}
