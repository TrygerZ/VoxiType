//! Global hotkey registration via tauri-plugin-global-shortcut.

pub mod config;

pub use config::{HotkeyConfig, HotkeyMode};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, Result};
use crate::util::MutexExt;

static PTT_KEY_DOWN: AtomicBool = AtomicBool::new(false);

/// Serializes PTT event evaluation against hotkey rebinds.
///
/// `dispatch_shortcut_event` holds this lock for the whole evaluate-and-dispatch
/// sequence, and the rebind functions hold it across their register/unregister/
/// persist swap. Without it a PTT event could evaluate against a half-swapped
/// binding (old handler already gone, new one not yet active) and be treated as
/// a fresh press. Key events are human-paced, so contention is nil.
static PTT_EVENT_LOCK: Mutex<()> = Mutex::new(());

/// Reset the PTT key-down guard flag.
///
/// Plain store: callers already holding [`PTT_EVENT_LOCK`] must not re-lock, so
/// this never acquires the event lock.
pub fn reset_key_down() {
    PTT_KEY_DOWN.store(false, Ordering::SeqCst);
}

/// Commands produced by the PTT hotkey event evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PttCommand {
    Start,
    Stop,
}

/// Evaluates a PTT shortcut event against key state and recording status.
///
/// Returns `Some(PttCommand)` when a transition must execute, or `None` if redundant.
///
/// # Rationale: Lost-Release Recovery
/// If OS focus shifts while held, `Released` may never be sent by the OS. When the recording
/// later finishes (via UI stop or timeout), the pipeline returns to Idle while `key_down`
/// remains `true`. When a new `Pressed` arrives and `!is_recording`, `ptt_action` self-heals
/// by accepting the press and re-asserting `key_down = true`.
pub fn ptt_action(
    key_down: &AtomicBool,
    event: ShortcutState,
    is_recording: bool,
) -> Option<PttCommand> {
    match event {
        ShortcutState::Pressed => {
            let was_down = key_down.swap(true, Ordering::SeqCst);
            if was_down && is_recording {
                // Ignore repeat press events while already holding and actively recording.
                None
            } else {
                Some(PttCommand::Start)
            }
        }
        ShortcutState::Released => {
            let was_down = key_down.swap(false, Ordering::SeqCst);
            if was_down && is_recording {
                Some(PttCommand::Stop)
            } else {
                // Ignore phantom/duplicate releases or releases after recording stopped externally.
                None
            }
        }
    }
}

fn dispatch_shortcut_event<R: Runtime>(app: &AppHandle<R>, mode: HotkeyMode, state: ShortcutState) {
    // Held across the whole evaluate + dispatch so a rebind cannot swap the
    // binding between the key-state check and the resulting start/stop.
    // Deadlock check: hotkey_start/hotkey_stop do not re-enter hotkey dispatch,
    // and the rebind path only calls this lock, never dispatch.
    let _guard = PTT_EVENT_LOCK.lock_recover();
    match mode {
        HotkeyMode::Ptt => {
            let is_recording = app.state::<crate::AppStateInner>().pipeline.state_tag()
                == crate::pipeline::AppStateTag::Recording;

            if let Some(cmd) = ptt_action(&PTT_KEY_DOWN, state, is_recording) {
                match cmd {
                    PttCommand::Start => crate::commands::runtime::hotkey_start(app),
                    PttCommand::Stop => crate::commands::runtime::hotkey_stop(app),
                }
            }
        }
        HotkeyMode::Toggle => {
            // Mode Toggle only responds to Pressed. Kept without extra dither guard to
            // avoid altering toggle semantics; pipeline state tag already gates transitions.
            if state == ShortcutState::Pressed {
                crate::commands::runtime::hotkey_toggle(app);
            }
        }
    }
}

/// Parse an accelerator like "Ctrl+Space" into a plugin [`Shortcut`].
pub fn parse_shortcut(accelerator: &str) -> Result<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| AppError::hotkey_conflict(format!("Invalid hotkey '{accelerator}': {e}")))
}

/// Check whether two accelerator strings represent identical key combinations.
pub fn accelerators_equal(a: &str, b: &str) -> bool {
    match (parse_shortcut(a), parse_shortcut(b)) {
        (Ok(sc_a), Ok(sc_b)) => sc_a == sc_b,
        _ => false,
    }
}

fn register_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    shortcut: Shortcut,
    cfg: &HotkeyConfig,
) -> Result<()> {
    let mode = cfg.mode;
    let gs = app.global_shortcut();
    let app_handle = app.clone();
    gs.on_shortcut(shortcut, move |_app, _sc, event| {
        dispatch_shortcut_event(&app_handle, mode, event.state());
    })
    .map_err(|e| AppError::hotkey_conflict(format!("Failed to register '{}': {e}", cfg.key)))
}

fn unregister_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: Shortcut) -> Result<()> {
    let gs = app.global_shortcut();
    if gs.is_registered(shortcut) {
        gs.unregister(shortcut).map_err(|e| {
            AppError::hotkey_conflict(format!("Failed to unregister shortcut: {e}"))
        })?;
    }
    Ok(())
}

/// Register the global hotkey on startup.
pub fn register<R: Runtime>(app: &AppHandle<R>, cfg: &HotkeyConfig) -> Result<()> {
    // The flag is false at startup, but clear it explicitly so a restart path
    // that reuses this function never inherits a stale key-down.
    reset_key_down();
    let shortcut = parse_shortcut(&cfg.key)?;
    register_shortcut(app, shortcut, cfg)
}

/// Whether the accelerator is currently registered with the plugin. Returns
/// `false` when the accelerator string is invalid.
pub fn is_registered<R: Runtime>(app: &AppHandle<R>, accelerator: &str) -> bool {
    parse_shortcut(accelerator)
        .map(|shortcut| app.global_shortcut().is_registered(shortcut))
        .unwrap_or(false)
}

/// Unregister `old` and verify it is gone; retry once, then fail.
///
/// A failed `unregister` call does not guarantee the binding survived, so the
/// authoritative check is `is_registered`. If the binding is still present
/// after two attempts the old hotkey would fire alongside the new one, so this
/// reports a conflict instead of silently leaving both live.
fn ensure_unregistered(
    mut unregister: impl FnMut() -> Result<()>,
    is_registered: impl Fn() -> bool,
) -> Result<()> {
    if let Err(e) = unregister() {
        tracing::warn!("Unregister reported an error; verifying binding state: {e}");
    }
    if !is_registered() {
        return Ok(());
    }

    if let Err(e) = unregister() {
        tracing::warn!("Unregister retry reported an error; verifying binding state: {e}");
    }
    if is_registered() {
        return Err(AppError::hotkey_conflict(
            "Failed to unregister the previous hotkey; it is still active",
        ));
    }
    Ok(())
}

/// Rebind to a candidate hotkey using a transactional register-then-swap pattern:
/// 1. If candidate accelerator is identical to old:
///    - If mode is identical: no-op success.
///    - If mode changed: swap handler and rollback if registration fails.
/// 2. If candidate accelerator is different:
///    - Register candidate first (old remains active on failure).
///    - Unregister old shortcut.
///    - Persist to DB.
///    - On persist failure: rollback by unregistering candidate and restoring old.
pub fn rebind<R: Runtime, F>(
    app: &AppHandle<R>,
    new_cfg: &HotkeyConfig,
    old_cfg: Option<&HotkeyConfig>,
    persist: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    let new_sc = parse_shortcut(&new_cfg.key)?;
    let old_sc = old_cfg.and_then(|c| parse_shortcut(&c.key).ok());

    if Some(new_sc) == old_sc {
        return rebind_identical_accelerator(app, new_sc, new_cfg, old_cfg, persist);
    }

    rebind_swap_with_rollback(app, new_sc, new_cfg, old_sc, old_cfg, persist)
}

fn rebind_identical_accelerator<R: Runtime, F>(
    app: &AppHandle<R>,
    shortcut: Shortcut,
    new_cfg: &HotkeyConfig,
    old_cfg: Option<&HotkeyConfig>,
    persist: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    // Hold the event lock across the whole swap so no PTT event can evaluate
    // against a half-swapped binding. Deadlock check: register_shortcut,
    // unregister_shortcut, and persist never take this lock or re-enter
    // dispatch.
    let _guard = PTT_EVENT_LOCK.lock_recover();

    let mode_changed = old_cfg.is_some_and(|old| old.mode != new_cfg.mode);
    if !mode_changed {
        // No binding change: the key-down flag stays as it was.
        return persist();
    }

    // Unbind existing handler before rebinding with updated mode.
    unregister_shortcut(app, shortcut)?;
    if let Err(e) = register_shortcut(app, shortcut, new_cfg) {
        if let Some(old) = old_cfg {
            let _ = register_shortcut(app, shortcut, old);
        }
        return Err(e);
    }

    if let Err(e) = persist() {
        let _ = unregister_shortcut(app, shortcut);
        if let Some(old) = old_cfg {
            let _ = register_shortcut(app, shortcut, old);
        }
        return Err(e);
    }
    // Success: the new binding is the only live one, so clear the stale flag.
    // On rollback above the previous binding and flag are left untouched.
    reset_key_down();
    Ok(())
}

fn rebind_swap_with_rollback<R: Runtime, F>(
    app: &AppHandle<R>,
    new_sc: Shortcut,
    new_cfg: &HotkeyConfig,
    old_sc: Option<Shortcut>,
    old_cfg: Option<&HotkeyConfig>,
    persist: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    // Hold the event lock across the whole swap so no PTT event can evaluate
    // against a half-swapped binding. Deadlock check: register_shortcut,
    // unregister_shortcut, and persist never take this lock or re-enter
    // dispatch.
    let _guard = PTT_EVENT_LOCK.lock_recover();

    // Step 1: Register candidate first. Old binding is retained if this fails.
    register_shortcut(app, new_sc, new_cfg)?;

    // Step 2: Candidate registered successfully; unregister old binding.
    if let Some(old) = old_sc {
        if let Err(e) = ensure_unregistered(
            || unregister_shortcut(app, old),
            || app.global_shortcut().is_registered(old),
        ) {
            // Rollback: drop the candidate so only the previous binding remains.
            let _ = unregister_shortcut(app, new_sc);
            return Err(e);
        }
    }

    // Step 3: Persist new setting to storage.
    if let Err(save_err) = persist() {
        // Rollback: remove candidate and restore old binding.
        let _ = unregister_shortcut(app, new_sc);
        if let (Some(old), Some(cfg)) = (old_sc, old_cfg) {
            let _ = register_shortcut(app, old, cfg);
        }
        return Err(save_err);
    }

    // Success: the new binding is the only live one, so clear the stale flag.
    // On rollback above the previous binding and flag are left untouched.
    reset_key_down();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ptt_pressed_released_pressed_cycle() {
        let key_down = AtomicBool::new(false);

        // First press -> Start
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );
        assert!(key_down.load(Ordering::SeqCst));

        // Release while recording -> Stop
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Released, true),
            Some(PttCommand::Stop)
        );
        assert!(!key_down.load(Ordering::SeqCst));

        // Second press -> Start
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );
        assert!(key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_duplicate_pressed_ignored() {
        let key_down = AtomicBool::new(false);

        // Initial press
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );

        // Duplicate press while still down and recording -> ignored
        assert_eq!(ptt_action(&key_down, ShortcutState::Pressed, true), None);
        assert_eq!(ptt_action(&key_down, ShortcutState::Pressed, true), None);
        assert!(key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_released_before_pressed_ignored() {
        let key_down = AtomicBool::new(false);

        // Ghost release before any press -> ignored
        assert_eq!(ptt_action(&key_down, ShortcutState::Released, false), None);
        assert!(!key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_duplicate_released_ignored() {
        let key_down = AtomicBool::new(false);

        // Press -> Start
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );

        // First release -> Stop
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Released, true),
            Some(PttCommand::Stop)
        );
        assert!(!key_down.load(Ordering::SeqCst));

        // Duplicate release -> ignored
        assert_eq!(ptt_action(&key_down, ShortcutState::Released, false), None);
        assert!(!key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_rapid_cycles() {
        let key_down = AtomicBool::new(false);

        for _ in 0..10 {
            assert_eq!(
                ptt_action(&key_down, ShortcutState::Pressed, false),
                Some(PttCommand::Start)
            );
            assert_eq!(
                ptt_action(&key_down, ShortcutState::Released, true),
                Some(PttCommand::Stop)
            );
        }
        assert!(!key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_lost_release_recovery() {
        let key_down = AtomicBool::new(false);

        // Press starts recording
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );
        assert!(key_down.load(Ordering::SeqCst));

        // Pipeline stops externally (UI stop or timeout), is_recording becomes false,
        // but OS lost the Released event, so key_down is still true.
        // Next Press arrives: should recover and start recording, not get stuck.
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );
        assert!(key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ptt_delayed_release_after_external_stop() {
        let key_down = AtomicBool::new(false);

        // Press starts recording
        assert_eq!(
            ptt_action(&key_down, ShortcutState::Pressed, false),
            Some(PttCommand::Start)
        );

        // Recording stops externally (is_recording = false).
        // Delayed Released arrives: should NOT emit Stop, but should clear key_down.
        assert_eq!(ptt_action(&key_down, ShortcutState::Released, false), None);
        assert!(!key_down.load(Ordering::SeqCst));
    }

    #[test]
    fn test_parse_shortcut_valid() {
        assert!(parse_shortcut("Ctrl+Space").is_ok());
        assert!(parse_shortcut("Alt+Shift+F1").is_ok());
    }

    #[test]
    fn test_parse_shortcut_invalid() {
        let res = parse_shortcut("InvalidNonExistentKey");
        assert!(matches!(
            res,
            Err(AppError {
                code: crate::error::ErrorCode::HotkeyConflict,
                ..
            })
        ));
    }

    #[test]
    fn test_accelerators_equal_identical_and_case() {
        assert!(accelerators_equal("Ctrl+Space", "Ctrl+Space"));
        assert!(accelerators_equal("Ctrl+Space", "ctrl+space"));
        assert!(accelerators_equal("Alt+Shift+A", "Shift+Alt+A"));
    }

    #[test]
    fn test_accelerators_equal_different() {
        assert!(!accelerators_equal("Ctrl+Space", "Alt+Space"));
        assert!(!accelerators_equal("Ctrl+Space", "Ctrl+Shift+Space"));
    }

    #[test]
    fn test_accelerators_equal_invalid() {
        assert!(!accelerators_equal("InvalidKey", "Ctrl+Space"));
        assert!(!accelerators_equal("InvalidKey1", "InvalidKey2"));
    }

    #[test]
    fn ensure_unregistered_recovers_after_first_error() {
        use std::cell::Cell;
        let registered = Cell::new(true);
        let calls = Cell::new(0);
        let result = ensure_unregistered(
            || {
                calls.set(calls.get() + 1);
                if calls.get() == 1 {
                    // First attempt errors and leaves the binding registered,
                    // forcing the retry.
                    return Err(AppError::hotkey_conflict("boom"));
                }
                registered.set(false);
                Ok(())
            },
            || registered.get(),
        );
        assert!(result.is_ok());
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn ensure_unregistered_retries_once_then_fails() {
        use std::cell::Cell;
        let registered = Cell::new(true);
        let calls = Cell::new(0);
        let result = ensure_unregistered(
            || {
                calls.set(calls.get() + 1);
                Err(AppError::hotkey_conflict("boom"))
            },
            || registered.get(),
        );
        assert!(matches!(
            result,
            Err(AppError {
                code: crate::error::ErrorCode::HotkeyConflict,
                ..
            })
        ));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn ensure_unregistered_ok_when_gone_first_try() {
        use std::cell::Cell;
        let registered = Cell::new(true);
        let calls = Cell::new(0);
        let result = ensure_unregistered(
            || {
                calls.set(calls.get() + 1);
                registered.set(false);
                Ok(())
            },
            || registered.get(),
        );
        assert!(result.is_ok());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn ensure_unregistered_ok_when_error_but_binding_gone() {
        use std::cell::Cell;
        let calls = Cell::new(0);
        let result = ensure_unregistered(
            || {
                calls.set(calls.get() + 1);
                Err(AppError::hotkey_conflict("boom"))
            },
            || false,
        );
        assert!(result.is_ok());
        assert_eq!(calls.get(), 1);
    }
}
