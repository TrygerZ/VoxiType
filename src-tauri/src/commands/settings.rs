//! Settings commands.

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::error::AppError;
use crate::storage::SettingsManager;
use crate::AppStateInner;

#[tauri::command]
pub fn get_settings(state: State<'_, AppStateInner>) -> std::result::Result<Value, AppError> {
    let mut all = SettingsManager::new(&state.db).all()?;
    // Never leak secrets to the frontend.
    if let Value::Object(map) = &mut all {
        let has_key = map
            .get("groq_api_key")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        map.insert("groq_api_key".to_string(), Value::String(String::new()));
        map.insert("groq_api_key_set".to_string(), Value::Bool(has_key));
    }
    Ok(all)
}

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppStateInner>,
    key: String,
    value: Value,
) -> std::result::Result<(), AppError> {
    let encoded = if key == "groq_api_key" {
        let plain = value.as_str().unwrap_or_default();
        let enc = crate::crypto::encrypt_api_key(plain, &state.master_key)?;
        serde_json::to_string(&enc)?
    } else {
        serde_json::to_string(&value)?
    };
    SettingsManager::new(&state.db).set_raw(&key, &encoded)
}

#[tauri::command]
pub fn set_floating_widget_enabled<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> std::result::Result<(), AppError> {
    let state = app.state::<AppStateInner>();
    SettingsManager::new(&state.db).set("floating_widget", &enabled)?;
    crate::overlay::apply_enabled(&app, enabled);
    Ok(())
}
