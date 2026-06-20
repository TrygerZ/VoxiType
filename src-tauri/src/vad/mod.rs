//! Voice Activity Detection.
//!
//! Two implementations:
//! - [`energy::EnergyVad`] — always available, RMS-energy based.
//! - `silero::SileroVad` — Silero ONNX model (feature `local-vad`).

pub mod config;
pub mod energy;
#[cfg(feature = "local-vad")]
pub mod silero;

pub use config::VadConfig;

use crate::error::Result;

/// Result of analyzing one audio window.
#[derive(Debug, Clone, Copy)]
pub struct VadResult {
    pub is_speech: bool,
    pub probability: f32,
    pub speech_started: bool,
    pub speech_ended: bool,
}

/// A stateful voice-activity detector consuming fixed-size windows.
pub trait VadEngine: Send {
    /// Analyze one window of mono 16 kHz samples.
    fn process_window(&mut self, window: &[f32]) -> Result<VadResult>;
    /// Reset internal state between utterances.
    fn reset(&mut self);
}

/// Build the best available VAD for the given config.
///
/// Falls back to the energy detector when the Silero feature is disabled.
pub fn create_vad(config: VadConfig, model_path: Option<&std::path::Path>) -> Box<dyn VadEngine> {
    #[cfg(feature = "local-vad")]
    {
        if let Some(path) = model_path {
            match silero::SileroVad::new(config.clone(), path) {
                Ok(vad) => return Box::new(vad),
                Err(e) => tracing::warn!("Silero VAD unavailable, using energy VAD: {e}"),
            }
        }
    }
    let _ = model_path;
    Box::new(energy::EnergyVad::new(config))
}
