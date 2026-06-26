//! Local whisper.cpp STT engine (feature `local-stt`).
//!
//! Wraps `whisper-rs`. Requires the GGML model to be present on disk and a
//! C/C++ toolchain (cmake + libclang) at build time.
//!
//! NOTE: Only compiled with `--features local-stt`.

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::types::WhisperCppConfig;
use super::{SttConfig, SttEngine, TranscriptionResult};
use crate::error::{AppError, Result};
use crate::util::MutexExt;

pub struct WhisperCppEngine {
    ctx: Mutex<WhisperContext>,
    config: WhisperCppConfig,
}

impl WhisperCppEngine {
    pub fn new(config: WhisperCppConfig) -> Result<Self> {
        let model_path = PathBuf::from(&config.model_path);
        if !model_path.exists() {
            return Err(AppError::model_not_found(format!(
                "Whisper model not found at {}",
                model_path.display()
            )));
        }
        let ctx = WhisperContext::new_with_params(
            &config.model_path,
            WhisperContextParameters::default(),
        )
        .map_err(|e| AppError::stt(format!("Failed to load Whisper model: {e}")))?;

        Ok(Self {
            ctx: Mutex::new(ctx),
            config,
        })
    }
}

#[async_trait]
impl SttEngine for WhisperCppEngine {
    async fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<TranscriptionResult> {
        let started = std::time::Instant::now();
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(self.config.threads as i32);
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let lang = if config.language == "auto" {
            None
        } else {
            Some(config.language.as_str())
        };
        params.set_language(lang);

        if let Some(prompt) = &config.initial_prompt {
            params.set_initial_prompt(prompt);
        }

        let ctx = self.ctx.lock_recover();
        let mut state = ctx
            .create_state()
            .map_err(|e| AppError::stt(format!("Whisper state error: {e}")))?;
        state
            .full(params, audio)
            .map_err(|e| AppError::stt(format!("Whisper transcription failed: {e}")))?;

        let num_segments = state
            .full_n_segments()
            .map_err(|e| AppError::stt(format!("segment count error: {e}")))?;
        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(seg) = state.full_get_segment_text(i) {
                text.push_str(&seg);
            }
        }

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            confidence: 1.0,
            language: config.language.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            raw_response: None,
        })
    }

    fn name(&self) -> &'static str {
        "whisper_cpp"
    }
}
