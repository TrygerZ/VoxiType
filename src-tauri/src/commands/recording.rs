//! Recording lifecycle commands.

use tauri::{AppHandle, Runtime};

use crate::error::AppError;

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
