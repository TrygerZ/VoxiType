//! Clipboard access via arboard.
//!
//! One `Clipboard` is kept open for the process lifetime. On X11 the clipboard
//! contents are served by the owning process, so the handle must outlive every
//! write: dropping the last handle destroys the owner window and the target app
//! reads stale or empty data. Keeping it also lets the restored content survive
//! after injection instead of vanishing with the writer.

use std::borrow::Cow;
use std::sync::Mutex;

use arboard::{Clipboard, Error};

use crate::error::{AppError, Result};
use crate::util::MutexExt;

/// The process-wide clipboard handle. `None` until first use, and after
/// [`shutdown`].
static CLIPBOARD: Mutex<Option<Clipboard>> = Mutex::new(None);

/// Run `f` against the shared clipboard, opening it on first use.
///
/// Returns `None` when the clipboard cannot be opened; the caller maps that to
/// its own "unavailable" result.
fn with_clipboard<T>(f: impl FnOnce(&mut Clipboard) -> T) -> Option<T> {
    let mut guard = CLIPBOARD.lock_recover();
    if guard.is_none() {
        match Clipboard::new() {
            Ok(cb) => *guard = Some(cb),
            Err(e) => {
                tracing::debug!("Clipboard open failed: {e}");
                return None;
            }
        }
    }
    guard.as_mut().map(f)
}

/// Drop the shared handle so X11 can hand the current contents to a clipboard
/// manager before the process exits.
pub fn shutdown() {
    let _ = CLIPBOARD.lock_recover().take();
}

/// Snapshot of clipboard content before text injection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardSnapshot {
    /// Clipboard holds valid UTF-8 text.
    Text(String),
    /// Clipboard holds an RGBA8 image; restorable via arboard.
    Image {
        width: usize,
        height: usize,
        bytes: Vec<u8>,
    },
    /// Clipboard holds non-text content that cannot be re-serialized (HTML, files).
    NonText,
    /// Clipboard could not be accessed or contains no recognized data.
    Unavailable,
}

/// Read a snapshot of the current clipboard content.
///
/// Order matters: a clipboard may carry both text and an image (e.g. copied
/// rich content), and restoring text is cheaper and lossless, so text wins.
/// Only a clipboard with no text falls through to the image probe.
pub fn read_snapshot() -> ClipboardSnapshot {
    with_clipboard(|cb| {
        if let Ok(text) = cb.get_text() {
            return ClipboardSnapshot::Text(text);
        }
        if let Ok(image) = cb.get_image() {
            return ClipboardSnapshot::Image {
                width: image.width,
                height: image.height,
                bytes: image.bytes.into_owned(),
            };
        }
        categorize_snapshot(cb.get_text(), || probe_has_non_text(cb))
    })
    .unwrap_or(ClipboardSnapshot::Unavailable)
}

/// Check if non-text payload (image, HTML, files) exists on clipboard.
fn probe_has_non_text(cb: &mut Clipboard) -> bool {
    if cb.get_image().is_ok() {
        return true;
    }
    if cb.get().html().is_ok() {
        return true;
    }
    if let Ok(files) = cb.get().file_list() {
        if !files.is_empty() {
            return true;
        }
    }
    false
}

/// Categorize clipboard result into snapshot enum.
pub(crate) fn categorize_snapshot(
    text_result: std::result::Result<String, Error>,
    has_non_text: impl FnOnce() -> bool,
) -> ClipboardSnapshot {
    match text_result {
        Ok(text) => ClipboardSnapshot::Text(text),
        Err(Error::ConversionFailure) => ClipboardSnapshot::NonText,
        Err(Error::ContentNotAvailable) => {
            if has_non_text() {
                ClipboardSnapshot::NonText
            } else {
                ClipboardSnapshot::Unavailable
            }
        }
        Err(_) => ClipboardSnapshot::Unavailable,
    }
}

/// Read text from the clipboard.
pub fn read_text() -> Option<String> {
    match read_snapshot() {
        ClipboardSnapshot::Text(text) => Some(text),
        _ => None,
    }
}

/// Write text to the clipboard.
pub fn write_text(text: &str) -> Result<()> {
    let written = with_clipboard(|cb| cb.set_text(text.to_string()));
    match written {
        Some(Ok(())) => Ok(()),
        Some(Err(e)) => Err(AppError::injection(format!("Clipboard set failed: {e}"))),
        None => Err(AppError::injection("Clipboard open failed")),
    }
}

/// Write an RGBA8 image to the clipboard.
pub fn write_image(width: usize, height: usize, bytes: &[u8]) -> Result<()> {
    let written = with_clipboard(|cb| {
        cb.set_image(arboard::ImageData {
            width,
            height,
            bytes: Cow::Borrowed(bytes),
        })
    });
    match written {
        Some(Ok(())) => Ok(()),
        Some(Err(e)) => Err(AppError::injection(format!(
            "Clipboard set image failed: {e}"
        ))),
        None => Err(AppError::injection("Clipboard open failed")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categorize_text_success() {
        let snap = categorize_snapshot(Ok("sample".into()), || false);
        assert_eq!(snap, ClipboardSnapshot::Text("sample".into()));
    }

    #[test]
    fn categorize_conversion_failure_yields_non_text() {
        let snap = categorize_snapshot(Err(Error::ConversionFailure), || false);
        assert_eq!(snap, ClipboardSnapshot::NonText);
    }

    #[test]
    fn categorize_content_not_available_with_non_text() {
        let snap = categorize_snapshot(Err(Error::ContentNotAvailable), || true);
        assert_eq!(snap, ClipboardSnapshot::NonText);
    }

    #[test]
    fn categorize_content_not_available_without_non_text() {
        let snap = categorize_snapshot(Err(Error::ContentNotAvailable), || false);
        assert_eq!(snap, ClipboardSnapshot::Unavailable);
    }

    #[test]
    fn categorize_clipboard_occupied_yields_unavailable() {
        let snap = categorize_snapshot(Err(Error::ClipboardOccupied), || true);
        assert_eq!(snap, ClipboardSnapshot::Unavailable);
    }

    #[test]
    fn read_snapshot_returns_image_dimensions_and_bytes() {
        use crate::util::MutexExt;
        let _guard = super::super::hybrid::CLIPBOARD_INJECTION_LOCK.lock_recover();
        let original_text = read_text();
        // 2x2 RGBA8: red, green, blue, white.
        let bytes = vec![
            255, 0, 0, 255, //
            0, 255, 0, 255, //
            0, 0, 255, 255, //
            255, 255, 255, 255,
        ];
        write_image(2, 2, &bytes).unwrap();

        match read_snapshot() {
            ClipboardSnapshot::Image {
                width,
                height,
                bytes: read,
            } => {
                assert_eq!((width, height), (2, 2));
                assert_eq!(read, bytes);
            }
            other => panic!("expected Image snapshot, got {other:?}"),
        }

        // Leave the clipboard as we found it for the other clipboard tests.
        if let Some(text) = original_text {
            write_text(&text).unwrap();
        }
    }
}
