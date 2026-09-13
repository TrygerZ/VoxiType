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
/// at the IPC boundary so a compromised page can't pollute config.
/// `whisper_cpp_binary_path` and `whisper_cpp_model_path` are intentionally
/// absent: writing them requires `set_whisper_cpp_paths`, which only accepts
/// paths previously returned by the native file picker (see commands::misc).
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
        return Err(AppError::internal(format!("Unknown setting key '{key}'")));
    }
    let encoded = encode_setting_value(&key, &value, &state.master_key)?;
    SettingsManager::new(&state.db).set_raw(&key, &encoded)
}

fn encode_setting_value(
    key: &str,
    value: &Value,
    master_key: &[u8; 32],
) -> std::result::Result<String, AppError> {
    if key == "groq_api_key" {
        // Prevent accidental type coercion from clearing the stored secret.
        let plain = value
            .as_str()
            .ok_or_else(|| AppError::invalid_input("Setting 'groq_api_key' must be a string"))?;
        let enc = crate::crypto::encrypt_api_key(plain, master_key)?;
        Ok(serde_json::to_string(&enc)?)
    } else {
        Ok(serde_json::to_string(value)?)
    }
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
            "whisper_cpp_threads",
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
        // Keys an attacker would need to forge to hijack local STT are not
        // settable; whisper paths go through set_whisper_cpp_paths instead.
        assert!(!SETTABLE_KEYS.contains(&"whisper_cpp_binary_path"));
        assert!(!SETTABLE_KEYS.contains(&"whisper_cpp_model_path"));
        assert!(!SETTABLE_KEYS.contains(&"floating_widget_pos"));
        assert!(!SETTABLE_KEYS.contains(&"hotkey"));
        assert!(!SETTABLE_KEYS.contains(&"evil_setting"));
    }

    #[test]
    fn encode_groq_api_key_rejects_non_string() {
        let key = [0u8; 32];
        let bad_values = [
            serde_json::json!(123),
            serde_json::json!(true),
            serde_json::json!(null),
            serde_json::json!({"key": "val"}),
            serde_json::json!(["abc"]),
        ];
        for v in bad_values {
            let err = encode_setting_value("groq_api_key", &v, &key).unwrap_err();
            assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
        }
    }

    #[test]
    fn encode_groq_api_key_allows_empty_and_valid_string() {
        let key = [0u8; 32];
        let empty = encode_setting_value("groq_api_key", &serde_json::json!(""), &key).unwrap();
        assert!(!empty.is_empty());

        let valid =
            encode_setting_value("groq_api_key", &serde_json::json!("gsk_test"), &key).unwrap();
        assert!(!valid.is_empty());
    }
}
