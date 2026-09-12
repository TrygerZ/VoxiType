//! Global hotkey registration via tauri-plugin-global-shortcut.

pub mod config;

pub use config::{HotkeyConfig, HotkeyMode};

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, Result};

static PTT_KEY_DOWN: AtomicBool = AtomicBool::new(false);

/// Reset the PTT key-down guard flag.
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
fn parse_shortcut(accelerator: &str) -> Result<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| AppError::hotkey_conflict(format!("Invalid hotkey '{accelerator}': {e}")))
}

/// Register the global hotkey. On press/release it drives the pipeline and
/// emits state changes to the frontend.
pub fn register<R: Runtime>(app: &AppHandle<R>, cfg: &HotkeyConfig) -> Result<()> {
    reset_key_down();
    let shortcut = parse_shortcut(&cfg.key)?;
    let mode = cfg.mode;
    let gs = app.global_shortcut();

    // Unregister any previous binding to allow rebinding.
    if let Err(e) = gs.unregister_all() {
        tracing::warn!("Failed to unregister previous hotkey: {e}");
    }

    let app_handle = app.clone();
    gs.on_shortcut(shortcut, move |_app, _sc, event| {
        dispatch_shortcut_event(&app_handle, mode, event.state());
    })
    .map_err(|e| AppError::hotkey_conflict(format!("Failed to register '{}': {e}", cfg.key)))?;

    Ok(())
}

/// Rebind to a new hotkey: unregister old, register new.
pub fn rebind<R: Runtime>(app: &AppHandle<R>, cfg: &HotkeyConfig) -> Result<()> {
    register(app, cfg)
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
}
