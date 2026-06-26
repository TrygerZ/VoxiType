//! Audio capture subsystem.
//!
//! Pipeline: microphone (cpal) -> resampler (48k stereo -> 16k mono) ->
//! ring buffer. The captured mono 16 kHz `f32` samples feed the VAD and STT
//! stages downstream.

pub mod capture;
pub mod device;
pub mod resampler;

pub use capture::{AudioCaptureImpl, AudioConfig};
pub use device::DeviceInfo;

/// Target sample rate for the whole downstream pipeline (Whisper wants 16 kHz mono).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;
