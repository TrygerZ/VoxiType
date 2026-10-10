//! System tray icon and menu management.

pub mod menu;

use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Listener, Manager, Runtime};

use crate::error::{AppError, Result};

/// Create the tray icon with menu and click handlers.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    let (menu, record_item) = menu::build_menu(app)?;
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
            if let TrayIconEvent::Click { button, .. } = event {
                if button == MouseButton::Left {
                    let app = tray.app_handle();
                    show_main_window(app);
                }
            }
        })
        .build(app)
        .map_err(|e| AppError::internal(format!("Tray build failed: {e}")))?;

    // Keep "Toggle Recording" in sync with pipeline state: it is disabled while
    // Processing, where a toggle is a silent no-op (`hotkey_toggle` ignores the
    // Processing state). The listener lives for the app lifetime (like the tray
    // itself), so no cleanup is required.
    let listener_app = app.clone();
    app.listen("state_changed", move |event| {
        // A malformed payload must never leave the item stuck disabled.
        let enabled = serde_json::from_str::<serde_json::Value>(event.payload())
            .ok()
            .and_then(|value| {
                value
                    .get("state")
                    .and_then(|state| state.as_str())
                    .map(menu::record_item_enabled)
            })
            .unwrap_or(true);
        let item = record_item.clone();
        // Never marshal to the main thread inline here: this callback runs on the
        // emitting thread (hotkey dispatch holds the plugin's shortcuts mutex at
        // that point), and a blocking marshal would invert lock order with a
        // concurrent rebind. Queueing is non-blocking.
        if let Err(e) = listener_app.run_on_main_thread(move || {
            if let Err(e) = item.set_enabled(enabled) {
                tracing::warn!("Failed to update tray record item: {e}");
            }
        }) {
            tracing::warn!("Failed to queue tray record item update: {e}");
        }
    });

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "record" => crate::commands::runtime::hotkey_toggle(app),
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
