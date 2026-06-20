//! Audio capture subsystem.
//!
//! Pipeline: microphone (cpal) -> resampler (48k stereo -> 16k mono) ->
//! ring buffer. The captured mono 16 kHz `f32` samples feed the VAD and STT
//! stages downstream.

pub mod capture;
pub mod device;
pub mod resampler;
pub mod ring_buffer;

pub use capture::{AudioCaptureImpl, AudioConfig};
pub use device::DeviceInfo;

use crate::error::Result;

/// Target sample rate for the whole downstream pipeline (Whisper/Silero want 16 kHz mono).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Abstraction over a microphone capture backend.
pub trait AudioCapture: Send {
    /// Begin capturing. Returns immediately; samples accumulate internally.
    fn start(&mut self, config: &AudioConfig) -> Result<()>;
    /// Stop capturing and return the captured 16 kHz mono samples.
    fn stop(&mut self) -> Result<Vec<f32>>;
    /// Discard any in-progress capture without returning audio.
    fn cancel(&mut self) -> Result<()>;
    /// Current normalized input level (0.0 - 1.0) for UI metering.
    fn level(&self) -> f32;
    /// Whether a capture is currently in progress.
    fn is_active(&self) -> bool;
    /// Enumerate available input devices.
    fn device_list(&self) -> Result<Vec<DeviceInfo>>;
}
