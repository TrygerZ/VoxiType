//! Silero VAD via ONNX Runtime (feature `local-vad`).
//!
//! Loads `silero_vad.onnx` and runs per-window inference, maintaining the
//! model's recurrent state across calls. Requires the `ort` prebuilt runtime.
//!
//! NOTE: This module only compiles with `--features local-vad`. The default
//! build uses [`super::energy::EnergyVad`].

use std::path::Path;

use ndarray::{Array1, Array2, Array3};
use ort::session::Session;
use ort::value::Tensor;

use super::{VadConfig, VadEngine, VadResult};
use crate::error::{AppError, Result};

pub struct SileroVad {
    session: Session,
    config: VadConfig,
    state: Array3<f32>,
    in_speech: bool,
    speech_ms: u32,
    silence_ms: u32,
}

impl SileroVad {
    pub fn new(config: VadConfig, model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            return Err(AppError::model_not_found(format!(
                "Silero VAD model not found at {}",
                model_path.display()
            )));
        }
        let session = Session::builder()
            .map_err(|e| AppError::vad(format!("ORT session builder failed: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| AppError::vad(format!("Failed to load Silero model: {e}")))?;

        Ok(Self {
            session,
            config,
            // Silero v5 state shape: [2, 1, 128].
            state: Array3::<f32>::zeros((2, 1, 128)),
            in_speech: false,
            speech_ms: 0,
            silence_ms: 0,
        })
    }

    fn infer(&mut self, window: &[f32]) -> Result<f32> {
        let samples = Array2::from_shape_vec((1, window.len()), window.to_vec())
            .map_err(|e| AppError::vad(format!("Input shape error: {e}")))?;
        let sr = Array1::from_vec(vec![self.config.sample_rate as i64]);

        let input = Tensor::from_array(samples)
            .map_err(|e| AppError::vad(format!("Tensor build failed: {e}")))?;
        let state = Tensor::from_array(self.state.clone())
            .map_err(|e| AppError::vad(format!("State tensor failed: {e}")))?;
        let sr_tensor =
            Tensor::from_array(sr).map_err(|e| AppError::vad(format!("SR tensor failed: {e}")))?;

        let outputs = self
            .session
            .run(ort::inputs![
                "input" => input,
                "state" => state,
                "sr" => sr_tensor,
            ])
            .map_err(|e| AppError::vad(format!("VAD inference failed: {e}")))?;

        let (_, prob_data) = outputs["output"]
            .try_extract_tensor::<f32>()
            .map_err(|e| AppError::vad(format!("Output extract failed: {e}")))?;
        let probability = prob_data.first().copied().unwrap_or(0.0);

        if let Ok((_, new_state)) = outputs["stateN"].try_extract_tensor::<f32>() {
            if new_state.len() == self.state.len() {
                self.state = Array3::from_shape_vec((2, 1, 128), new_state.to_vec())
                    .unwrap_or_else(|_| self.state.clone());
            }
        }

        Ok(probability)
    }
}

impl VadEngine for SileroVad {
    fn process_window(&mut self, window: &[f32]) -> Result<VadResult> {
        let probability = self.infer(window)?;
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
        self.state = Array3::<f32>::zeros((2, 1, 128));
        self.in_speech = false;
        self.speech_ms = 0;
        self.silence_ms = 0;
    }
}
