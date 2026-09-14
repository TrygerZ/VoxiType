//! Settings commands.

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::error::AppError;
use crate::storage::SettingsManager;
use crate::AppStateInner;

#[tauri::command]
pub fn get_settings(state: State<'_, AppStateInner>) -> std::result::Result<Value, AppError> {
    let mut all = SettingsManager::new(&state.db).all()?;
    sanitize_settings_for_frontend(&mut all);
    Ok(all)
}

pub(crate) fn sanitize_settings_for_frontend(all: &mut Value) {
    // Never leak secrets to the frontend.
    if let Value::Object(map) = all {
        let has_key = map
            .get("groq_api_key")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        map.insert("groq_api_key".to_string(), Value::String(String::new()));
        map.insert("groq_api_key_set".to_string(), Value::Bool(has_key));
    }
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
    "floating_widget_auto_hide_seconds",
];

#[tauri::command]
pub fn update_setting<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppStateInner>,
    key: String,
    value: Value,
) -> std::result::Result<(), AppError> {
    if !SETTABLE_KEYS.contains(&key.as_str()) {
        return Err(AppError::internal(format!("Unknown setting key '{key}'")));
    }
    if key == "floating_widget_auto_hide_seconds" {
        validate_auto_hide_seconds(&value)?;
    }
    let encoded = encode_setting_value(&key, &value, &state.master_key)?;
    SettingsManager::new(&state.db).set_raw(&key, &encoded)?;
    if key == "floating_widget_auto_hide_seconds" {
        crate::overlay::reset_idle_timer_and_reconcile(&app);
    }
    Ok(())
}

pub(crate) fn validate_auto_hide_seconds(value: &Value) -> std::result::Result<u64, AppError> {
    let secs = value
        .as_i64()
        .filter(|&v| v >= 0)
        .map(|v| v as u64)
        .ok_or_else(|| {
            AppError::invalid_input(
                "Setting 'floating_widget_auto_hide_seconds' must be an integer",
            )
        })?;
    if secs != 0 && !(3..=60).contains(&secs) {
        return Err(AppError::invalid_input(
            "Setting 'floating_widget_auto_hide_seconds' must be 0 or between 3 and 60",
        ));
    }
    Ok(secs)
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
            "floating_widget_auto_hide_seconds",
        ] {
            assert!(
                SETTABLE_KEYS.contains(&known),
                "expected '{known}' in SETTABLE_KEYS"
            );
        }
    }

    #[test]
    fn validate_auto_hide_seconds_accepts_valid_range() {
        assert_eq!(
            validate_auto_hide_seconds(&serde_json::json!(0)).unwrap(),
            0
        );
        assert_eq!(
            validate_auto_hide_seconds(&serde_json::json!(3)).unwrap(),
            3
        );
        assert_eq!(
            validate_auto_hide_seconds(&serde_json::json!(30)).unwrap(),
            30
        );
        assert_eq!(
            validate_auto_hide_seconds(&serde_json::json!(60)).unwrap(),
            60
        );
    }

    #[test]
    fn validate_auto_hide_seconds_rejects_out_of_range() {
        for bad in [1, 2, 61, 100] {
            let err = validate_auto_hide_seconds(&serde_json::json!(bad)).unwrap_err();
            assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
        }
    }

    #[test]
    fn validate_auto_hide_seconds_rejects_non_integer() {
        let bad_values = [
            serde_json::json!(-1),
            serde_json::json!(3.5),
            serde_json::json!("10"),
            serde_json::json!(true),
            serde_json::json!(null),
            serde_json::json!([10]),
        ];
        for v in bad_values {
            let err = validate_auto_hide_seconds(&v).unwrap_err();
            assert_eq!(err.code, crate::error::ErrorCode::InvalidInput);
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

    #[test]
    fn sanitize_settings_masks_key_and_sets_indicator() {
        let mut val = serde_json::json!({
            "groq_api_key": "enc:v1:secret",
            "language": "en"
        });
        sanitize_settings_for_frontend(&mut val);
        assert_eq!(val["groq_api_key"], "");
        assert_eq!(val["groq_api_key_set"], true);
        assert_eq!(val["language"], "en");

        let mut empty_val = serde_json::json!({
            "groq_api_key": "",
            "language": "en"
        });
        sanitize_settings_for_frontend(&mut empty_val);
        assert_eq!(empty_val["groq_api_key"], "");
        assert_eq!(empty_val["groq_api_key_set"], false);

        let mut missing_val = serde_json::json!({
            "language": "en"
        });
        sanitize_settings_for_frontend(&mut missing_val);
        assert_eq!(missing_val["groq_api_key"], "");
        assert_eq!(missing_val["groq_api_key_set"], false);
    }
}
