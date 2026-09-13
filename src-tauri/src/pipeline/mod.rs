//! Pipeline orchestration.
//!
//! Owns the [`AppState`] and the audio capture handle. Commands call
//! `start_recording` / `stop_recording`; the heavy STT -> LLM -> injection work
//! runs in [`batch`].

pub mod batch;
pub mod state_machine;

pub use state_machine::{AppState, AppStateTag, StateEvent};

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::audio::{AudioCaptureImpl, AudioConfig};
use crate::error::{AppError, Result};
use crate::util::MutexExt;

/// Central pipeline coordinator. Stored in Tauri managed state.
pub struct PipelineOrchestrator {
    state: Mutex<AppState>,
    audio: Mutex<AudioCaptureImpl>,
    generation: AtomicU64,
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
            generation: AtomicU64::new(0),
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
                self.generation.fetch_add(1, Ordering::SeqCst);
                Ok(tag)
            }
            Err((original, e)) => {
                *guard = original;
                Err(e)
            }
        }
    }

    fn verify_session(&self, target_gen: u64) -> bool {
        let guard = self.state.lock_recover();
        matches!(*guard, AppState::Recording { .. })
            && self.generation.load(Ordering::SeqCst) == target_gen
    }

    /// Start the underlying audio capture stream, but only if the pipeline
    /// is still in the Recording state.
    ///
    /// The state lock is NOT held during blocking device initialization.
    /// A generation token tracks the session identity; after initialization,
    /// the session is re-verified and cancelled if stop/cancel occurred meanwhile.
    ///
    /// Returns `Ok(false)` when the start was skipped or cancelled because
    /// recording had already ended.
    pub fn start_capture_if_recording(&self, config: &AudioConfig) -> Result<bool> {
        let target_gen = {
            let guard = self.state.lock_recover();
            if !matches!(*guard, AppState::Recording { .. }) {
                return Ok(false);
            }
            self.generation.load(Ordering::SeqCst)
        };

        let active_capture = match AudioCaptureImpl::open_stream(config) {
            Ok(capture) => capture,
            Err(e) => {
                if self.verify_session(target_gen) {
                    return Err(e);
                }
                tracing::debug!("Suppressed error on aborted session: {e}");
                return Ok(false);
            }
        };

        if !self.verify_session(target_gen) {
            active_capture.cancel();
            return Ok(false);
        }

        self.audio.lock_recover().install(active_capture);
        Ok(true)
    }

    #[cfg(test)]
    pub fn current_generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
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

    #[test]
    fn generation_advances_on_state_transitions() {
        let pipeline = PipelineOrchestrator::new();
        assert_eq!(pipeline.current_generation(), 0);

        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        assert_eq!(pipeline.current_generation(), 1);

        pipeline.apply(StateEvent::StopRecording).unwrap();
        assert_eq!(pipeline.current_generation(), 2);

        pipeline.finish_processing().unwrap();
        assert_eq!(pipeline.current_generation(), 3);
    }

    #[test]
    fn session_aborted_when_generation_changes() {
        let pipeline = PipelineOrchestrator::new();
        pipeline
            .apply(StateEvent::StartRecording { active_app: None })
            .unwrap();
        let target_gen = pipeline.current_generation();
        assert!(pipeline.verify_session(target_gen));

        pipeline.cancel_recording().unwrap();
        assert!(!pipeline.verify_session(target_gen));
    }
}
