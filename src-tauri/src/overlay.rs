//! Floating overlay window control.
//!
//! The `floating-widget` window is declared (hidden) in `tauri.conf.json`. When
//! the user enables it (setting `floating_widget`, default on) the overlay is
//! shown persistently, always-on-top and click-through-friendly, and can be
//! dragged anywhere on screen. Its position is remembered across launches
//! (setting `floating_widget_pos`). While recording/processing the widget plays
//! its live animation; when the feature is turned off the window is hidden.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, Runtime, WebviewWindow, WindowEvent};

use crate::pipeline::AppStateTag;
use crate::storage::SettingsManager;
use crate::util::MutexExt;
use crate::AppStateInner;

const LABEL: &str = "floating-widget";

/// Cached overlay settings to avoid querying SQLite on every idle-monitor tick.
#[derive(Debug, Clone, Copy)]
pub struct CachedOverlaySettings {
    pub auto_hide_seconds: u64,
    pub is_enabled: bool,
    pub fetched_at: Instant,
}

pub const SETTINGS_CACHE_TTL: Duration = Duration::from_secs(5);

/// Runtime state for the floating-widget idle auto-hide timer.
#[derive(Debug, Default)]
pub struct WidgetTimerState {
    pub hidden_by_timeout: AtomicBool,
    pub is_animating_hide: AtomicBool,
    pub idle_deadline: Mutex<Option<Instant>>,
    pub hide_generation: AtomicU64,
    pub is_shutdown: AtomicBool,
    pub cached_settings: Mutex<Option<CachedOverlaySettings>>,
}

impl WidgetTimerState {
    pub fn new() -> Self {
        Self {
            hidden_by_timeout: AtomicBool::new(false),
            is_animating_hide: AtomicBool::new(false),
            idle_deadline: Mutex::new(None),
            hide_generation: AtomicU64::new(0),
            is_shutdown: AtomicBool::new(false),
            cached_settings: Mutex::new(None),
        }
    }

    pub fn shutdown(&self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }

    pub fn is_shutdown(&self) -> bool {
        self.is_shutdown.load(Ordering::SeqCst)
    }

    /// Read settings from cache if within TTL, otherwise query DB and update cache.
    pub fn get_or_refresh_settings(&self, db: &crate::storage::Database) -> (u64, bool) {
        let now = Instant::now();
        let mut cache_guard = self.cached_settings.lock_recover();
        if let Some(cached) = *cache_guard {
            if now.duration_since(cached.fetched_at) < SETTINGS_CACHE_TTL {
                return (cached.auto_hide_seconds, cached.is_enabled);
            }
        }
        let auto_hide = auto_hide_seconds(db);
        let enabled = is_enabled_from_db(db);
        *cache_guard = Some(CachedOverlaySettings {
            auto_hide_seconds: auto_hide,
            is_enabled: enabled,
            fetched_at: now,
        });
        (auto_hide, enabled)
    }

    /// Invalidate the settings cache so the next read queries SQLite.
    pub fn invalidate_settings_cache(&self) {
        *self.cached_settings.lock_recover() = None;
    }

    /// Explicitly update the cached `is_enabled` flag (e.g. when toggled via IPC).
    pub fn update_cached_enabled(&self, enabled: bool) {
        let mut cache_guard = self.cached_settings.lock_recover();
        if let Some(ref mut cached) = *cache_guard {
            cached.is_enabled = enabled;
        }
    }

    pub fn next_hide_generation(&self) -> u64 {
        self.hide_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn cancel_pending_hide(&self) {
        self.hide_generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn current_hide_generation(&self) -> u64 {
        self.hide_generation.load(Ordering::SeqCst)
    }
}

/// Query whether the floating widget is enabled directly from SQLite.
pub fn is_enabled_from_db(db: &crate::storage::Database) -> bool {
    SettingsManager::new(db)
        .get::<bool>("floating_widget")
        .ok()
        .flatten()
        .unwrap_or(true)
}

/// Whether the floating-widget webview has finished its first mount. The
/// overlay window is created hidden and only `show()` once the page has
/// painted its transparent content, so a blank white square never flashes over
/// the animation layer -- prominent in dev where the Vite dev server loads
/// the overlay slower than the bundled build. The frontend invokes
/// `reveal_floating_widget` (handled in commands) once React mounts.
static WIDGET_READY: AtomicBool = AtomicBool::new(false);

/// Persisted top-left position of the overlay window (physical pixels).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WidgetPos {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

/// Whether the floating widget feature is enabled (defaults to true).
pub fn is_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    let Some(state) = app.try_state::<AppStateInner>() else {
        return true;
    };
    state.widget_timer.get_or_refresh_settings(&state.db).1
}

/// Apply the enabled/disabled state: show the overlay (restoring its saved
/// position) or hide it. Called on startup and whenever the user toggles it.
pub fn apply_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    let Some(state) = app.try_state::<AppStateInner>() else {
        return;
    };
    state.widget_timer.update_cached_enabled(enabled);
    state
        .widget_timer
        .hidden_by_timeout
        .store(false, Ordering::SeqCst);
    state
        .widget_timer
        .is_animating_hide
        .store(false, Ordering::SeqCst);
    state.widget_timer.cancel_pending_hide();
    if enabled {
        reset_idle_timer(&state);
    }
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
            let gen = state.widget_timer.next_hide_generation();
            crate::events::emit_widget_reveal_requested(app, gen);
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
    let state = app.state::<AppStateInner>();
    state
        .widget_timer
        .hidden_by_timeout
        .store(false, Ordering::SeqCst);
    state
        .widget_timer
        .is_animating_hide
        .store(false, Ordering::SeqCst);
    state.widget_timer.cancel_pending_hide();
    reset_idle_timer(&state);
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };

    let win = win.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Small delay so WebView2 can finish setting up its transparency
        // pipeline before the window becomes visible — prevents a white
        // flash (known WebView2 limitation on Windows, tauri#14515).
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        if !win.is_visible().unwrap_or(false) {
            restore_position(&app, &win);
            let _ = win.show();
        }
        let _ = win.set_always_on_top(true);
    });
}

/// Ensure the overlay is visible for an active recording/processing session.
/// No-op when the feature is disabled; never repositions an already-visible
/// window so it will not jump out from under the user's cursor.
pub fn ensure_visible<R: Runtime>(app: &AppHandle<R>) {
    if !is_enabled(app) {
        return;
    }
    let state = app.state::<AppStateInner>();
    let was_hidden = state
        .widget_timer
        .hidden_by_timeout
        .swap(false, Ordering::SeqCst);
    let was_animating = state
        .widget_timer
        .is_animating_hide
        .swap(false, Ordering::SeqCst);
    let gen = state.widget_timer.next_hide_generation();
    reset_idle_timer(&state);
    // Do not force the overlay visible before its transparent content has
    // mounted; that produces a white-square flash in dev. `reveal_if_enabled`
    // handles the first show once React signals it is ready.
    if !WIDGET_READY.load(Ordering::SeqCst) {
        return;
    }
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };

    let win = win.clone();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        // Small delay so WebView2 can finish setting up its transparency
        // pipeline before the window becomes visible — prevents a white
        // flash (known WebView2 limitation on Windows, tauri#14515).
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

        let not_visible = !win.is_visible().unwrap_or(false);
        if not_visible {
            restore_position(&app, &win);
            let _ = win.show();
        }
        let _ = win.set_always_on_top(true);

        if was_hidden || was_animating || not_visible {
            crate::events::emit_widget_reveal_requested(&app, gen);
        }
    });
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

/// Query the configured auto-hide timeout in seconds (0 = disabled).
pub fn auto_hide_seconds(db: &crate::storage::Database) -> u64 {
    SettingsManager::new(db)
        .get::<u64>("floating_widget_auto_hide_seconds")
        .ok()
        .flatten()
        .unwrap_or(0)
}

/// Compute deadline from current time and timeout seconds.
pub fn compute_idle_deadline(now: Instant, timeout_secs: u64) -> Option<Instant> {
    if timeout_secs > 0 {
        Some(now + Duration::from_secs(timeout_secs))
    } else {
        None
    }
}

/// Pure decision function for determining whether cancelling a hide requires emitting a reveal event.
pub fn should_emit_cancellation_reveal(
    was_animating_hide: bool,
    floating_widget_enabled: bool,
    is_visible: bool,
) -> bool {
    was_animating_hide && floating_widget_enabled && is_visible
}

/// Reset the idle timeout deadline based on configured timeout seconds.
/// Returns true if an in-flight hide animation was cancelled.
pub fn reset_idle_timer(state: &AppStateInner) -> bool {
    let was_animating = state
        .widget_timer
        .is_animating_hide
        .swap(false, Ordering::SeqCst);
    state.widget_timer.cancel_pending_hide();
    let (secs, _) = state.widget_timer.get_or_refresh_settings(&state.db);
    let deadline = compute_idle_deadline(Instant::now(), secs);
    *state.widget_timer.idle_deadline.lock_recover() = deadline;
    was_animating
}

/// Reset the idle timer and reconcile frontend state if an in-flight hide was cancelled
/// while the widget is enabled and visible.
pub fn reset_idle_timer_and_reconcile<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppStateInner>();
    let was_animating = reset_idle_timer(&state);
    let enabled = is_enabled(app);
    let is_visible = app
        .get_webview_window(LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);

    if should_emit_cancellation_reveal(was_animating, enabled, is_visible) {
        let gen = state.widget_timer.current_hide_generation();
        crate::events::emit_widget_reveal_requested(app, gen);
    }
}

/// Parameters for determining floating-widget auto-hide eligibility.
#[derive(Debug, Clone, Copy)]
pub struct AutoHideParams {
    pub timeout_secs: u64,
    pub deadline: Option<Instant>,
    pub now: Instant,
    pub pipeline_state: AppStateTag,
    pub floating_widget_enabled: bool,
    pub is_visible: bool,
    pub hidden_by_timeout: bool,
    pub is_animating_hide: bool,
}

/// Pure decision function for floating-widget auto-hide.
pub fn should_auto_hide(params: AutoHideParams) -> bool {
    if params.timeout_secs == 0
        || params.hidden_by_timeout
        || params.is_animating_hide
        || !params.floating_widget_enabled
        || !params.is_visible
    {
        return false;
    }
    let Some(dl) = params.deadline else {
        return false;
    };
    if params.now < dl {
        return false;
    }
    matches!(
        params.pipeline_state,
        AppStateTag::Idle | AppStateTag::Error
    )
}

/// Pure decision function for checking whether a hide ACK or fallback can proceed.
pub fn is_hide_ack_valid(
    current_gen: u64,
    ack_gen: u64,
    pipeline_state: AppStateTag,
    is_animating_hide: bool,
) -> bool {
    current_gen == ack_gen
        && is_animating_hide
        && matches!(pipeline_state, AppStateTag::Idle | AppStateTag::Error)
}

fn resolve_deadline(state: &AppStateInner, timeout_secs: u64) -> Option<Instant> {
    if timeout_secs == 0 {
        return None;
    }
    let mut guard = state.widget_timer.idle_deadline.lock_recover();
    Some(*guard.get_or_insert_with(|| Instant::now() + Duration::from_secs(timeout_secs)))
}

fn guard_and_hide<R: Runtime>(state: &AppStateInner, win: &WebviewWindow<R>) -> bool {
    let is_idle_or_err = |s: &AppStateInner| {
        matches!(
            s.pipeline.state_tag(),
            AppStateTag::Idle | AppStateTag::Error
        )
    };
    if !is_idle_or_err(state) {
        state
            .widget_timer
            .is_animating_hide
            .store(false, Ordering::SeqCst);
        return false;
    }
    state
        .widget_timer
        .hidden_by_timeout
        .store(true, Ordering::SeqCst);
    state
        .widget_timer
        .is_animating_hide
        .store(false, Ordering::SeqCst);
    if !is_idle_or_err(state) {
        state
            .widget_timer
            .hidden_by_timeout
            .store(false, Ordering::SeqCst);
        return false;
    }
    let _ = win.hide();
    true
}

/// Request an animated auto-hide from the frontend, with a fallback timeout in case the frontend
/// fails to acknowledge within 1000ms.
pub fn request_animated_hide<R: Runtime>(
    app: &AppHandle<R>,
    state: &AppStateInner,
    win: &WebviewWindow<R>,
) -> bool {
    let is_idle_or_err = matches!(
        state.pipeline.state_tag(),
        AppStateTag::Idle | AppStateTag::Error
    );
    if !is_idle_or_err {
        return false;
    }
    state
        .widget_timer
        .is_animating_hide
        .store(true, Ordering::SeqCst);
    let gen = state.widget_timer.next_hide_generation();

    crate::events::emit_widget_hide_requested(app, gen);

    let app_clone = app.clone();
    let win_clone = win.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1000)).await;
        let state = app_clone.state::<AppStateInner>();
        if is_hide_ack_valid(
            state.widget_timer.current_hide_generation(),
            gen,
            state.pipeline.state_tag(),
            state.widget_timer.is_animating_hide.load(Ordering::SeqCst),
        ) && guard_and_hide(&state, &win_clone)
        {
            tracing::debug!("Floating widget auto-hidden via fallback timeout (gen={gen})");
        }
    });

    true
}

/// Acknowledge from the frontend that the hide animation has finished.
pub fn acknowledge_hide<R: Runtime>(app: &AppHandle<R>, gen: u64) -> bool {
    let state = app.state::<AppStateInner>();
    if !is_hide_ack_valid(
        state.widget_timer.current_hide_generation(),
        gen,
        state.pipeline.state_tag(),
        state.widget_timer.is_animating_hide.load(Ordering::SeqCst),
    ) {
        return false;
    }
    let Some(win) = app.get_webview_window(LABEL) else {
        return false;
    };
    if guard_and_hide(&state, &win) {
        tracing::debug!("Floating widget auto-hidden via animation ACK (gen={gen})");
        true
    } else {
        false
    }
}

fn check_and_auto_hide<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app.try_state::<AppStateInner>() else {
        return;
    };
    if state.widget_timer.hidden_by_timeout.load(Ordering::SeqCst)
        || state.widget_timer.is_animating_hide.load(Ordering::SeqCst)
    {
        return;
    }
    let (timeout_secs, enabled) = state.widget_timer.get_or_refresh_settings(&state.db);
    let deadline = resolve_deadline(&state, timeout_secs);
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };

    let eligible = should_auto_hide(AutoHideParams {
        timeout_secs,
        deadline,
        now: Instant::now(),
        pipeline_state: state.pipeline.state_tag(),
        floating_widget_enabled: enabled,
        is_visible: win.is_visible().unwrap_or(false),
        hidden_by_timeout: false,
        is_animating_hide: false,
    });
    if eligible && request_animated_hide(app, &state, &win) {
        tracing::debug!("Floating widget auto-hide animation requested after {timeout_secs}s idle");
    }
}

/// Spawn the background task that checks every second whether the widget should auto-hide.
pub fn start_idle_monitor<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    let Some(state) = app.try_state::<AppStateInner>() else {
        return;
    };
    reset_idle_timer(&state);

    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            let Some(state) = app.try_state::<AppStateInner>() else {
                break;
            };
            if state.widget_timer.is_shutdown() {
                break;
            }
            check_and_auto_hide(&app);
        }
    });
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

fn clamp_and_persist_position<R: Runtime>(app: &AppHandle<R>, mut saved: WidgetPos) {
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

    let Some(state) = app.try_state::<AppStateInner>() else {
        return;
    };
    let _ = SettingsManager::new(&state.db).set("floating_widget_pos", &saved);
}

/// Single watchdog task that debounces drag events by resetting a deadline timer.
pub(crate) async fn run_position_watchdog<F>(
    mut rx: tokio::sync::watch::Receiver<Option<WidgetPos>>,
    quiet_period: Duration,
    mut persist_fn: F,
) where
    F: FnMut(WidgetPos),
{
    let mut pending: Option<WidgetPos> = None;
    loop {
        if pending.is_some() {
            tokio::select! {
                res = rx.changed() => {
                    if res.is_err() {
                        break;
                    }
                    pending = *rx.borrow_and_update();
                }
                _ = tokio::time::sleep(quiet_period) => {
                    if let Some(pos) = pending.take() {
                        persist_fn(pos);
                    }
                }
            }
        } else {
            if rx.changed().await.is_err() {
                break;
            }
            pending = *rx.borrow_and_update();
        }
    }
}

/// Register a debounced listener that remembers the overlay position whenever
/// the user drags it. Only the final resting position of a drag is written
/// (300 ms quiet period) using a single watchdog task rather than spawning per event.
pub fn setup_persistence<R: Runtime>(app: &AppHandle<R>) {
    let Some(win) = app.get_webview_window(LABEL) else {
        return;
    };
    let app = app.clone();
    let (tx, rx) = tokio::sync::watch::channel(None::<WidgetPos>);

    tauri::async_runtime::spawn(async move {
        run_position_watchdog(rx, Duration::from_millis(300), move |pos| {
            clamp_and_persist_position(&app, pos);
        })
        .await;
    });

    win.on_window_event(move |event| {
        if let WindowEvent::Moved(pos) = event {
            let _ = tx.send(Some(WidgetPos { x: pos.x, y: pos.y }));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn test_params(now: Instant, deadline: Option<Instant>, tag: AppStateTag) -> AutoHideParams {
        AutoHideParams {
            timeout_secs: 5,
            deadline,
            now,
            pipeline_state: tag,
            floating_widget_enabled: true,
            is_visible: true,
            hidden_by_timeout: false,
            is_animating_hide: false,
        }
    }

    #[test]
    fn compute_idle_deadline_calculates_correctly() {
        let now = Instant::now();
        assert_eq!(compute_idle_deadline(now, 0), None);
        assert_eq!(
            compute_idle_deadline(now, 5),
            Some(now + Duration::from_secs(5))
        );
    }

    #[test]
    fn should_auto_hide_requires_positive_timeout() {
        let now = Instant::now();
        let past = now - Duration::from_secs(1);
        let mut p = test_params(now, Some(past), AppStateTag::Idle);
        p.timeout_secs = 0;
        assert!(!should_auto_hide(p));
    }

    #[test]
    fn should_auto_hide_requires_deadline_to_pass() {
        let now = Instant::now();
        let future = now + Duration::from_secs(10);
        let mut p = test_params(now, Some(future), AppStateTag::Idle);
        p.timeout_secs = 10;
        assert!(!should_auto_hide(p));

        let p_none = test_params(now, None, AppStateTag::Idle);
        assert!(!should_auto_hide(p_none));
    }

    #[test]
    fn should_auto_hide_only_in_idle_or_error() {
        let now = Instant::now();
        let past = now - Duration::from_secs(1);
        assert!(should_auto_hide(test_params(
            now,
            Some(past),
            AppStateTag::Idle
        )));
        assert!(should_auto_hide(test_params(
            now,
            Some(past),
            AppStateTag::Error
        )));
        assert!(!should_auto_hide(test_params(
            now,
            Some(past),
            AppStateTag::Recording
        )));
        assert!(!should_auto_hide(test_params(
            now,
            Some(past),
            AppStateTag::Processing
        )));
    }

    #[test]
    fn should_auto_hide_requires_enabled_and_visible_and_unhidden() {
        let now = Instant::now();
        let past = now - Duration::from_secs(1);

        let mut p = test_params(now, Some(past), AppStateTag::Idle);
        p.floating_widget_enabled = false;
        assert!(!should_auto_hide(p));

        let mut p = test_params(now, Some(past), AppStateTag::Idle);
        p.is_visible = false;
        assert!(!should_auto_hide(p));

        let mut p = test_params(now, Some(past), AppStateTag::Idle);
        p.hidden_by_timeout = true;
        assert!(!should_auto_hide(p));

        let p = test_params(now, Some(past), AppStateTag::Idle);
        assert!(should_auto_hide(p));
    }

    #[test]
    fn should_auto_hide_rejects_when_animating_hide() {
        let now = Instant::now();
        let past = now - Duration::from_secs(1);
        let mut p = test_params(now, Some(past), AppStateTag::Idle);
        p.is_animating_hide = true;
        assert!(!should_auto_hide(p));
    }

    #[test]
    fn widget_timer_state_generations_and_cancellation() {
        let state = WidgetTimerState::new();
        assert_eq!(state.current_hide_generation(), 0);
        let g1 = state.next_hide_generation();
        assert_eq!(g1, 1);
        assert_eq!(state.current_hide_generation(), 1);
        state.cancel_pending_hide();
        assert_eq!(state.current_hide_generation(), 2);
    }

    #[test]
    fn is_hide_ack_valid_logic() {
        assert!(is_hide_ack_valid(1, 1, AppStateTag::Idle, true));
        assert!(is_hide_ack_valid(1, 1, AppStateTag::Error, true));
        // Stale generation
        assert!(!is_hide_ack_valid(2, 1, AppStateTag::Idle, true));
        // Not currently animating
        assert!(!is_hide_ack_valid(1, 1, AppStateTag::Idle, false));
        // In recording or processing
        assert!(!is_hide_ack_valid(1, 1, AppStateTag::Recording, true));
        assert!(!is_hide_ack_valid(1, 1, AppStateTag::Processing, true));
    }

    #[test]
    fn widget_timer_state_initial_values() {
        let state = WidgetTimerState::new();
        assert!(!state.hidden_by_timeout.load(Ordering::SeqCst));
        assert!(!state.is_animating_hide.load(Ordering::SeqCst));
        assert_eq!(state.current_hide_generation(), 0);
        assert_eq!(*state.idle_deadline.lock_recover(), None);
    }

    #[test]
    fn auto_hide_seconds_reads_from_database() {
        let db = crate::storage::Database::open_in_memory().unwrap();
        assert_eq!(auto_hide_seconds(&db), 0);
        SettingsManager::new(&db)
            .set("floating_widget_auto_hide_seconds", &15u64)
            .unwrap();
        assert_eq!(auto_hide_seconds(&db), 15);
    }

    #[test]
    fn widget_timer_state_hidden_by_timeout_toggle() {
        let state = WidgetTimerState::new();
        state.hidden_by_timeout.store(true, Ordering::SeqCst);
        assert!(state.hidden_by_timeout.load(Ordering::SeqCst));
        state.hidden_by_timeout.store(false, Ordering::SeqCst);
        assert!(!state.hidden_by_timeout.load(Ordering::SeqCst));
    }

    #[test]
    fn should_emit_cancellation_reveal_logic() {
        // Only emit reveal when an in-flight hide animation was cancelled and window is visible + enabled
        assert!(should_emit_cancellation_reveal(true, true, true));
        // Normal reset (not animating hide) must NOT emit reveal
        assert!(!should_emit_cancellation_reveal(false, true, true));
        // Disabled widget must NOT emit reveal
        assert!(!should_emit_cancellation_reveal(true, false, true));
        // Hidden/non-visible window must NOT emit reveal
        assert!(!should_emit_cancellation_reveal(true, true, false));
    }

    #[test]
    fn widget_timer_shutdown_flag_works() {
        let state = WidgetTimerState::new();
        assert!(!state.is_shutdown());
        state.shutdown();
        assert!(state.is_shutdown());
    }

    #[test]
    fn settings_cache_reduces_queries_and_respects_updates() {
        let db = crate::storage::Database::open_in_memory().unwrap();
        let timer = WidgetTimerState::new();

        // Initial read fetches defaults from DB.
        let (timeout, enabled) = timer.get_or_refresh_settings(&db);
        assert_eq!(timeout, 0);
        assert!(enabled);

        // Update DB directly.
        SettingsManager::new(&db)
            .set("floating_widget_auto_hide_seconds", &30u64)
            .unwrap();
        SettingsManager::new(&db)
            .set("floating_widget", &false)
            .unwrap();

        // Immediate subsequent read uses cache, reducing queries.
        let (cached_timeout, cached_enabled) = timer.get_or_refresh_settings(&db);
        assert_eq!(cached_timeout, 0);
        assert!(cached_enabled);

        // Explicit enable update updates the cache immediately.
        timer.update_cached_enabled(false);
        assert!(!timer.get_or_refresh_settings(&db).1);

        // Invalidation forces reload from DB.
        timer.invalidate_settings_cache();
        let (reloaded_timeout, reloaded_enabled) = timer.get_or_refresh_settings(&db);
        assert_eq!(reloaded_timeout, 30);
        assert!(!reloaded_enabled);
    }

    #[tokio::test]
    async fn run_position_watchdog_debounces_burst_into_single_write() {
        let (tx, rx) = tokio::sync::watch::channel(None::<WidgetPos>);
        let persisted = Arc::new(Mutex::new(Vec::new()));
        let persisted_clone = persisted.clone();

        let watchdog = tokio::spawn(async move {
            run_position_watchdog(rx, Duration::from_millis(50), move |pos| {
                persisted_clone.lock_recover().push(pos);
            })
            .await;
        });

        // Rapid burst of 10 position updates.
        for i in 1..=10 {
            tx.send(Some(WidgetPos {
                x: i * 10,
                y: i * 20,
            }))
            .unwrap();
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Wait for quiet window (50 ms) to expire.
        tokio::time::sleep(Duration::from_millis(100)).await;

        let writes = persisted.lock_recover().clone();
        assert_eq!(writes.len(), 1, "Burst of moves must yield exactly 1 write");
        assert_eq!(writes[0].x, 100);
        assert_eq!(writes[0].y, 200);

        // A second burst after quiet window produces a second single write.
        tx.send(Some(WidgetPos { x: 500, y: 600 })).unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;

        let writes2 = persisted.lock_recover().clone();
        assert_eq!(writes2.len(), 2);
        assert_eq!(writes2[1].x, 500);
        assert_eq!(writes2[1].y, 600);

        drop(tx);
        let _ = watchdog.await;
    }
}
