//! Pipeline orchestration.
//!
//! Owns the [`AppState`] and the audio capture handle. Commands call
//! `start_recording` / `stop_recording`; the heavy STT -> LLM -> injection work
//! runs in [`batch`].

pub mod batch;
pub mod state_machine;

pub use state_machine::{AppState, AppStateTag, StateEvent};

use std::sync::Mutex;

use crate::audio::{AudioCapture, AudioCaptureImpl, AudioConfig};
use crate::error::{AppError, Result};

/// Central pipeline coordinator. Stored in Tauri managed state.
pub struct PipelineOrchestrator {
    state: Mutex<AppState>,
    audio: Mutex<AudioCaptureImpl>,
}

impl Default for PipelineOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineOrchestrator {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(AppState::Idle),
            audio: Mutex::new(AudioCaptureImpl::new()),
        }
    }

    /// Current serializable state tag.
    pub fn state_tag(&self) -> AppStateTag {
        self.state.lock().unwrap().tag()
    }

    /// Current audio input level (0.0 - 1.0).
    pub fn audio_level(&self) -> f32 {
        self.audio.lock().unwrap().level()
    }

    /// Apply a state event, mutating the stored state in place.
    pub fn apply(&self, event: StateEvent) -> Result<AppStateTag> {
        let mut guard = self.state.lock().unwrap();
        // Replace via take/transition. `Idle` is a cheap placeholder.
        let current = std::mem::replace(&mut *guard, AppState::Idle);
        match current.transition(event) {
            Ok(next) => {
                let tag = next.tag();
                *guard = next;
                Ok(tag)
            }
            Err(e) => {
                // Restore Idle on invalid transition (guard already holds Idle).
                Err(e)
            }
        }
    }

    /// Begin capturing audio.
    pub fn start_recording(&self, config: &AudioConfig) -> Result<AppStateTag> {
        let tag = self.apply(StateEvent::StartRecording)?;
        if let Err(e) = self.audio.lock().unwrap().start(config) {
            // Roll back to Idle on capture failure.
            let _ = self.apply(StateEvent::CancelRecording);
            return Err(e);
        }
        Ok(tag)
    }

    /// Stop capturing and return the captured samples, moving to Processing.
    pub fn stop_recording(&self) -> Result<Vec<f32>> {
        let samples = self.audio.lock().unwrap().stop()?;
        self.apply(StateEvent::StopRecording)?;
        Ok(samples)
    }

    /// Cancel an in-progress recording.
    pub fn cancel_recording(&self) -> Result<()> {
        self.audio.lock().unwrap().cancel()?;
        self.apply(StateEvent::CancelRecording)?;
        Ok(())
    }

    /// Mark processing as finished.
    pub fn finish_processing(&self) -> Result<()> {
        self.apply(StateEvent::ProcessingComplete)?;
        Ok(())
    }

    /// Move to error state.
    pub fn set_error(&self, err: &AppError) {
        let _ = self.apply(StateEvent::Error {
            message: err.message.clone(),
            code: err.code,
        });
    }
}
