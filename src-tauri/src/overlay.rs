//! Floating overlay window control.
//!
//! The `floating-widget` window is declared (hidden) in `tauri.conf.json`. This
//! module shows it near the cursor while recording/processing and hides it when
//! the app returns to idle.

use tauri::{AppHandle, Manager, Runtime};

const LABEL: &str = "floating-widget";

/// Show the overlay near the current cursor position (best-effort).
pub fn show<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(LABEL) else {
        tracing::warn!("floating-widget window not found");
        return;
    };

    if let Some(pos) = cursor_anchored_position(app, &win) {
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

/// Compute a position just below-right of the cursor, clamped to the monitor.
fn cursor_anchored_position<R: Runtime>(
    app: &AppHandle<R>,
    win: &tauri::WebviewWindow<R>,
) -> Option<tauri::PhysicalPosition<i32>> {
    let cursor = app.cursor_position().ok()?;
    let size = win.outer_size().ok()?;

    let mut x = cursor.x as i32 + 16;
    let mut y = cursor.y as i32 + 16;

    if let Ok(Some(monitor)) = win.current_monitor() {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let max_x = m_pos.x + m_size.width as i32 - size.width as i32 - 8;
        let max_y = m_pos.y + m_size.height as i32 - size.height as i32 - 8;
        x = x.min(max_x).max(m_pos.x + 8);
        y = y.min(max_y).max(m_pos.y + 8);
    }

    Some(tauri::PhysicalPosition::new(x, y))
}
