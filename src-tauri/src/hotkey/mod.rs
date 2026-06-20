//! Global hotkey registration via tauri-plugin-global-shortcut.

pub mod config;

pub use config::{HotkeyConfig, HotkeyMode};

use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::{AppError, Result};

/// Parse an accelerator like "Ctrl+Space" into a plugin [`Shortcut`].
fn parse_shortcut(accelerator: &str) -> Result<Shortcut> {
    accelerator
        .parse::<Shortcut>()
        .map_err(|e| AppError::hotkey_conflict(format!("Invalid hotkey '{accelerator}': {e}")))
}

/// Register the global hotkey. On press/release it drives the pipeline and
/// emits state changes to the frontend.
pub fn register<R: Runtime>(app: &AppHandle<R>, cfg: &HotkeyConfig) -> Result<()> {
    let shortcut = parse_shortcut(&cfg.key)?;
    let mode = cfg.mode;
    let gs = app.global_shortcut();

    // Unregister any previous binding to allow rebinding.
    let _ = gs.unregister_all();

    let app_handle = app.clone();
    gs.on_shortcut(shortcut, move |_app, _sc, event| {
        match (mode, event.state()) {
            (HotkeyMode::Ptt, ShortcutState::Pressed) => {
                crate::commands::hotkey_start(&app_handle);
            }
            (HotkeyMode::Ptt, ShortcutState::Released) => {
                crate::commands::hotkey_stop(&app_handle);
            }
            (HotkeyMode::Toggle, ShortcutState::Pressed) => {
                crate::commands::hotkey_toggle(&app_handle);
            }
            (HotkeyMode::Toggle, ShortcutState::Released) => {}
        }
    })
    .map_err(|e| AppError::hotkey_conflict(format!("Failed to register '{}': {e}", cfg.key)))?;

    Ok(())
}

/// Unregister all global shortcuts.
pub fn unregister_all<R: Runtime>(app: &AppHandle<R>) -> Result<()> {
    app.global_shortcut()
        .unregister_all()
        .map_err(|e| AppError::hotkey_conflict(format!("Failed to unregister: {e}")))?;
    Ok(())
}

/// Rebind to a new hotkey: unregister old, register new.
pub fn rebind<R: Runtime>(app: &AppHandle<R>, cfg: &HotkeyConfig) -> Result<()> {
    unregister_all(app)?;
    register(app, cfg)
}
