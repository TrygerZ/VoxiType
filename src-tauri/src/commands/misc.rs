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
        remember_picker_dir(&state, &kind, path);
    }
    Ok(picked)
}

/// Picker kinds that gate whisper.cpp path settings.
const PICKER_KIND_BINARY: &str = "whisper_binary";
const PICKER_KIND_MODEL: &str = "whisper_model";
const PICKER_KIND_DATA_DIRECTORY: &str = "data_directory";

fn remember_picker_dir(state: &AppStateInner, kind: &str, picked_path: &str) {
    let Some(parent) = Path::new(picked_path).parent() else {
        tracing::warn!("Picked '{kind}' file has no parent directory");
        return;
    };
    let Ok(dir) = parent.canonicalize() else {
        tracing::warn!("Could not canonicalize picker directory for '{kind}'");
        return;
    };
    match state.last_picker_dirs.lock() {
        Ok(mut dirs) => {
            dirs.insert(kind.to_string(), dir);
        }
        Err(_) => {
            tracing::warn!("Picker directory state poisoned; next gated write may be refused")
        }
    }
}

/// A path is acceptable only when a dialog result of `kind` exists and the
/// path canonicalizes under that result's parent directory. Canonicalizing
/// both sides resolves `..` traversal and case differences on Windows; a
/// missing file fails `canonicalize` and is rejected outright.
fn path_matches_picker_dir(base: Option<&Path>, path: &str) -> bool {
    let Some(base) = base else {
        return false;
    };
    match Path::new(path).canonicalize() {
        Ok(candidate) => candidate.starts_with(base),
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
    if path_matches_picker_dir(dirs.get(kind).map(PathBuf::as_path), path) {
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
        let mut dirs = state
            .last_picker_dirs
            .lock()
            .map_err(|_| AppError::internal("Internal picker state unavailable"))?;
        dirs.insert(PICKER_KIND_DATA_DIRECTORY.to_string(), canonical);
    }
    Ok(picked)
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

/// Persist whisper.cpp paths. The only sanctioned write path for these
/// settings: each value must live under the directory of a file the user
/// actually picked with the native dialog in this session, so a compromised
/// webview cannot point local STT at an attacker-controlled binary.
#[tauri::command]
pub fn set_whisper_cpp_paths(
    state: State<'_, AppStateInner>,
    binary_path: Option<String>,
    model_path: Option<String>,
) -> std::result::Result<(), AppError> {
    let settings = SettingsManager::new(&state.db);
    if let Some(binary) = &binary_path {
        ensure_picker_backed_path(&state, PICKER_KIND_BINARY, binary)?;
        settings.set_raw("whisper_cpp_binary_path", &serde_json::to_string(binary)?)?;
    }
    if let Some(model) = &model_path {
        ensure_picker_backed_path(&state, PICKER_KIND_MODEL, model)?;
        settings.set_raw("whisper_cpp_model_path", &serde_json::to_string(model)?)?;
    }
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

#[tauri::command]
pub async fn test_groq_api(
    state: State<'_, AppStateInner>,
    api_key: String,
) -> std::result::Result<(), AppError> {
    let api_key = if api_key.trim().is_empty() {
        super::runtime::decrypted_api_key(&state)
    } else {
        api_key
    };
    if api_key.trim().is_empty() {
        return Err(AppError::api_key_missing("Groq API key is not set"));
    }

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

    let engine = crate::stt::whisper_cpp::WhisperCppEngine::new(WhisperCppConfig {
        binary_path,
        model_path,
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
    /// canonicalized dir and the canonicalized file path as a string.
    fn scratch_dir_with_file(test_name: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!("voxitype-misc-{test_name}"));
        std::fs::create_dir_all(&dir).expect("scratch dir creation failed");
        let file = dir.join("tool.exe");
        std::fs::write(&file, b"stub").expect("stub file write failed");
        let canonical_dir = dir.canonicalize().expect("canonicalize dir failed");
        let canonical_file = file.canonicalize().expect("canonicalize file failed");
        (canonical_dir, canonical_file.to_string_lossy().into_owned())
    }

    #[test]
    fn picker_gated_path_accepts_dialog_result() {
        let (dir, file) = scratch_dir_with_file("accept");
        assert!(path_matches_picker_dir(Some(&dir), &file));
    }

    #[test]
    fn picker_gated_path_rejects_traversal_and_siblings() {
        let (dir, _file) = scratch_dir_with_file("traversal");
        // Sibling directory with an existing file.
        let sibling = dir
            .parent()
            .map(|p| p.join(".."))
            .unwrap_or_else(|| std::env::temp_dir().join("voxitype-misc-sibling"));
        std::fs::create_dir_all(&sibling).ok();
        let sibling_file = sibling.join("evil.exe");
        std::fs::write(&sibling_file, b"stub").ok();
        if let Ok(evil) = sibling_file.canonicalize() {
            assert!(!path_matches_picker_dir(
                Some(&dir),
                &evil.to_string_lossy()
            ));
        }
        // Traversal that resolves outside the base dir.
        let escaped = dir.join("..").join("voxitype-misc-accept").join("tool.exe");
        assert!(!path_matches_picker_dir(
            Some(&dir),
            &escaped.to_string_lossy()
        ));
    }

    #[test]
    fn picker_gated_path_requires_prior_pick() {
        let (_dir, file) = scratch_dir_with_file("no-base");
        assert!(!path_matches_picker_dir(None, &file));
    }

    #[test]
    fn picker_gated_path_rejects_missing_file() {
        let (dir, _file) = scratch_dir_with_file("missing-file");
        let ghost = dir.join("does-not-exist.exe");
        assert!(!path_matches_picker_dir(
            Some(&dir),
            &ghost.to_string_lossy()
        ));
    }
}
