//! Onboarding microphone level meter.
//!
//! The dictation pipeline only emits `audio_level` while Recording, so the
//! onboarding microphone step has nothing to display. This module owns a
//! second, independent capture that streams the level outside the Recording
//! state. A generation token makes start/stop race-safe: device
//! initialization runs on a blocking thread and the resulting capture is only
//! installed when the request it belongs to is still current.

use std::sync::Mutex;

use super::capture::{ActiveCapture, AudioCaptureImpl};
use crate::util::MutexExt;

struct MicMeterInner {
    /// Whether a meter capture is currently wanted.
    desired: bool,
    /// Bumped on every start/stop; a capture whose generation is no longer
    /// current is superseded or stopped and must be cancelled.
    gen: u64,
    capture: Option<AudioCaptureImpl>,
}

/// Owns the onboarding meter capture and the desired/actual state.
pub struct MicMeterState {
    inner: Mutex<MicMeterInner>,
}

impl Default for MicMeterState {
    fn default() -> Self {
        Self::new()
    }
}

impl MicMeterState {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(MicMeterInner {
                desired: false,
                gen: 0,
                capture: None,
            }),
        }
    }

    /// Mark the meter as wanted and return the generation for a pending start.
    pub fn begin_start(&self) -> u64 {
        let mut inner = self.inner.lock_recover();
        inner.desired = true;
        inner.gen = inner.gen.wrapping_add(1);
        inner.gen
    }

    /// Install a freshly opened capture when it still belongs to the current
    /// start request; otherwise cancel it and report that it was rejected.
    pub fn commit(&self, gen: u64, capture: ActiveCapture) -> bool {
        let mut inner = self.inner.lock_recover();
        if inner.gen != gen || !inner.desired {
            // Drop the lock before joining the capture thread so a concurrent
            // start/stop is not blocked on device teardown.
            drop(inner);
            capture.cancel();
            return false;
        }
        inner
            .capture
            .get_or_insert_with(AudioCaptureImpl::new)
            .install(capture);
        true
    }

    /// Stop the meter: clear the desired flag, invalidate pending starts, and
    /// cancel any installed capture.
    pub fn stop(&self) {
        let mut inner = self.inner.lock_recover();
        inner.desired = false;
        inner.gen = inner.gen.wrapping_add(1);
        if let Some(mut capture) = inner.capture.take() {
            let _ = capture.cancel();
        }
    }

    /// Whether the meter is currently wanted.
    pub fn is_active(&self) -> bool {
        self.inner.lock_recover().desired
    }

    /// The current generation. An emitter spawned for an earlier generation
    /// is superseded once this changes, even if `is_active()` flipped back to
    /// true before the emitter observed the inactive window.
    pub fn generation(&self) -> u64 {
        self.inner.lock_recover().gen
    }

    /// The stored capture's current level, or 0.0 when none is installed.
    pub fn level(&self) -> f32 {
        self.inner
            .lock_recover()
            .capture
            .as_ref()
            .map_or(0.0, |capture| capture.level())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_then_late_commit_is_rejected() {
        let meter = MicMeterState::new();
        let gen = meter.begin_start();
        meter.stop();

        let (capture, finished) = ActiveCapture::dummy();
        assert!(!meter.commit(gen, capture));
        assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn newer_start_supersedes_older_commit() {
        let meter = MicMeterState::new();
        let older = meter.begin_start();
        let newer = meter.begin_start();

        let (newer_capture, newer_finished) = ActiveCapture::dummy();
        assert!(meter.commit(newer, newer_capture));
        assert!(!newer_finished.load(std::sync::atomic::Ordering::SeqCst));

        let (older_capture, older_finished) = ActiveCapture::dummy();
        assert!(!meter.commit(older, older_capture));
        assert!(older_finished.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn generation_changes_on_begin_start_and_stop() {
        let meter = MicMeterState::new();
        let initial = meter.generation();

        let started = meter.begin_start();
        assert_ne!(started, initial, "begin_start must bump the generation");
        assert_eq!(meter.generation(), started);

        meter.stop();
        assert_ne!(meter.generation(), started, "stop must bump the generation");
    }

    #[test]
    fn stop_cancels_active_capture() {
        let meter = MicMeterState::new();
        let gen = meter.begin_start();
        let (capture, finished) = ActiveCapture::dummy();
        assert!(meter.commit(gen, capture));
        assert!(meter.is_active());
        assert!(!finished.load(std::sync::atomic::Ordering::SeqCst));

        meter.stop();

        assert!(!meter.is_active());
        assert!(finished.load(std::sync::atomic::Ordering::SeqCst));
    }
}
