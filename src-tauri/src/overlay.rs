//! Floating overlay window control.
//!
//! The `floating-widget` window is declared (hidden) in `tauri.conf.json`. When
//! the user enables it (setting `floating_widget`, default on) the overlay is
//! shown persistently, always-on-top and click-through-friendly, and can be
//! dragged anywhere on screen. Its position is remembered across launches
//! (setting `floating_widget_pos`). While recording/processing the widget plays
//! its live animation; when the feature is turned off the window is hidden.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, Runtime, WebviewWindow, WindowEvent};

use crate::storage::SettingsManager;
use crate::util::MutexExt;
use crate::AppStateInner;

const LABEL: &str = "floating-widget";

/// Whether the floating-widget webview has finished its first mount. The
/// overlay window is created hidden and only `show()` once the page has
/// painted its transparent content, so a blank white square never flashes over
/// the animation layer -- prominent in dev where the Vite dev server loads
/// the overlay slower than the bundled build. The frontend invokes
/// `reveal_floating_widget` (handled in commands) once React mounts.
static WIDGET_READY: AtomicBool = AtomicBool::new(false);

/// Persisted top-left position of the overlay window (physical pixels).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct WidgetPos {
    x: i32,
    y: i32,
}

/// Whether the floating widget feature is enabled (defaults to true).
pub fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    let state = app.state::<AppStateInner>();
    SettingsManager::new(&state.db)
        .get::<bool>("floating_widget")
        .ok()
        .flatten()
        .unwrap_or(true)
}

/// Apply the enabled/disabled state: show the overlay (restoring its saved
/// position) or hide it. Called on startup and whenever the user toggles it.
pub fn apply_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let Some(win) = app.get_webview_window(LABEL) else {
        tracing::warn!("floating-widget window not found");
        return;
    };
    if enabled {
        let _ = win.set_always_on_top(true);
        // Only show now if the overlay page already painted; otherwise wait for
        // `reveal_if_enabled` (called by the frontend on mount) to avoid a
        // white-square flash over the animation layer while the page loads.
        if WIDGET_READY.load(Ordering::SeqCst) {
            let _ = win.show();
        }
    } else {
        let _ = win.hide();
    }
}

/// Mark the overlay ready and reveal it if the feature is enabled. Called by
/// the `reveal_floating_widget` IPC command once the floating page mounts.
pub fn reveal_if_enabled<R: Runtime>(app: &AppHandle<R>) {
    WIDGET_READY.store(true, Ordering::SeqCst);
    if !is_enabled(app) {
        return;
    }
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    if !win.is_visible().unwrap_or(false) {
        restore_position(app, &win);
        let _ = win.show();
    }
    let _ = win.set_always_on_top(true);
}

/// Ensure the overlay is visible for an active recording/processing session.
/// No-op when the feature is disabled; never repositions an already-visible
/// window so it will not jump out from under the user's cursor.
pub fn ensure_visible<R: Runtime>(app: &AppHandle<R>) {
    if !is_enabled(app) {
        return;
    }
    // Do not force the overlay visible before its transparent content has
    // mounted; that produces a white-square flash in dev. `reveal_if_enabled`
    // handles the first show once React signals it is ready.
    if !WIDGET_READY.load(Ordering::SeqCst) {
        return;
    }
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    if !win.is_visible().unwrap_or(false) {
        restore_position(app, &win);
        let _ = win.show();
    }
    let _ = win.set_always_on_top(true);
}

/// Hide the overlay only when the feature is disabled. When enabled the widget
/// stays put (persistent) and simply returns to its idle appearance.
pub fn maybe_hide<R: Runtime>(app: &AppHandle<R>) {
    if is_enabled(app) {
        return;
    }
    if let Some(win) = app.get_webview_window(LABEL) {
        let _ = win.hide();
    }
}

/// Persist the widget's current position to settings.
pub fn persist_position<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let Ok(pos) = win.outer_position() else {
        return;
    };
    let state = app.state::<AppStateInner>();
    let _ = SettingsManager::new(&state.db)
        .set("floating_widget_pos", &WidgetPos { x: pos.x, y: pos.y });
}

/// Register a debounced listener that remembers the overlay position whenever
/// the user drags it. Only the final resting position of a drag is written
/// (300 ms quiet period), keeping DB writes minimal during a drag.
pub fn setup_persistence<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let app = app.clone();
    let pending: Arc<Mutex<WidgetPos>> = Arc::new(Mutex::new(WidgetPos { x: 0, y: 0 }));
    let generation = Arc::new(AtomicU64::new(0));

    win.on_window_event(move |event| {
        if let WindowEvent::Moved(pos) = event {
            *pending.lock_recover() = WidgetPos { x: pos.x, y: pos.y };
            let my_gen = generation.fetch_add(1, Ordering::SeqCst) + 1;

            let app = app.clone();
            let pending = pending.clone();
            let generation = generation.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(300)).await;
                // Only the last move within the quiet window writes.
                if generation.load(Ordering::SeqCst) != my_gen {
                    return;
                }
                let mut saved = *pending.lock_recover();
                let Some(win) = app.get_webview_window(LABEL) else {
                    return;
                };

                // Clamp position to monitor bounds to prevent widget from going off-screen
                let monitor = win.current_monitor().ok().flatten().or_else(|| {
                    win.available_monitors()
                        .ok()
                        .and_then(|ms| ms.into_iter().next())
                });

                if let Some(m) = monitor {
                    let m_pos = m.position();
                    let m_size = m.size();
                    if let Ok(w_size) = win.outer_size() {
                        let min_x = m_pos.x;
                        let max_x = m_pos.x + m_size.width as i32 - w_size.width as i32;
                        let min_y = m_pos.y;
                        let max_y = m_pos.y + m_size.height as i32 - w_size.height as i32;

                        let clamped_x = saved.x.clamp(min_x, max_x);
                        let clamped_y = saved.y.clamp(min_y, max_y);

                        if clamped_x != saved.x || clamped_y != saved.y {
                            saved.x = clamped_x;
                            saved.y = clamped_y;
                            let _ = win.set_position(PhysicalPosition::new(clamped_x, clamped_y));
                        }
                    }
                }

                let state = app.state::<AppStateInner>();
                let _ = SettingsManager::new(&state.db).set("floating_widget_pos", &saved);
            });
        }
    });
}

/// Restore the saved position if it still lands on a connected monitor,
/// otherwise fall back to the bottom-center of the current monitor.
fn restore_position<R: Runtime>(app: &AppHandle<R>, win: &WebviewWindow<R>) {
    let state = app.state::<AppStateInner>();
    let saved: Option<WidgetPos> = SettingsManager::new(&state.db)
        .get("floating_widget_pos")
        .ok()
        .flatten();

    if let Some(p) = saved.filter(|p| position_visible(win, p)) {
        let _ = win.set_position(PhysicalPosition::new(p.x, p.y));
    } else if let Some(pos) = bottom_center_position(win) {
        let _ = win.set_position(pos);
    }
}

/// Guard against restoring the widget off-screen (e.g. a monitor was
/// disconnected). The top-left must fall inside some connected monitor.
fn position_visible<R: Runtime>(win: &WebviewWindow<R>, p: &WidgetPos) -> bool {
    let Ok(monitors) = win.available_monitors() else {
        return false;
    };
    monitors.iter().any(|m| {
        let mp = m.position();
        let ms = m.size();
        let right = mp.x + ms.width as i32;
        let bottom = mp.y + ms.height as i32;
        p.x >= mp.x - 8 && p.x <= right - 32 && p.y >= mp.y - 8 && p.y <= bottom - 24
    })
}

/// Compute a position at the horizontal center, near the bottom of the monitor.
fn bottom_center_position<R: Runtime>(win: &WebviewWindow<R>) -> Option<PhysicalPosition<i32>> {
    let monitor = win.current_monitor().ok().flatten()?;
    let m_pos = monitor.position();
    let m_size = monitor.size();
    let w_size = win.outer_size().ok()?;

    let x = m_pos.x + (m_size.width as i32 - w_size.width as i32) / 2;
    let y = m_pos.y + m_size.height as i32 - w_size.height as i32 - 48;

    Some(PhysicalPosition::new(x, y))
}
