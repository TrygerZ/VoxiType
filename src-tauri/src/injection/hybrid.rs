//! Hybrid injector: clipboard paste with keystroke fallback.

use std::sync::Mutex;
use std::time::Instant;

use super::{clipboard, keystroke, InjectResult, InjectStrategy, TextInjector};
use crate::error::Result;
use crate::util::MutexExt;

/// Serializes real-clipboard mutations across injection tests.
pub(crate) static CLIPBOARD_INJECTION_LOCK: Mutex<()> = Mutex::new(());

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

        // Save current clipboard content before writing dictation text.
        let prev = clipboard::read_snapshot();
        let mut restore_guard = ClipboardRestoreGuard::new(prev, text.to_string());

        clipboard::write_text(text)?;

        // Brief pause so the clipboard write propagates before we paste.
        std::thread::sleep(std::time::Duration::from_millis(
            CLIPBOARD_PROPAGATION_DELAY_MS,
        ));

        keystroke::paste()?;

        // Wait for the target app to consume the paste before restoring.
        // Keystrokes are processed asynchronously by the target app.
        std::thread::sleep(std::time::Duration::from_millis(PASTE_CONSUME_DELAY_MS));

        restore_guard.restore();

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

/// RAII guard ensuring clipboard restoration on all exit paths.
struct ClipboardRestoreGuard {
    snapshot: Option<clipboard::ClipboardSnapshot>,
    /// The text this injection wrote to the clipboard. The restore is skipped
    /// when the clipboard no longer holds it, so an external copy made during
    /// the injection window is preserved.
    dictated: String,
}

impl ClipboardRestoreGuard {
    fn new(snapshot: clipboard::ClipboardSnapshot, dictated: String) -> Self {
        Self {
            snapshot: Some(snapshot),
            dictated,
        }
    }

    /// Restore previous clipboard content and disarm guard.
    fn restore(&mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            restore_clipboard(&snapshot, &self.dictated);
        }
    }
}

impl Drop for ClipboardRestoreGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

fn restore_clipboard(prev: &clipboard::ClipboardSnapshot, dictated: &str) {
    match clipboard::read_text() {
        Some(current) if current == dictated => {}
        // The clipboard no longer holds the dictated text: an external writer
        // intervened (user copy, clipboard manager, sync tool). Keep theirs.
        // `None` also lands here: non-text, empty, or unreadable content means
        // we cannot prove the dictated text is still there, so do not clobber.
        _ => {
            tracing::info!(
                "Clipboard changed during injection; keeping external content, skipping restore"
            );
            return;
        }
    }

    match prev {
        clipboard::ClipboardSnapshot::Text(prev_text) => {
            if let Err(e) = clipboard::write_text(prev_text) {
                tracing::warn!(
                    "Failed to restore clipboard ({e}); wiping dictated text from clipboard"
                );
                wipe_clipboard_fail_safe();
            }
        }
        clipboard::ClipboardSnapshot::Image {
            width,
            height,
            bytes,
        } => {
            if let Err(e) = clipboard::write_image(*width, *height, bytes) {
                tracing::warn!(
                    "Failed to restore clipboard image ({e}); wiping dictated text from clipboard"
                );
                wipe_clipboard_fail_safe();
            }
        }
        clipboard::ClipboardSnapshot::NonText => {
            // ponytail: HTML and file-list content still cannot be re-serialized
            // through the text API, so they keep the wipe. Images are restored via
            // the Image arm above. Upgrade when arboard or the platform API supports
            // raw multi-format preservation.
            tracing::debug!(
                "Non-restorable non-text content on clipboard before injection; wiping dictated text"
            );
            wipe_clipboard_fail_safe();
        }
        clipboard::ClipboardSnapshot::Unavailable => {
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
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
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

    #[test]
    fn restore_guard_restores_on_drop() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::{self, ClipboardSnapshot};

        let test_prev = "VOXITYPE_TEST_GUARD_PREV";
        clipboard::write_text(test_prev).unwrap();

        {
            let guard = ClipboardRestoreGuard::new(
                ClipboardSnapshot::Text(test_prev.to_string()),
                "OVERWRITTEN".to_string(),
            );
            clipboard::write_text("OVERWRITTEN").unwrap();
            assert_eq!(clipboard::read_text().as_deref(), Some("OVERWRITTEN"));
            drop(guard);
        }

        assert_eq!(clipboard::read_text().as_deref(), Some(test_prev));
    }

    #[test]
    fn restore_guard_explicit_restore_disarms() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::ClipboardSnapshot;

        let test_prev = "VOXITYPE_TEST_GUARD_EXPLICIT";
        let mut guard = ClipboardRestoreGuard::new(
            ClipboardSnapshot::Text(test_prev.to_string()),
            test_prev.to_string(),
        );
        guard.restore();
        assert!(guard.snapshot.is_none());
    }

    #[test]
    fn restore_guard_restores_image_content() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::{self, ClipboardSnapshot};

        // 2x2 RGBA8 image on the clipboard before injection.
        let bytes = vec![
            255, 0, 0, 255, //
            0, 255, 0, 255, //
            0, 0, 255, 255, //
            255, 255, 255, 255,
        ];
        clipboard::write_image(2, 2, &bytes).unwrap();

        let dictated = "VOXITYPE_TEST_IMAGE_DICTATION";
        let mut guard = ClipboardRestoreGuard::new(
            ClipboardSnapshot::Image {
                width: 2,
                height: 2,
                bytes: bytes.clone(),
            },
            dictated.to_string(),
        );
        // Injection overwrites the image with the dictated text.
        clipboard::write_text(dictated).unwrap();
        guard.restore();

        match clipboard::read_snapshot() {
            ClipboardSnapshot::Image {
                width,
                height,
                bytes: restored,
            } => {
                assert_eq!((width, height), (2, 2));
                assert_eq!(restored, bytes);
            }
            other => panic!("expected Image snapshot after restore, got {other:?}"),
        }
    }

    #[test]
    fn restore_guard_wipes_dictation_on_non_text_restore() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::{self, ClipboardSnapshot};

        let sensitive = "VOXITYPE_TEST_SENSITIVE_DICTATION";
        clipboard::write_text(sensitive).unwrap();

        let mut guard =
            ClipboardRestoreGuard::new(ClipboardSnapshot::NonText, sensitive.to_string());

        // Invariant: text must NOT be wiped before restore is invoked (paste still in progress).
        assert_eq!(clipboard::read_text().as_deref(), Some(sensitive));

        // When restore is invoked (after paste delay), sensitive text must be wiped.
        guard.restore();
        assert_eq!(clipboard::read_text().as_deref(), Some(""));
    }

    #[test]
    fn restore_guard_wipes_dictation_on_non_text_drop() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::{self, ClipboardSnapshot};

        let sensitive = "VOXITYPE_TEST_DROP_WIPE";
        clipboard::write_text(sensitive).unwrap();

        {
            let _guard =
                ClipboardRestoreGuard::new(ClipboardSnapshot::NonText, sensitive.to_string());
            // Invariant: text preserved inside scope before drop.
            assert_eq!(clipboard::read_text().as_deref(), Some(sensitive));
        }

        // On drop, fail-safe wipes text from clipboard.
        assert_eq!(clipboard::read_text().as_deref(), Some(""));
    }

    #[test]
    fn restore_skips_when_clipboard_changed_externally() {
        let _guard = CLIPBOARD_INJECTION_LOCK.lock_recover();
        use crate::injection::clipboard::{self, ClipboardSnapshot};

        let dictated = "VOXITYPE_TEST_DICTATED";
        let external = "VX_EXTERNAL";
        clipboard::write_text(dictated).unwrap();

        let guard = ClipboardRestoreGuard::new(
            ClipboardSnapshot::Text("ORIGINAL".to_string()),
            dictated.to_string(),
        );
        // An external writer (user copy, clipboard manager) replaces the dictated text.
        clipboard::write_text(external).unwrap();

        drop(guard);

        // The external content survives; the stale snapshot is not restored.
        assert_eq!(clipboard::read_text().as_deref(), Some(external));
    }
}
