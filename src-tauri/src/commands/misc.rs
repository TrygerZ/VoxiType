//! Microphone, hotkey, app-info, file picker, and update commands.

use std::process::Command;

use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::audio::DeviceInfo;
use crate::error::AppError;
use crate::hotkey;
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
    let cfg = hotkey::HotkeyConfig { key, mode: hk_mode };

    hotkey::rebind(&app, &cfg)?;
    {
        let state = app.state::<AppStateInner>();
        crate::storage::SettingsManager::new(&state.db).set("hotkey", &cfg)?;
    }
    Ok(())
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

#[tauri::command]
pub fn open_url(url: String) -> std::result::Result<(), AppError> {
    open::that(&url).map_err(|e| AppError::internal(format!("Failed to open URL: {e}")))?;
    Ok(())
}

#[tauri::command]
pub async fn pick_setup_file(kind: String) -> std::result::Result<Option<String>, AppError> {
    tokio::task::spawn_blocking(move || pick_setup_file_blocking(&kind))
        .await
        .map_err(|e| AppError::internal(format!("File picker failed: {e}")))?
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
    binary_path: String,
    model_path: String,
    language: String,
    threads: u32,
) -> std::result::Result<(), AppError> {
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
