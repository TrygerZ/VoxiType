//! Tray menu construction.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Runtime};

use crate::error::{AppError, Result};

/// Build the tray context menu. Returns the menu and the "Toggle Recording"
/// item so the caller can enable/disable it as pipeline state changes.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(Menu<R>, MenuItem<R>)> {
    let record = MenuItem::with_id(app, "record", "Toggle Recording", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let history = MenuItem::with_id(app, "history", "History", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let dictionary = MenuItem::with_id(app, "dictionary", "Dictionary", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let about = MenuItem::with_id(app, "about", "About", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
        .map_err(|e| AppError::internal(e.to_string()))?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|e| AppError::internal(e.to_string()))?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|e| AppError::internal(e.to_string()))?;

    let menu = Menu::with_items(
        app,
        &[
            &record,
            &sep1,
            &settings,
            &history,
            &dictionary,
            &about,
            &sep2,
            &quit,
        ],
    )
    .map_err(|e| AppError::internal(e.to_string()))?;

    Ok((menu, record))
}

/// Whether the "Toggle Recording" item should be enabled for a serialized
/// pipeline state (`AppStateTag` is `#[serde(rename_all = "lowercase")]`).
///
/// During Processing a toggle is a silent no-op (`hotkey_toggle` ignores the
/// Processing state), so the item is disabled there and enabled everywhere
/// else. Unknown states default to enabled so a future state never leaves the
/// item permanently greyed out.
pub fn record_item_enabled(state: &str) -> bool {
    state != "processing"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_item_disabled_only_while_processing() {
        assert!(!record_item_enabled("processing"));
        assert!(record_item_enabled("idle"));
        assert!(record_item_enabled("recording"));
        assert!(record_item_enabled("error"));
    }

    #[test]
    fn record_item_enabled_for_unknown_state() {
        assert!(record_item_enabled(""));
        assert!(record_item_enabled("some-future-state"));
    }
}
