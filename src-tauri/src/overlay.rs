//! Floating overlay window control.
//!
//! The `floating-widget` window is declared (hidden) in `tauri.conf.json`. This
//! module shows it at the bottom-center of the screen while recording/processing
//! and hides it when the app returns to idle.

use tauri::{AppHandle, Manager, Runtime};

const LABEL: &str = "floating-widget";

/// Show the overlay at the bottom-center of the screen.
pub fn show<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(LABEL) else {
        tracing::warn!("floating-widget window not found");
        return;
    };

    if let Some(pos) = bottom_center_position(&win) {
        let _ = win.set_position(pos);
    }
    let _ = win.show();
    // Keep it above other windows without stealing focus.
    let _ = win.set_always_on_top(true);
}

/// Hide the overlay.
pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}

/// Compute a position at the horizontal center, near the bottom of the monitor.
fn bottom_center_position<R: Runtime>(
    win: &tauri::WebviewWindow<R>,
) -> Option<tauri::PhysicalPosition<i32>> {
    let monitor = win.current_monitor().ok()??;
    let m_pos = monitor.position();
    let m_size = monitor.size();
    let w_size = win.outer_size().ok()?;

    let x = m_pos.x + (m_size.width as i32 - w_size.width as i32) / 2;
    let y = m_pos.y + m_size.height as i32 - w_size.height as i32 - 48;

    Some(tauri::PhysicalPosition::new(x, y))
}
