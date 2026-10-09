//! Audio capture subsystem.
//!
//! Pipeline: microphone (cpal) -> resampler (48k stereo -> 16k mono) ->
//! ring buffer. The captured mono 16 kHz `f32` samples feed the VAD and STT
//! stages downstream.

pub mod capture;
pub mod device;
pub mod file_decode;
pub mod meter;
pub mod resampler;

pub use capture::{
    has_enough_voice, ActiveCapture, AudioCaptureImpl, AudioConfig, MAX_CAPTURE_SECS,
};
pub use device::DeviceInfo;
pub use meter::MicMeterState;

/// Target sample rate for the whole downstream pipeline (Whisper wants 16 kHz mono).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;
