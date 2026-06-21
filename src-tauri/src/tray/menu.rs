//! Tray menu construction.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Runtime};

use crate::error::{AppError, Result};

/// Build the tray context menu.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> Result<Menu<R>> {
    let record = MenuItem::with_id(app, "record", "Start Recording", true, None::<&str>)
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

    Menu::with_items(
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
    .map_err(|e| AppError::internal(e.to_string()))
}
