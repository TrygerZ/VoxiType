//! System tray icon and menu management.

pub mod menu;

use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::error::{AppError, Result};

/// Create the tray icon with menu and click handlers.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let menu = menu::build_menu(app)?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| AppError::internal("No default window icon for tray"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("VoxiType")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                let app = tray.app_handle();
                show_main_window(app);
            }
        })
        .build(app)
        .map_err(|e| AppError::internal(format!("Tray build failed: {e}")))?;

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "record" => crate::commands::hotkey_toggle(app),
        "settings" => navigate(app, "settings"),
        "history" => navigate(app, "history"),
        "dictionary" => navigate(app, "dictionary"),
        "about" => navigate(app, "about"),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

/// Show main window and ask the frontend to navigate to a route.
fn navigate<R: Runtime>(app: &AppHandle<R>, route: &str) {
    show_main_window(app);
    let _ = tauri::Emitter::emit(app, "navigate", route);
}
