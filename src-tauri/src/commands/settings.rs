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

/// Keys the frontend may write via `update_setting`. Anything else is rejected
/// at the IPC boundary so a compromised page can't pollute config (e.g. point
/// `whisper_cpp_binary_path` at an attacker path then wait for local STT).
/// `hotkey` and `floating_widget` have their own commands; `floating_widget_pos`
/// is written backend-side only.
const SETTABLE_KEYS: &[&str] = &[
    "onboarding_completed",
    "language",
    "sound_cues",
    "stt_engine",
    "stt_language",
    "stt_model",
    "groq_api_key",
    "whisper_cpp_binary_path",
    "whisper_cpp_model_path",
    "whisper_cpp_threads",
    "auto_start",
    "auto_update",
    "command_mode",
    "telemetry",
    "mic_device",
    "llm_engine",
    "llm_model",
    "per_app_mode",
    "active_mode",
    "translation_enabled",
    "translation_target",
];

#[tauri::command]
pub fn update_setting(
    state: State<'_, AppStateInner>,
    key: String,
    value: Value,
) -> std::result::Result<(), AppError> {
    if !SETTABLE_KEYS.contains(&key.as_str()) {
        return Err(AppError::internal(format!(
            "Unknown setting key '{key}'"
        )));
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_covers_known_keys() {
        for known in [
            "onboarding_completed",
            "language",
            "groq_api_key",
            "whisper_cpp_binary_path",
            "llm_engine",
            "active_mode",
        ] {
            assert!(
                SETTABLE_KEYS.contains(&known),
                "expected '{known}' in SETTABLE_KEYS"
            );
        }
    }

    #[test]
    fn allowlist_rejects_attacker_keys() {
        // A key an attacker would need to forge to hijack local STT is not settable.
        assert!(!SETTABLE_KEYS.contains(&"floating_widget_pos"));
        assert!(!SETTABLE_KEYS.contains(&"hotkey"));
        assert!(!SETTABLE_KEYS.contains(&"evil_setting"));
    }
}
