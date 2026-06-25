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
use crate::util::MutexExt;

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
        self.state.lock_recover().tag()
    }

    /// Current audio input level (0.0 - 1.0).
    pub fn audio_level(&self) -> f32 {
        self.audio.lock_recover().level()
    }

    /// Get the elapsed duration of the current recording, if in Recording state.
    pub fn recording_duration(&self) -> Option<std::time::Duration> {
        let guard = self.state.lock_recover();
        match &*guard {
            AppState::Recording { start_time } => Some(start_time.elapsed()),
            _ => None,
        }
    }

    /// Apply a state event, mutating the stored state in place.
    pub fn apply(&self, event: StateEvent) -> Result<AppStateTag> {
        let mut guard = self.state.lock_recover();
        let current = std::mem::replace(&mut *guard, AppState::Idle);
        match current.transition(event) {
            Ok(next) => {
                let tag = next.tag();
                *guard = next;
                Ok(tag)
            }
            Err((original, e)) => {
                *guard = original;
                Err(e)
            }
        }
    }

    /// Start the underlying audio capture stream.
    pub fn start_capture(&self, config: &AudioConfig) -> Result<()> {
        self.audio.lock_recover().start(config)
    }

    /// Stop capturing and return the captured samples, moving to Processing.
    pub fn stop_recording(&self) -> Result<Vec<f32>> {
        self.apply(StateEvent::StopRecording)?;
        let samples = self.audio.lock_recover().stop()?;
        Ok(samples)
    }

    /// Cancel an in-progress recording.
    pub fn cancel_recording(&self) -> Result<()> {
        self.apply(StateEvent::CancelRecording)?;
        self.audio.lock_recover().cancel()?;
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
