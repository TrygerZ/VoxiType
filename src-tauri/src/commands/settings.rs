//! Settings commands.

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};
use tauri_plugin_autostart::ManagerExt;

use crate::commands::runtime::{MAX_WHISPER_THREADS, MIN_WHISPER_THREADS};
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
/// `whisper_cpp_binary_path`, `whisper_cpp_model_path`, and
/// `whisper_cpp_binary_token` are intentionally absent: writing them requires
/// `set_whisper_cpp_paths`, which only accepts paths previously returned by the
/// native file picker and seals the matching token (see commands::misc).
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
    validate_setting_value(&key, &value)?;
    let encoded = encode_setting_value(&key, &value, &state.master_key)?;
    SettingsManager::new(&state.db).set_raw(&key, &encoded)?;
    if key == "floating_widget_auto_hide_seconds" {
        crate::overlay::reset_idle_timer_and_reconcile(&app);
    }
    if key == "auto_start" {
        // The setting is persisted first; startup reconciliation retries the
        // apply, so a failure here self-heals on the next launch.
        let enabled = value
            .as_bool()
            .ok_or_else(|| AppError::invalid_input("Setting 'auto_start' must be a boolean"))?;
        apply_autostart(&app, enabled)?;
    }
    Ok(())
}

/// Apply the desired Windows startup registration. No-ops in debug builds so
/// `tauri dev` binaries never pollute the user's Run registry key.
pub(crate) fn apply_autostart<R: Runtime>(
    app: &AppHandle<R>,
    enabled: bool,
) -> std::result::Result<(), AppError> {
    if cfg!(debug_assertions) {
        tracing::debug!("Skipping autostart change in debug build (enabled={enabled})");
        return Ok(());
    }
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|e| AppError::internal(format!("Failed to update autostart: {e}")))
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

/// Reject a whisper.cpp thread count outside the range the engine honors.
/// Read-side clamping (see `runtime::whisper_cpp_config_from_settings`) stays
/// for legacy rows; this stops a compromised webview from persisting one.
fn validate_whisper_threads(value: &Value) -> std::result::Result<(), AppError> {
    let in_range = value.as_u64().is_some_and(|v| {
        (u64::from(MIN_WHISPER_THREADS)..=u64::from(MAX_WHISPER_THREADS)).contains(&v)
    });
    if !in_range {
        return Err(AppError::invalid_input(format!(
            "Setting 'whisper_cpp_threads' must be an integer between {MIN_WHISPER_THREADS} and {MAX_WHISPER_THREADS}"
        )));
    }
    Ok(())
}

/// Upper bound for the serialized JSON of any single setting value. Bounds
/// what a compromised webview can persist into the settings table.
const MAX_SETTING_JSON_BYTES: usize = 64 * 1024;
/// Longest accepted identifier-like string setting (mic device id, model name).
const MAX_SETTING_STRING_LEN: usize = 4 * 1024;
/// Longest accepted custom mode id (built-ins are shorter; custom ids exist).
const MAX_ACTIVE_MODE_LEN: usize = 64;

const STT_ENGINES: &[&str] = &["groq", "whisper_cpp"];
const LLM_ENGINES: &[&str] = &["off", "ollama", "groq", "rule_based"];
const UI_LANGUAGES: &[&str] = &["en", "id"];

/// Validate a setting value at the write boundary. Enum-valued keys are
/// checked against their known sets so garbage can never persist and degrade
/// silently at read time (M-6); every value is length-capped (M-13).
fn validate_setting_value(key: &str, value: &Value) -> std::result::Result<(), AppError> {
    let serialized = serde_json::to_string(value)?;
    if serialized.len() > MAX_SETTING_JSON_BYTES {
        return Err(AppError::invalid_input(format!(
            "Setting '{key}' exceeds {MAX_SETTING_JSON_BYTES} bytes"
        )));
    }
    match key {
        "stt_engine" => validate_enum(key, value, STT_ENGINES),
        "llm_engine" => validate_enum(key, value, LLM_ENGINES),
        "language" => validate_enum(key, value, UI_LANGUAGES),
        // stt_language accepts the "auto" sentinel in addition to a language code.
        "stt_language" => validate_language_or_auto(key, value),
        "translation_target" => validate_language_code(key, value),
        "active_mode" => validate_active_mode(value),
        "mic_device" | "stt_model" | "llm_model" => {
            validate_string_len(key, value, MAX_SETTING_STRING_LEN)
        }
        "floating_widget_auto_hide_seconds" => validate_auto_hide_seconds(value).map(|_| ()),
        "whisper_cpp_threads" => validate_whisper_threads(value),
        _ => Ok(()),
    }
}

/// Reject a value that is not one of the known strings for `key`.
fn validate_enum(key: &str, value: &Value, allowed: &[&str]) -> std::result::Result<(), AppError> {
    let s = value
        .as_str()
        .ok_or_else(|| AppError::invalid_input(format!("Setting '{key}' must be a string")))?;
    if !allowed.contains(&s) {
        return Err(AppError::invalid_input(format!(
            "Setting '{key}' has unknown value '{s}'"
        )));
    }
    Ok(())
}

/// Accept "auto" or a language code for `stt_language`.
fn validate_language_or_auto(key: &str, value: &Value) -> std::result::Result<(), AppError> {
    if value.as_str() == Some("auto") {
        return Ok(());
    }
    validate_language_code(key, value)
}

/// Accept a language code: two or three lowercase letters, optional
/// `-`/`_` region suffix of two letters (e.g. "en", "id", "pt-BR"). The
/// region is case-insensitive to accept the ISO 3166 uppercase convention.
fn validate_language_code(key: &str, value: &Value) -> std::result::Result<(), AppError> {
    let s = value
        .as_str()
        .ok_or_else(|| AppError::invalid_input(format!("Setting '{key}' must be a string")))?;
    let mut parts = s.split(['-', '_']);
    let base = parts.next().unwrap_or("");
    let region = parts.next();
    let valid_base = (2..=3).contains(&base.len()) && base.chars().all(|c| c.is_ascii_lowercase());
    let valid_region = match region {
        None => true,
        Some(r) => r.len() == 2 && r.chars().all(|c| c.is_ascii_alphabetic()),
    };
    if !valid_base || !valid_region || parts.next().is_some() {
        return Err(AppError::invalid_input(format!(
            "Setting '{key}' has invalid language code '{s}'"
        )));
    }
    Ok(())
}

/// Custom mode ids are allowed, but bounded and free of control characters.
fn validate_active_mode(value: &Value) -> std::result::Result<(), AppError> {
    let s = value
        .as_str()
        .ok_or_else(|| AppError::invalid_input("Setting 'active_mode' must be a string"))?;
    if s.trim().is_empty() {
        return Err(AppError::invalid_input(
            "Setting 'active_mode' must not be empty",
        ));
    }
    if s.chars().count() > MAX_ACTIVE_MODE_LEN {
        return Err(AppError::invalid_input(format!(
            "Setting 'active_mode' exceeds {MAX_ACTIVE_MODE_LEN} characters"
        )));
    }
    if s.chars().any(char::is_control) {
        return Err(AppError::invalid_input(
            "Setting 'active_mode' must not contain control characters",
        ));
    }
    Ok(())
}

fn validate_string_len(key: &str, value: &Value, max: usize) -> std::result::Result<(), AppError> {
    let s = value
        .as_str()
        .ok_or_else(|| AppError::invalid_input(format!("Setting '{key}' must be a string")))?;
    if s.chars().count() > max {
        return Err(AppError::invalid_input(format!(
            "Setting '{key}' exceeds {max} characters"
        )));
    }
    Ok(())
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
        // Trim before encrypting: a pasted trailing space passes Test (which
        // trims) but breaks dictation auth. Empty stays empty.
        let enc = crate::crypto::encrypt_api_key(plain.trim(), master_key)?;
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
        assert!(!SETTABLE_KEYS.contains(&"whisper_cpp_binary_token"));
        assert!(!SETTABLE_KEYS.contains(&"floating_widget_pos"));
        assert!(!SETTABLE_KEYS.contains(&"hotkey"));
        assert!(!SETTABLE_KEYS.contains(&"evil_setting"));
    }

    fn assert_invalid(key: &str, value: serde_json::Value) {
        let err = validate_setting_value(key, &value).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidInput, "key {key}");
    }

    #[test]
    fn validate_setting_accepts_valid_enum_values() {
        assert!(validate_setting_value("stt_engine", &serde_json::json!("groq")).is_ok());
        assert!(validate_setting_value("stt_engine", &serde_json::json!("whisper_cpp")).is_ok());
        for engine in ["off", "ollama", "groq", "rule_based"] {
            assert!(
                validate_setting_value("llm_engine", &serde_json::json!(engine)).is_ok(),
                "llm_engine {engine}"
            );
        }
        for lang in ["en", "id"] {
            assert!(validate_setting_value("language", &serde_json::json!(lang)).is_ok());
        }
    }

    #[test]
    fn validate_setting_rejects_unknown_enum_values() {
        assert_invalid("stt_engine", serde_json::json!("whisper"));
        assert_invalid("stt_engine", serde_json::json!(42));
        assert_invalid("llm_engine", serde_json::json!("gpt"));
        assert_invalid("language", serde_json::json!("fr"));
        assert_invalid("language", serde_json::json!(null));
    }

    #[test]
    fn validate_setting_language_code_rules() {
        for ok in ["en", "id", "auto", "eng", "pt-BR", "zh_cn"] {
            assert!(
                validate_setting_value("stt_language", &serde_json::json!(ok)).is_ok(),
                "stt_language {ok}"
            );
        }
        for ok in ["en", "id", "pt-BR"] {
            assert!(
                validate_setting_value("translation_target", &serde_json::json!(ok)).is_ok(),
                "translation_target {ok}"
            );
        }
        // translation_target has no "auto" sentinel.
        assert_invalid("translation_target", serde_json::json!("auto"));
        for bad in ["english", "e", "EN", "en-US-extra", "en1"] {
            assert_invalid("stt_language", serde_json::json!(bad));
        }
    }

    #[test]
    fn validate_setting_active_mode_rules() {
        assert!(validate_setting_value("active_mode", &serde_json::json!("dictation")).is_ok());
        assert!(validate_setting_value("active_mode", &serde_json::json!("my-custom")).is_ok());
        assert_invalid("active_mode", serde_json::json!(""));
        assert_invalid("active_mode", serde_json::json!("   "));
        assert_invalid("active_mode", serde_json::json!("bad\nmode"));
        assert_invalid("active_mode", serde_json::json!("x".repeat(65)));
    }

    #[test]
    fn validate_setting_length_caps() {
        assert!(validate_setting_value("stt_model", &serde_json::json!("x".repeat(4096))).is_ok());
        assert_invalid("stt_model", serde_json::json!("x".repeat(4097)));
        assert_invalid("mic_device", serde_json::json!("x".repeat(4097)));
        assert_invalid("llm_model", serde_json::json!(123));
        // Serialized JSON over 64 KB is rejected for any key.
        assert_invalid("sound_cues", serde_json::json!("x".repeat(70 * 1024)));
    }

    #[test]
    fn validate_setting_whisper_threads_range() {
        assert!(validate_setting_value("whisper_cpp_threads", &serde_json::json!(1)).is_ok());
        assert!(validate_setting_value("whisper_cpp_threads", &serde_json::json!(32)).is_ok());
        assert_invalid("whisper_cpp_threads", serde_json::json!(0));
        assert_invalid("whisper_cpp_threads", serde_json::json!(33));
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
    fn encode_groq_api_key_trims_whitespace_before_encrypting() {
        let key = [5u8; 32];
        let encoded =
            encode_setting_value("groq_api_key", &serde_json::json!("  key123  "), &key).unwrap();
        let stored: String = serde_json::from_str(&encoded).unwrap();
        let decrypted = crate::crypto::decrypt_api_key(&stored, &key).unwrap();
        assert_eq!(decrypted, "key123");
    }

    #[test]
    fn encode_groq_api_key_trims_whitespace_only_to_empty() {
        let key = [6u8; 32];
        let encoded =
            encode_setting_value("groq_api_key", &serde_json::json!("   "), &key).unwrap();
        let stored: String = serde_json::from_str(&encoded).unwrap();
        // Whitespace-only trims to empty, which encrypt_api_key stores as empty.
        assert_eq!(stored, "");
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
