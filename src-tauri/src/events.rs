//! Typed event emitters (backend -> frontend).

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

use crate::pipeline::AppStateTag;

#[derive(Debug, Clone, Serialize)]
pub struct StateChanged {
    pub state: AppStateTag,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionComplete {
    pub id: String,
    pub text: String,
    pub word_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionError {
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioLevel {
    pub level: f32,
}

/// Emit a state change event.
pub fn emit_state<R: Runtime>(app: &AppHandle<R>, state: AppStateTag) {
    let _ = app.emit("state_changed", StateChanged { state });
}

pub fn emit_transcription_complete<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
    word_count: u32,
) {
    let _ = app.emit(
        "transcription_complete",
        TranscriptionComplete {
            id: id.to_string(),
            text: text.to_string(),
            word_count,
        },
    );
}

pub fn emit_transcription_error<R: Runtime>(app: &AppHandle<R>, message: &str, code: &str) {
    let _ = app.emit(
        "transcription_error",
        TranscriptionError {
            message: message.to_string(),
            code: code.to_string(),
        },
    );
}

pub fn emit_audio_level<R: Runtime>(app: &AppHandle<R>, level: f32) {
    let _ = app.emit("audio_level", AudioLevel { level });
}
