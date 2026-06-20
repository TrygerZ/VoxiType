//! Voice Activity Detection configuration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub enabled: bool,
    /// Speech probability threshold (0.0 - 1.0).
    pub threshold: f32,
    /// Silence duration before declaring end-of-utterance.
    pub silence_duration_ms: u32,
    /// Minimum speech duration before declaring speech start.
    pub min_speech_duration_ms: u32,
    /// Analysis window size in milliseconds.
    pub window_size_ms: u32,
    pub sample_rate: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 0.5,
            silence_duration_ms: 500,
            min_speech_duration_ms: 200,
            window_size_ms: 30,
            sample_rate: 16_000,
        }
    }
}

impl VadConfig {
    /// Number of samples in one analysis window.
    pub fn window_samples(&self) -> usize {
        (self.sample_rate as usize * self.window_size_ms as usize) / 1000
    }
}
