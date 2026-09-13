//! Unified application error type.
//!
//! Each module produces its own failures which are converted into [`AppError`]
//! at the boundary. `AppError` implements `Serialize` so it can cross the Tauri
//! IPC boundary as a structured error.

use serde::Serialize;
use std::fmt;

/// Stable machine-readable error codes, mirrored on the frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    // Audio
    MicNotFound,
    MicPermissionDenied,
    AudioDeviceError,
    RecordingTimeout,
    ProcessingTimeout,
    // STT
    SttModelNotFound,
    SttEngineError,
    SttApiError,
    SttApiKeyInvalid,
    // LLM
    LlmConnectionRefused,
    LlmApiError,
    LlmApiKeyInvalid,
    // Injection
    InjectionFailed,
    InjectionPermission,
    // System
    HotkeyConflict,
    StorageError,
    DataDirectoryError,
    UpdateError,
    // Pipeline
    InvalidTransition,
    Timeout,
    // Misc
    InvalidInput,
    NetworkError,
    Internal,
}

/// The single error type that bubbles up to the command layer.
#[derive(Debug, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    /// HTTP status of a failed API response, when the error originated from
    /// an HTTP call. Lets the retry logic distinguish permanent client
    /// errors (4xx) from transient server/rate-limit failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            http_status: None,
        }
    }

    /// Attach the originating HTTP status so `util::is_retryable` can
    /// classify the failure as transient or permanent.
    pub fn with_http_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }

    // --- Convenience constructors -----------------------------------------
    pub fn audio_device_not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::MicNotFound, msg)
    }
    pub fn audio_permission_denied() -> Self {
        Self::new(
            ErrorCode::MicPermissionDenied,
            "Microphone permission denied",
        )
    }
    pub fn audio(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::AudioDeviceError, msg)
    }
    pub fn stt(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::SttEngineError, msg)
    }
    pub fn stt_api(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::SttApiError, msg)
    }
    pub fn model_not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::SttModelNotFound, msg)
    }
    pub fn api_key_missing(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::SttApiKeyInvalid, msg)
    }
    pub fn llm_api_key_missing(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::LlmApiKeyInvalid, msg)
    }
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::LlmApiError, msg)
    }
    pub fn llm_connection_refused(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::LlmConnectionRefused, msg)
    }
    pub fn injection(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InjectionFailed, msg)
    }
    pub fn hotkey_conflict(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::HotkeyConflict, msg)
    }
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::StorageError, msg)
    }
    pub fn data_directory(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::DataDirectoryError, msg)
    }
    pub fn network(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::NetworkError, msg)
    }
    pub fn invalid_transition(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidTransition, msg)
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Timeout, msg)
    }
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, msg)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, AppError>;

// --- Conversions from common third-party errors ---------------------------

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::storage(e.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            AppError::timeout(e.to_string())
        } else {
            AppError::network(e.to_string())
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::internal(format!("JSON error: {e}"))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::internal(format!("IO error: {e}"))
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::internal(format!("Tauri error: {e}"))
    }
}
