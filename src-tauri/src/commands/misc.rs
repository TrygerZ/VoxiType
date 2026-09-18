//! Microphone, hotkey, app-info, file picker, and update commands.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::audio::DeviceInfo;
pub use crate::data_dir::DataDirectoryStatus;
use crate::error::AppError;
use crate::hotkey;
use crate::storage::SettingsManager;
use crate::stt::{SttConfig, SttEngine, WhisperCppConfig};
use crate::AppStateInner;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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
    let new_cfg = hotkey::HotkeyConfig { key, mode: hk_mode };
    let state = app.state::<AppStateInner>();
    let old_cfg = SettingsManager::new(&state.db)
        .get::<hotkey::HotkeyConfig>("hotkey")
        .ok()
        .flatten();

    hotkey::rebind(&app, &new_cfg, old_cfg.as_ref(), || {
        SettingsManager::new(&state.db).set("hotkey", &new_cfg)
    })
}

#[tauri::command]
pub fn get_app_info<R: Runtime>(app: AppHandle<R>) -> Value {
    let state = app.state::<AppStateInner>();
    let db_path = state.app_data_dir.join("data").join("voxitype.db");
    serde_json::json!({
        "name": "VoxiType",
        "version": app.package_info().version.to_string(),
        "tauri": "2",
        "data_dir": state.app_data_dir.to_string_lossy(),
        "db_path": db_path.to_string_lossy(),
    })
}

#[tauri::command]
pub async fn check_updates<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<crate::updater::UpdateInfo, AppError> {
    let current = app.package_info().version.to_string();
    crate::updater::check(&current).await
}

#[tauri::command]
pub fn open_url(url: String) -> std::result::Result<(), AppError> {
    let url = validate_open_url(&url)?;
    open::that(&url).map_err(|e| AppError::internal(format!("Failed to open URL: {e}")))?;
    Ok(())
}

/// Only allow http(s) URLs to a fixed host allowlist. `open::that` dispatches
/// via the OS shell handler, which honors `file://`, `smb://`, and bare local
/// paths — rejecting those prevents launching arbitrary executables.
/// ponytail: hand-rolled parse instead of the `url` crate to avoid a new dep;
/// upgrade to `url::Url` if query/fragment/auth validation ever matters.
fn validate_open_url(url: &str) -> std::result::Result<String, AppError> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| AppError::internal("Invalid URL: missing scheme"))?;
    if scheme != "http" && scheme != "https" {
        return Err(AppError::internal(format!(
            "Refused URL with scheme '{scheme}': only http/https allowed"
        )));
    }
    if rest.contains('\\') {
        return Err(AppError::internal(
            "Refused URL: backslash not permitted in URL",
        ));
    }
    // Strip userinfo, port, path, query, fragment to isolate the host.
    let authority = rest.split('/').next().unwrap_or("");
    let host = authority
        .split('@')
        .next_back()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    const ALLOWED: &[&str] = &["console.groq.com", "github.com", "huggingface.co"];
    if !ALLOWED.contains(&host) {
        return Err(AppError::internal(format!(
            "Refused URL to host '{host}': not in allowlist"
        )));
    }
    Ok(url.to_string())
}

/// Reveal the floating-widget window once its page has mounted. The frontend
/// invokes this after render so the overlay only becomes visible once its
/// transparent content is painted, avoiding a white-square flash in dev.
#[tauri::command]
pub fn reveal_floating_widget<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    crate::overlay::reveal_if_enabled(&app);
    Ok(())
}

/// Reset the floating-widget idle timeout timer upon pointer activity (hover, click, hold).
#[tauri::command]
pub fn reset_widget_idle_timer<R: Runtime>(app: AppHandle<R>) -> std::result::Result<(), AppError> {
    crate::overlay::reset_idle_timer_and_reconcile(&app);
    Ok(())
}

/// Acknowledge from the frontend that the floating widget hide animation has completed.
#[tauri::command]
pub fn ack_widget_hide<R: Runtime>(
    app: AppHandle<R>,
    id: u64,
) -> std::result::Result<(), AppError> {
    crate::overlay::acknowledge_hide(&app, id);
    Ok(())
}

#[tauri::command]
pub async fn pick_setup_file(
    state: State<'_, AppStateInner>,
    kind: String,
) -> std::result::Result<Option<String>, AppError> {
    let picked = {
        let kind = kind.clone();
        tokio::task::spawn_blocking(move || pick_setup_file_blocking(&kind))
            .await
            .map_err(|e| AppError::internal(format!("File picker failed: {e}")))??
    };
    if let Some(path) = &picked {
        let canonical = Path::new(path)
            .canonicalize()
            .map_err(|e| AppError::internal(format!("Failed to canonicalize picked path: {e}")))?;
        if canonical.is_dir() {
            return Err(AppError::internal("Selected path cannot be a directory"));
        }
        let clean = canonical_to_clean_string(&canonical);
        remember_picker_path(&state, &kind, canonical);
        return Ok(Some(clean));
    }
    Ok(None)
}

/// Picker kinds that gate whisper.cpp path settings.
const PICKER_KIND_BINARY: &str = "whisper_binary";
const PICKER_KIND_MODEL: &str = "whisper_model";
const PICKER_KIND_DATA_DIRECTORY: &str = "data_directory";

fn canonical_to_clean_string(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s.into_owned()
    }
}

fn remember_picker_path(state: &AppStateInner, kind: &str, canonical: PathBuf) {
    match state.last_picker_dirs.lock() {
        Ok(mut dirs) => {
            dirs.insert(kind.to_string(), canonical);
        }
        Err(_) => {
            tracing::warn!("Picker directory state poisoned; next gated write may be refused")
        }
    }
}

/// A path is acceptable only when a dialog result of `kind` exists and the
/// path canonicalizes to the exact same canonical path selected by the user.
/// Canonicalizing resolves `..` traversal and case differences on Windows; a
/// missing file fails `canonicalize` and is rejected outright.
fn path_matches_picker_path(expected: Option<&Path>, path: &str) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    match Path::new(path).canonicalize() {
        Ok(candidate) => candidate == expected,
        Err(_) => false,
    }
}

fn ensure_picker_backed_path(
    state: &AppStateInner,
    kind: &str,
    path: &str,
) -> std::result::Result<(), AppError> {
    let dirs = state
        .last_picker_dirs
        .lock()
        .map_err(|_| AppError::internal("Internal picker state unavailable"))?;
    if path_matches_picker_path(dirs.get(kind).map(PathBuf::as_path), path) {
        Ok(())
    } else {
        Err(AppError::internal(format!(
            "Refused {kind} path: it was not selected via the setup file dialog"
        )))
    }
}

/// Selects a directory. `set_data_directory` only accepts this dialog's result.
#[tauri::command]
pub async fn pick_data_directory(
    state: State<'_, AppStateInner>,
) -> std::result::Result<Option<String>, AppError> {
    let picked = tokio::task::spawn_blocking(pick_data_directory_blocking)
        .await
        .map_err(|e| AppError::internal(format!("Folder picker failed: {e}")))??;
    if let Some(path) = &picked {
        let canonical = Path::new(path)
            .canonicalize()
            .map_err(|e| AppError::data_directory(format!("Invalid selected directory: {e}")))?;
        let clean = canonical_to_clean_string(&canonical);
        let mut dirs = state
            .last_picker_dirs
            .lock()
            .map_err(|_| AppError::internal("Internal picker state unavailable"))?;
        dirs.insert(PICKER_KIND_DATA_DIRECTORY.to_string(), canonical);
        return Ok(Some(clean));
    }
    Ok(None)
}

#[tauri::command]
/// Writes the data-directory marker only; the new directory takes effect after
/// application restart, when startup resolves the marker before opening storage.
pub fn set_data_directory(
    state: State<'_, AppStateInner>,
    path: String,
) -> std::result::Result<(), AppError> {
    ensure_picker_backed_path(&state, PICKER_KIND_DATA_DIRECTORY, &path)?;
    crate::data_dir::write_marker(
        &state.default_app_data_dir,
        &state.app_data_dir,
        Path::new(&path),
    )?;
    crate::data_dir::clear_error(&state.default_app_data_dir);
    Ok(())
}

#[tauri::command]
pub fn get_data_directory(state: State<'_, AppStateInner>) -> crate::data_dir::DataDirectoryStatus {
    crate::data_dir::get_status(&state.default_app_data_dir, &state.app_data_dir)
}

#[tauri::command]
pub fn restart_app(app: AppHandle) {
    tracing::info!("Restarting application via restart_app command");
    app.restart();
}

fn pick_data_directory_blocking() -> std::result::Result<Option<String>, AppError> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.FolderBrowserDialog
$dialog.Description = 'Select VoxiType data directory'
$dialog.ShowNewFolderButton = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.SelectedPath)
}
"#;
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-STA",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        script,
    ]);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command
        .output()
        .map_err(|e| AppError::internal(format!("Failed to open folder picker: {e}")))?;
    if !output.status.success() {
        return Err(AppError::internal(format!(
            "Folder picker failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!path.is_empty()).then_some(path))
}

/// Persist whisper.cpp paths atomically after validating both.
///
/// Both paths must resolve to the exact canonical files selected
/// via the native setup dialog in this session.
#[tauri::command]
pub fn set_whisper_cpp_paths(
    state: State<'_, AppStateInner>,
    binary_path: Option<String>,
    model_path: Option<String>,
) -> std::result::Result<(), AppError> {
    // Validate both paths first so failure leaves existing settings untouched.
    let canonical_binary = if let Some(binary) = &binary_path {
        ensure_picker_backed_path(&state, PICKER_KIND_BINARY, binary)?;
        let canonical = Path::new(binary).canonicalize().map_err(|e| {
            AppError::stt(format!("Failed to canonicalize whisper binary path: {e}"))
        })?;
        if canonical.is_dir() {
            return Err(AppError::stt("Whisper binary path cannot be a directory"));
        }
        Some(canonical_to_clean_string(&canonical))
    } else {
        None
    };

    let canonical_model = if let Some(model) = &model_path {
        ensure_picker_backed_path(&state, PICKER_KIND_MODEL, model)?;
        let canonical = Path::new(model).canonicalize().map_err(|e| {
            AppError::stt(format!("Failed to canonicalize whisper model path: {e}"))
        })?;
        if canonical.is_dir() {
            return Err(AppError::stt("Whisper model path cannot be a directory"));
        }
        Some(canonical_to_clean_string(&canonical))
    } else {
        None
    };

    let binary_json = canonical_binary
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;
    let model_json = canonical_model
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    let mut entries = Vec::with_capacity(2);
    if let Some(ref val) = binary_json {
        entries.push(("whisper_cpp_binary_path", val.as_str()));
    }
    if let Some(ref val) = model_json {
        entries.push(("whisper_cpp_model_path", val.as_str()));
    }

    SettingsManager::new(&state.db).set_raw_batch(&entries)?;
    Ok(())
}

fn pick_setup_file_blocking(kind: &str) -> std::result::Result<Option<String>, AppError> {
    let (title, filter) = match kind {
        "whisper_binary" => (
            "Select whisper-cli.exe",
            "whisper-cli.exe|whisper-cli.exe|Executable files (*.exe)|*.exe|All files (*.*)|*.*",
        ),
        "whisper_model" => (
            "Select whisper.cpp GGML model",
            "GGML model files (ggml-*.bin)|ggml-*.bin|Binary model files (*.bin)|*.bin|All files (*.*)|*.*",
        ),
        _ => return Err(AppError::internal("Unknown setup file picker kind")),
    };

    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = $env:VX_PICK_TITLE
$dialog.Filter = $env:VX_PICK_FILTER
$dialog.CheckFileExists = $true
$dialog.Multiselect = $false
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;

    let mut command = Command::new("powershell.exe");
    command
        .args([
            "-NoProfile",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("VX_PICK_TITLE", title)
        .env("VX_PICK_FILTER", filter);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|e| AppError::internal(format!("Failed to open file picker: {e}")))?;
    if !output.status.success() {
        return Err(AppError::internal(format!(
            "File picker failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        Ok(None)
    } else {
        Ok(Some(path))
    }
}

pub(crate) fn resolve_test_api_key(
    state: &AppStateInner,
    api_key: Option<String>,
) -> std::result::Result<String, AppError> {
    match api_key.as_deref().map(str::trim) {
        Some(k) if !k.is_empty() => Ok(k.to_string()),
        _ => {
            let key = super::runtime::decrypted_api_key(state)?;
            if key.trim().is_empty() {
                return Err(AppError::api_key_missing("Groq API key is not set"));
            }
            Ok(key)
        }
    }
}

#[tauri::command]
pub async fn test_groq_api(
    state: State<'_, AppStateInner>,
    api_key: Option<String>,
) -> std::result::Result<(), AppError> {
    let api_key = resolve_test_api_key(&state, api_key)?;

    let client = crate::util::http_client();
    let resp = client
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(api_key.trim())
        .send()
        .await
        .map_err(|e| AppError::stt(format!("Network error: {e}")))?;

    match resp.status() {
        reqwest::StatusCode::OK => Ok(()),
        reqwest::StatusCode::UNAUTHORIZED => Err(AppError::new(
            crate::error::ErrorCode::SttApiKeyInvalid,
            "Invalid Groq API key",
        )),
        status => Err(AppError::stt(format!("Groq API returned {status}"))),
    }
}

#[tauri::command]
pub async fn test_whisper_cpp(
    state: State<'_, AppStateInner>,
    binary_path: String,
    model_path: String,
    language: String,
    threads: u32,
) -> std::result::Result<(), AppError> {
    // Same gate as set_whisper_cpp_paths: never execute a binary that did
    // not come from the native setup dialog.
    ensure_picker_backed_path(&state, PICKER_KIND_BINARY, &binary_path)?;
    ensure_picker_backed_path(&state, PICKER_KIND_MODEL, &model_path)?;

    let canonical_binary = Path::new(&binary_path)
        .canonicalize()
        .map_err(|e| AppError::stt(format!("Failed to canonicalize whisper binary path: {e}")))?;
    if canonical_binary.is_dir() {
        return Err(AppError::stt("Whisper binary path cannot be a directory"));
    }
    let canonical_model = Path::new(&model_path)
        .canonicalize()
        .map_err(|e| AppError::stt(format!("Failed to canonicalize whisper model path: {e}")))?;

    let engine = crate::stt::whisper_cpp::WhisperCppEngine::new(WhisperCppConfig {
        binary_path: canonical_to_clean_string(&canonical_binary),
        model_path: canonical_to_clean_string(&canonical_model),
        threads: threads.max(1),
    });
    let config = SttConfig {
        language,
        initial_prompt: None,
        temperature: 0.0,
    };
    let silence = vec![0.0; 16_000];
    engine.transcribe(&silence, &config).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_allowed_https_hosts() {
        assert!(validate_open_url("https://console.groq.com").is_ok());
        assert!(validate_open_url("https://github.com/ggml-org/whisper.cpp").is_ok());
        assert!(validate_open_url("https://huggingface.co/ggerganov/whisper.cpp").is_ok());
    }

    #[test]
    fn rejects_non_http_schemes() {
        assert!(validate_open_url("file:///C:/Windows/System32/cmd.exe").is_err());
        assert!(validate_open_url("smb://attacker/share/payload.lnk").is_err());
        assert!(validate_open_url("C:\\Windows\\System32\\cmd.exe").is_err());
    }

    #[test]
    fn rejects_unallowed_hosts() {
        assert!(validate_open_url("https://evil.com").is_err());
        assert!(validate_open_url("https://github.com.evil.com").is_err());
    }

    #[test]
    fn strips_userinfo_and_port() {
        assert!(validate_open_url("https://user:pass@github.com:443/repo").is_ok());
        assert!(validate_open_url("https://user@evil.com").is_err());
    }

    #[test]
    fn rejects_backslash_authority_ambiguity() {
        assert!(validate_open_url(r"https://evil.com\@github.com").is_err());
        assert!(validate_open_url(r"https://github.com\path").is_err());
        assert!(validate_open_url("https://github.com").is_ok());
        assert!(validate_open_url("file:///C:/Windows/System32/cmd.exe").is_err());
    }

    /// Create a unique scratch dir with one file inside; returns the
    /// canonicalized file path and the clean file path as a string.
    fn scratch_dir_with_file(test_name: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("voxitype-misc-{test_name}"));
        std::fs::create_dir_all(&dir).expect("scratch dir creation failed");
        let file = dir.join("tool.exe");
        std::fs::write(&file, b"stub").expect("stub file write failed");
        let canonical_file = file.canonicalize().expect("canonicalize file failed");
        let clean = canonical_to_clean_string(&canonical_file);
        (canonical_file, clean)
    }

    #[test]
    fn picker_gated_path_accepts_dialog_result() {
        let (canonical_file, file) = scratch_dir_with_file("accept");
        assert!(path_matches_picker_path(Some(&canonical_file), &file));
    }

    #[test]
    fn picker_gated_path_rejects_traversal_and_siblings() {
        let (canonical_file, _file) = scratch_dir_with_file("traversal");
        let dir = canonical_file.parent().unwrap();

        // Sibling file in the same directory.
        let sibling_file = dir.join("sibling.exe");
        std::fs::write(&sibling_file, b"stub").ok();
        if let Ok(sibling) = sibling_file.canonicalize() {
            assert!(!path_matches_picker_path(
                Some(&canonical_file),
                &sibling.to_string_lossy()
            ));
        }

        // Sibling directory with an existing file.
        let sibling = dir
            .parent()
            .map(|p| p.join("voxitype-misc-sibling"))
            .unwrap_or_else(|| std::env::temp_dir().join("voxitype-misc-sibling"));
        std::fs::create_dir_all(&sibling).ok();
        let sibling_file = sibling.join("evil.exe");
        std::fs::write(&sibling_file, b"stub").ok();
        if let Ok(evil) = sibling_file.canonicalize() {
            assert!(!path_matches_picker_path(
                Some(&canonical_file),
                &evil.to_string_lossy()
            ));
        }
        // Traversal that resolves outside the selected file.
        let escaped = dir.join("..").join("voxitype-misc-accept").join("tool.exe");
        assert!(!path_matches_picker_path(
            Some(&canonical_file),
            &escaped.to_string_lossy()
        ));
    }

    #[test]
    fn picker_gated_path_requires_prior_pick() {
        let (_canonical_file, file) = scratch_dir_with_file("no-base");
        assert!(!path_matches_picker_path(None, &file));
    }

    #[test]
    fn picker_gated_path_rejects_missing_file() {
        let (canonical_file, _file) = scratch_dir_with_file("missing-file");
        let ghost = canonical_file.parent().unwrap().join("does-not-exist.exe");
        assert!(!path_matches_picker_path(
            Some(&canonical_file),
            &ghost.to_string_lossy()
        ));
    }

    fn test_app_state(master_key: [u8; 32]) -> AppStateInner {
        AppStateInner {
            db: crate::storage::Database::open_in_memory().unwrap(),
            pipeline: crate::pipeline::PipelineOrchestrator::new(),
            app_data_dir: PathBuf::from("/test"),
            default_app_data_dir: PathBuf::from("/test"),
            master_key,
            _log_guard: None,
            stt_engine: std::sync::Mutex::new(None),
            last_picker_dirs: std::sync::Mutex::new(std::collections::HashMap::new()),
            widget_timer: crate::overlay::WidgetTimerState::new(),
        }
    }

    #[test]
    fn resolve_test_api_key_uses_explicit_key() {
        let state = test_app_state([1u8; 32]);
        let resolved = resolve_test_api_key(&state, Some("gsk_direct_key".to_string())).unwrap();
        assert_eq!(resolved, "gsk_direct_key");
    }

    #[test]
    fn resolve_test_api_key_falls_back_to_stored_key_when_none_or_empty() {
        let master_key = [5u8; 32];
        let state = test_app_state(master_key);
        let secret = "gsk_stored_secret_key";
        let encrypted = crate::crypto::encrypt_api_key(secret, &master_key).unwrap();
        crate::storage::SettingsManager::new(&state.db)
            .set("groq_api_key", &encrypted)
            .unwrap();

        assert_eq!(resolve_test_api_key(&state, None).unwrap(), secret);
        assert_eq!(
            resolve_test_api_key(&state, Some(String::new())).unwrap(),
            secret
        );
        assert_eq!(
            resolve_test_api_key(&state, Some("   ".to_string())).unwrap(),
            secret
        );
    }

    #[test]
    fn resolve_test_api_key_fails_when_stored_key_not_set() {
        let state = test_app_state([1u8; 32]);
        let err_none = resolve_test_api_key(&state, None).unwrap_err();
        assert_eq!(err_none.code, crate::error::ErrorCode::SttApiKeyInvalid);
        assert_eq!(err_none.message, "Groq API key is not set");

        let err_empty = resolve_test_api_key(&state, Some(String::new())).unwrap_err();
        assert_eq!(err_empty.code, crate::error::ErrorCode::SttApiKeyInvalid);
        assert_eq!(err_empty.message, "Groq API key is not set");
    }

    #[test]
    fn resolve_test_api_key_fails_when_stored_key_undecryptable_without_leak() {
        let original_key = [3u8; 32];
        let state = test_app_state([4u8; 32]);
        let secret = "gsk_secret_should_never_leak";
        let encrypted = crate::crypto::encrypt_api_key(secret, &original_key).unwrap();
        crate::storage::SettingsManager::new(&state.db)
            .set("groq_api_key", &encrypted)
            .unwrap();

        let err = resolve_test_api_key(&state, None).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::SttApiKeyInvalid);
        assert_ne!(err.message, "Groq API key is not set");
        assert!(err.message.contains("decrypt") || err.message.contains("master key"));
        assert!(!err.message.contains(secret));
    }
}
