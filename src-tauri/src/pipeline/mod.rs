//! Pipeline orchestration.
//!
//! Owns the [`AppState`] and the audio capture handle. Commands call
//! `start_recording` / `stop_recording`; the heavy STT -> LLM -> injection work
//! runs in [`batch`].

pub mod batch;
pub mod state_machine;

pub use state_machine::{AppState, AppStateTag, StateEvent};

use std::sync::Mutex;

use crate::audio::{AudioCaptureImpl, AudioConfig};
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
            AppState::Recording { start_time, .. } => Some(start_time.elapsed()),
            _ => None,
        }
    }

    pub fn active_app(&self) -> Option<String> {
        let guard = self.state.lock_recover();
        match &*guard {
            AppState::Recording { active_app, .. } => active_app.clone(),
            AppState::Processing { active_app, .. } => active_app.clone(),
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

    /// Start the underlying audio capture stream, but only if the pipeline
    /// is still in the Recording state.
    ///
    /// The capture task runs asynchronously after the Recording transition;
    /// a fast press-and-release may already have cancelled or stopped the
    /// session by the time it executes. Starting the stream then would leave
    /// an orphaned cpal stream running while the app is Idle. The state lock
    /// is held across both the check and the start, so a concurrent
    /// stop/cancel cannot slip between them: it either completes before the
    /// check (start gets skipped) or blocks until the stream is up and then
    /// operates on a live session. Lock order is always state → audio; no
    /// code path acquires them in the reverse order.
    ///
    /// Returns `Ok(false)` when the start was skipped because recording had
    /// already ended.
    pub fn start_capture_if_recording(&self, config: &AudioConfig) -> Result<bool> {
        let guard = self.state.lock_recover();
        if !matches!(*guard, AppState::Recording { .. }) {
            return Ok(false);
        }
        self.audio.lock_recover().start(config)?;
        Ok(true)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn audio_config() -> AudioConfig {
        AudioConfig::default()
    }

    #[test]
    fn capture_start_skipped_when_idle() {
        let pipeline = PipelineOrchestrator::new();
        let started = pipeline
            .start_capture_if_recording(&audio_config())
            .unwrap();
        assert!(!started);
        assert_eq!(pipeline.state_tag(), AppStateTag::Idle);
    }

    #[test]
    fn capture_start_skipped_when_processing() {
        let pipeline = PipelineOrchestrator::new();
        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        // Pure state transition; no live capture session is touched.
        pipeline.apply(StateEvent::StopRecording).unwrap();

        let started = pipeline
            .start_capture_if_recording(&audio_config())
            .unwrap();
        assert!(!started);
        assert_eq!(pipeline.state_tag(), AppStateTag::Processing);
    }

    #[test]
    fn capture_start_skipped_after_cancel() {
        let pipeline = PipelineOrchestrator::new();
        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        pipeline.cancel_recording().unwrap();

        let started = pipeline
            .start_capture_if_recording(&audio_config())
            .unwrap();
        assert!(!started);
        assert_eq!(pipeline.state_tag(), AppStateTag::Idle);
    }

    #[test]
    fn capture_start_skipped_when_error() {
        let pipeline = PipelineOrchestrator::new();
        pipeline.set_error(&AppError::internal("boom"));

        let started = pipeline
            .start_capture_if_recording(&audio_config())
            .unwrap();
        assert!(!started);
        assert_eq!(pipeline.state_tag(), AppStateTag::Error);
    }

    #[test]
    fn cancel_then_error_allows_retry() {
        let pipeline = PipelineOrchestrator::new();
        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        assert_eq!(pipeline.state_tag(), AppStateTag::Recording);

        // Cancel recording cleans up and moves to Idle
        pipeline.cancel_recording().unwrap();
        assert_eq!(pipeline.state_tag(), AppStateTag::Idle);

        // Set error sets state to Error
        pipeline.set_error(&AppError::audio("startup failure"));
        assert_eq!(pipeline.state_tag(), AppStateTag::Error);

        // Retry: Recording -> Error -> Recording succeeds
        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        assert_eq!(pipeline.state_tag(), AppStateTag::Recording);
    }
}
