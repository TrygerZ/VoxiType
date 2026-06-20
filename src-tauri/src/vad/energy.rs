//! RMS-energy based VAD fallback.
//!
//! Not as robust as Silero but dependency-free and good enough to drive
//! start/end-of-utterance segmentation in quiet environments.

use super::{VadConfig, VadEngine, VadResult};
use crate::error::Result;

pub struct EnergyVad {
    config: VadConfig,
    in_speech: bool,
    speech_ms: u32,
    silence_ms: u32,
}

impl EnergyVad {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            in_speech: false,
            speech_ms: 0,
            silence_ms: 0,
        }
    }

    fn rms(window: &[f32]) -> f32 {
        if window.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = window.iter().map(|s| s * s).sum();
        (sum_sq / window.len() as f32).sqrt()
    }
}

impl VadEngine for EnergyVad {
    fn process_window(&mut self, window: &[f32]) -> Result<VadResult> {
        let rms = Self::rms(window);
        // Map RMS to a pseudo-probability. RMS ~0.1+ is clearly speech.
        let probability = (rms * 5.0).min(1.0);
        let is_loud = probability >= self.config.threshold;
        let win_ms = self.config.window_size_ms;

        let mut speech_started = false;
        let mut speech_ended = false;

        if is_loud {
            self.silence_ms = 0;
            self.speech_ms += win_ms;
            if !self.in_speech && self.speech_ms >= self.config.min_speech_duration_ms {
                self.in_speech = true;
                speech_started = true;
            }
        } else {
            self.silence_ms += win_ms;
            self.speech_ms = 0;
            if self.in_speech && self.silence_ms >= self.config.silence_duration_ms {
                self.in_speech = false;
                speech_ended = true;
            }
        }

        Ok(VadResult {
            is_speech: self.in_speech,
            probability,
            speech_started,
            speech_ended,
        })
    }

    fn reset(&mut self) {
        self.in_speech = false;
        self.speech_ms = 0;
        self.silence_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> VadConfig {
        VadConfig {
            min_speech_duration_ms: 60,
            silence_duration_ms: 90,
            window_size_ms: 30,
            ..Default::default()
        }
    }

    #[test]
    fn detects_speech_after_min_duration() {
        let mut vad = EnergyVad::new(cfg());
        let loud = vec![0.3f32; 480];
        vad.process_window(&loud).unwrap(); // 30ms
        let r = vad.process_window(&loud).unwrap(); // 60ms -> start
        assert!(r.speech_started);
        assert!(r.is_speech);
    }

    #[test]
    fn detects_silence_end() {
        let mut vad = EnergyVad::new(cfg());
        let loud = vec![0.3f32; 480];
        let quiet = vec![0.0f32; 480];
        vad.process_window(&loud).unwrap();
        vad.process_window(&loud).unwrap();
        vad.process_window(&quiet).unwrap();
        vad.process_window(&quiet).unwrap();
        let r = vad.process_window(&quiet).unwrap();
        assert!(r.speech_ended);
        assert!(!r.is_speech);
    }
}
