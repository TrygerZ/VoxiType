//! Recording lifecycle commands (start / stop / cancel / state / audio level).

use tauri::{AppHandle, Manager, Runtime, State};

use crate::error::AppError;
use crate::{events, AppStateInner};

use super::runtime;

#[tauri::command]
pub async fn start_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    runtime::hotkey_start(&app);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    runtime::hotkey_stop(&app);
    Ok(())
}

#[tauri::command]
pub async fn cancel_recording<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    let state = app.state::<AppStateInner>();
    state.pipeline.cancel_recording()?;
    events::emit_state(&app, state.pipeline.state_tag());
    crate::overlay::maybe_hide(&app);
    Ok(())
}

#[tauri::command]
pub fn get_state(state: State<'_, AppStateInner>) -> String {
    format!("{:?}", state.pipeline.state_tag())
}

#[tauri::command]
pub fn get_audio_level(state: State<'_, AppStateInner>) -> f32 {
    state.pipeline.audio_level()
}
