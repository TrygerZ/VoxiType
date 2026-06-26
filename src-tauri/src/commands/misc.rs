//! Microphone, hotkey, app-info, and update commands.

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime};

use crate::audio::DeviceInfo;
use crate::error::AppError;
use crate::hotkey;
use crate::AppStateInner;

#[tauri::command]
pub fn get_microphones() -> std::result::Result<Vec<DeviceInfo>, AppError> {
    crate::audio::device::list_input_devices()
}

#[tauri::command]
pub fn set_hotkey<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    mode: String,
) -> std::result::Result<(), AppError> {
    let hk_mode = match mode.as_str() {
        "toggle" => hotkey::HotkeyMode::Toggle,
        _ => hotkey::HotkeyMode::Ptt,
    };
    let cfg = hotkey::HotkeyConfig { key, mode: hk_mode };

    {
        let state = app.state::<AppStateInner>();
        crate::storage::SettingsManager::new(&state.db).set("hotkey", &cfg)?;
    }
    hotkey::rebind(&app, &cfg)
}

#[tauri::command]
pub fn get_app_info<R: Runtime>(app: AppHandle<R>) -> Value {
    serde_json::json!({
        "name": "VoxiType",
        "version": app.package_info().version.to_string(),
        "tauri": "2",
    })
}

#[tauri::command]
pub async fn check_updates<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<crate::updater::UpdateInfo, AppError> {
    let current = app.package_info().version.to_string();
    crate::updater::check(&current).await
}
