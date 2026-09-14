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
    pub duration_ms: i64,
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

#[derive(Debug, Clone, Serialize)]
pub struct WidgetHideRequested {
    pub id: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WidgetRevealRequested {
    pub id: u64,
}

/// Emit a state change event.
pub fn emit_state<R: Runtime>(app: &AppHandle<R>, state: AppStateTag) {
    if let Err(e) = app.emit("state_changed", StateChanged { state }) {
        tracing::warn!("Failed to emit state_changed: {e}");
    }
}

pub fn emit_transcription_complete<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
    word_count: u32,
    duration_ms: i64,
) {
    if let Err(e) = app.emit(
        "transcription_complete",
        TranscriptionComplete {
            id: id.to_string(),
            text: text.to_string(),
            word_count,
            duration_ms,
        },
    ) {
        tracing::warn!("Failed to emit transcription_complete: {e}");
    }
}

pub fn emit_transcription_error<R: Runtime>(app: &AppHandle<R>, message: &str, code: &str) {
    if let Err(e) = app.emit(
        "transcription_error",
        TranscriptionError {
            message: message.to_string(),
            code: code.to_string(),
        },
    ) {
        tracing::warn!("Failed to emit transcription_error: {e}");
    }
}

pub fn emit_audio_level<R: Runtime>(app: &AppHandle<R>, level: f32) {
    if let Err(e) = app.emit("audio_level", AudioLevel { level }) {
        tracing::warn!("Failed to emit audio_level: {e}");
    }
}

pub fn emit_widget_hide_requested<R: Runtime>(app: &AppHandle<R>, id: u64) {
    if let Err(e) = app.emit("floating_widget_hide_requested", WidgetHideRequested { id }) {
        tracing::warn!("Failed to emit floating_widget_hide_requested: {e}");
    }
}

pub fn emit_widget_reveal_requested<R: Runtime>(app: &AppHandle<R>, id: u64) {
    if let Err(e) = app.emit(
        "floating_widget_reveal_requested",
        WidgetRevealRequested { id },
    ) {
        tracing::warn!("Failed to emit floating_widget_reveal_requested: {e}");
    }
}
