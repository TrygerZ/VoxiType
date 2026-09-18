//! cpal-based microphone capture.
//!
//! `cpal::Stream` is `!Send`, so it is built and owned on a dedicated thread.
//! The public [`AudioCaptureImpl`] only holds `Send` handles (a shared state
//! `Arc` and a stop channel), which keeps the pipeline / Tauri managed state
//! `Send + Sync`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::SampleFormat;
use serde::{Deserialize, Serialize};

use super::device::resolve_device;
use super::resampler::Resampler;
use super::TARGET_SAMPLE_RATE;
use crate::error::{AppError, Result};
use crate::util::MutexExt;

/// ~2% decay per frame for smooth UI meter without erratic jumps
const LEVEL_DECAY_FACTOR: f32 = 0.98;

/// Runtime configuration for a capture session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub mic_device: String,
    pub input_gain: f32,
    pub sample_rate: u32,
    pub buffer_size_ms: u32,
    pub noise_gate_threshold: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            mic_device: "default".to_string(),
            input_gain: 1.0,
            sample_rate: 48_000,
            buffer_size_ms: 50,
            // Very low so quiet speech is never gated. STT handles silence
            // itself; the gate only drops near-dead-silent blocks.
            noise_gate_threshold: 0.005,
        }
    }
}

/// Shared capture state mutated from the cpal audio callback thread.
struct Shared {
    ring: Mutex<Vec<f32>>,
    resampler: Mutex<Resampler>,
    level: AtomicU32,
    gain: f32,
    noise_gate: f32,
}

impl Shared {
    fn set_level(&self, level: f32) {
        self.level.store(level.to_bits(), Ordering::Relaxed);
    }
    fn get_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }
}

/// cpal-based microphone capture implementation.
pub struct AudioCaptureImpl {
    shared: Option<Arc<Shared>>,
    stop_tx: Option<Sender<()>>,
    handle: Option<JoinHandle<()>>,
    active: bool,
}

impl Default for AudioCaptureImpl {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioCaptureImpl {
    pub fn new() -> Self {
        Self {
            shared: None,
            stop_tx: None,
            handle: None,
            active: false,
        }
    }
}

/// Build the cpal input stream feeding `shared`.
fn build_stream(
    device: cpal::Device,
    supported: cpal::SupportedStreamConfig,
    shared: Arc<Shared>,
) -> Result<cpal::Stream> {
    let stream_config: cpal::StreamConfig = supported.config();
    let err_fn = |e| tracing::error!("Audio stream error: {e}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| process_samples(&shared, data),
            err_fn,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                let floats: Vec<f32> = data
                    .iter()
                    .map(|&s| {
                        // Use i16::MAX + 1 to make the range symmetric: both
                        // i16::MIN and i16::MAX map to exactly ±1.0.
                        s as f32 / (i16::MAX as f32 + 1.0)
                    })
                    .collect();
                process_samples(&shared, &floats)
            },
            err_fn,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _| {
                let floats: Vec<f32> = data
                    .iter()
                    .map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
                    .collect();
                process_samples(&shared, &floats)
            },
            err_fn,
            None,
        ),
        other => {
            return Err(AppError::audio(format!(
                "Unsupported sample format: {other:?}"
            )))
        }
    }
    .map_err(|e| AppError::audio(format!("Failed to build input stream: {e}")))?;

    Ok(stream)
}

/// Process a callback's worth of interleaved samples into the ring buffer.
fn process_samples(shared: &Arc<Shared>, data: &[f32]) {
    // Compute the peak of the gain-applied block for both metering and gating.
    let mut peak = 0.0f32;
    for &s in data {
        let a = (s * shared.gain).abs();
        if a > peak {
            peak = a;
        }
    }

    // Peak-hold decay: smooth level changes, especially during silent blocks,
    // to match polling interval and prevent erratic UI waveform behavior.
    let current = shared.get_level();
    let new_level = if peak > current {
        peak
    } else {
        current * LEVEL_DECAY_FACTOR
    };
    shared.set_level(new_level.min(1.0));

    // Block-level noise gate: if the whole block is quieter than the threshold
    // treat it as silence. NEVER gate per-sample — speech waveforms cross zero
    // constantly, so zeroing individual samples below the threshold shreds the
    // signal and garbles transcription.
    let block_is_silent = peak < shared.noise_gate;

    let processed: Vec<f32> = if block_is_silent {
        vec![0.0; data.len()]
    } else {
        data.iter()
            .map(|&s| (s * shared.gain).clamp(-1.0, 1.0))
            .collect()
    };

    let mut resampler = shared.resampler.lock_recover();
    match resampler.process(&processed) {
        Ok(out) => {
            drop(resampler);
            if !out.is_empty() {
                shared.ring.lock_recover().extend_from_slice(&out);
            }
        }
        // Don't silently swallow: a dropped block is lost audio, so surface it.
        Err(e) => tracing::warn!("Resampler dropped an audio block: {e}"),
    }
}

/// An initialized capture stream before installation into [`AudioCaptureImpl`].
pub struct ActiveCapture {
    shared: Arc<Shared>,
    stop_tx: mpsc::Sender<()>,
    handle: std::thread::JoinHandle<()>,
}

impl ActiveCapture {
    /// Cancel and join the capture thread immediately.
    pub fn cancel(self) {
        let _ = self.stop_tx.send(());
        let _ = self.handle.join();
    }
}

#[cfg(test)]
impl ActiveCapture {
    pub(crate) fn dummy() -> (Self, Arc<std::sync::atomic::AtomicBool>) {
        let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let finished_clone = finished.clone();
        let (stop_tx, stop_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let _ = stop_rx.recv();
            finished_clone.store(true, Ordering::SeqCst);
        });
        let resampler = Resampler::new(16_000, TARGET_SAMPLE_RATE, 1).expect("resampler");
        let shared = Arc::new(Shared {
            ring: Mutex::new(Vec::new()),
            resampler: Mutex::new(resampler),
            level: AtomicU32::new(0),
            gain: 1.0,
            noise_gate: 0.0,
        });
        (
            ActiveCapture {
                shared,
                stop_tx,
                handle,
            },
            finished,
        )
    }
}

fn spawn_capture_thread(
    device: cpal::Device,
    supported: cpal::SupportedStreamConfig,
    shared: Arc<Shared>,
) -> (
    mpsc::Receiver<Result<()>>,
    mpsc::Sender<()>,
    std::thread::JoinHandle<()>,
) {
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let handle = std::thread::spawn(move || {
        let stream = match build_stream(device, supported, shared) {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(AppError::audio(format!("Failed to start stream: {e}"))));
            return;
        }
        let _ = ready_tx.send(Ok(()));
        let _ = stop_rx.recv();
        let _ = stream.pause();
    });

    (ready_rx, stop_tx, handle)
}

fn wait_for_ready(
    ready_rx: mpsc::Receiver<Result<()>>,
    handle: std::thread::JoinHandle<()>,
) -> Result<std::thread::JoinHandle<()>> {
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(handle),
        Ok(Err(e)) => {
            let _ = handle.join();
            Err(e)
        }
        Err(_) => Err(AppError::audio("Audio thread terminated unexpectedly")),
    }
}

impl AudioCaptureImpl {
    /// Initialize audio device and stream without mutating instance state.
    pub fn open_stream(config: &AudioConfig) -> Result<ActiveCapture> {
        let device = resolve_device(&config.mic_device)?;
        let supported = device
            .default_input_config()
            .map_err(|e| AppError::audio(format!("No default input config: {e}")))?;
        let in_rate = supported.sample_rate().0;
        let channels = supported.channels();

        let resampler = Resampler::new(in_rate, TARGET_SAMPLE_RATE, channels)?;
        let shared = Arc::new(Shared {
            ring: Mutex::new(Vec::new()),
            resampler: Mutex::new(resampler),
            level: AtomicU32::new(0),
            gain: config.input_gain,
            noise_gate: config.noise_gate_threshold,
        });

        let (ready_rx, stop_tx, handle) = spawn_capture_thread(device, supported, shared.clone());
        let handle = wait_for_ready(ready_rx, handle)?;

        Ok(ActiveCapture {
            shared,
            stop_tx,
            handle,
        })
    }

    /// Install an active capture handle into this instance.
    pub fn install(&mut self, capture: ActiveCapture) {
        if self.active || self.handle.is_some() {
            // Cancel and join prior capture to prevent orphaned threads or racing resamplers.
            let _ = self.cancel();
        }
        self.shared = Some(capture.shared);
        self.stop_tx = Some(capture.stop_tx);
        self.handle = Some(capture.handle);
        self.active = true;
    }

    /// Begin capturing. Returns immediately; samples accumulate internally.
    pub fn start(&mut self, config: &AudioConfig) -> Result<()> {
        if self.active {
            return Ok(());
        }
        let capture = Self::open_stream(config)?;
        self.install(capture);
        Ok(())
    }

    /// Stop capturing and return the captured 16 kHz mono samples.
    pub fn stop(&mut self) -> Result<Vec<f32>> {
        self.active = false;
        // Signal the capture thread to stop, then *join it* before touching the
        // resampler/ring. The cpal callback runs on that thread, so joining
        // guarantees no `process_samples` call can race the flush below —
        // otherwise the final audio block could be dropped or the resampler
        // tail corrupted mid-`process`.
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        let samples = if let Some(shared) = self.shared.take() {
            let tail = shared.resampler.lock_recover().flush().unwrap_or_default();
            let mut ring = shared.ring.lock_recover();
            ring.extend_from_slice(&tail);
            std::mem::take(&mut *ring)
        } else {
            Vec::new()
        };
        Ok(trim_silence(&samples, TARGET_SAMPLE_RATE))
    }

    /// Discard any in-progress capture without returning audio.
    pub fn cancel(&mut self) -> Result<()> {
        self.active = false;
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        // Join so the stream is fully paused/dropped before we release `shared`;
        // avoids a lingering callback writing into a soon-to-be-freed buffer.
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        self.shared = None;
        Ok(())
    }

    /// Check whether a capture session is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Current normalized input level (0.0 - 1.0) for UI metering.
    pub fn level(&self) -> f32 {
        self.shared.as_ref().map(|s| s.get_level()).unwrap_or(0.0)
    }
}

/// Trim leading/trailing near-silence, keeping ~250ms of padding on each side.
pub fn trim_silence(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    const THRESHOLD: f32 = 0.01;
    let pad = (sample_rate as usize) / 4; // 250 ms

    let first = samples.iter().position(|&s| s.abs() > THRESHOLD);
    let last = samples.iter().rposition(|&s| s.abs() > THRESHOLD);

    match (first, last) {
        (Some(f), Some(l)) => {
            let start = f.saturating_sub(pad);
            let end = (l + pad).min(samples.len());
            samples[start..end].to_vec()
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_silence_keeps_padding() {
        let mut samples = vec![0.0f32; 16_000];
        for s in samples.iter_mut().skip(8000).take(100) {
            *s = 0.5;
        }
        let trimmed = trim_silence(&samples, 16_000);
        assert!(!trimmed.is_empty());
        assert!(trimmed.len() < samples.len());
    }

    #[test]
    fn trim_silence_handles_all_silent() {
        let samples = vec![0.0f32; 100];
        let trimmed = trim_silence(&samples, 16_000);
        assert!(trimmed.is_empty());
    }

    #[test]
    fn test_audio_level_decay() {
        let resampler = Resampler::new(16_000, TARGET_SAMPLE_RATE, 1).unwrap();
        let shared = Arc::new(Shared {
            ring: Mutex::new(Vec::new()),
            resampler: Mutex::new(resampler),
            level: AtomicU32::new(0),
            gain: 1.0,
            noise_gate: 0.0,
        });

        assert_eq!(shared.get_level(), 0.0);

        // Process high amplitude
        process_samples(&shared, &[0.5]);
        let level1 = shared.get_level();
        assert!(level1 > 0.0);

        // Process silence - level should decay, not drop to 0 instantly
        process_samples(&shared, &[0.0]);
        let level2 = shared.get_level();
        assert!(level2 > 0.0);
        assert!(level2 < level1);
    }

    #[test]
    fn install_cancels_and_joins_previous_active_capture() {
        let mut capture_impl = AudioCaptureImpl::new();
        let (cap1, cap1_finished) = ActiveCapture::dummy();
        capture_impl.install(cap1);
        assert!(capture_impl.is_active());
        assert!(!cap1_finished.load(Ordering::SeqCst));

        let (cap2, cap2_finished) = ActiveCapture::dummy();
        capture_impl.install(cap2);
        assert!(capture_impl.is_active());
        // Previous capture thread must be cancelled and joined immediately
        assert!(cap1_finished.load(Ordering::SeqCst));
        assert!(!cap2_finished.load(Ordering::SeqCst));

        capture_impl.cancel().unwrap();
        assert!(!capture_impl.is_active());
        assert!(cap2_finished.load(Ordering::SeqCst));
    }
}
