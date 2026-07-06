//! STT engine factory.

use std::sync::Arc;

use super::groq_stt::GroqSttEngine;
use super::types::{GroqSttConfig, SttEngineKind, WhisperCppConfig};
use super::whisper_cpp::WhisperCppEngine;
use super::SttEngine;

/// Builds speech-to-text engines.
pub struct SttFactory;

impl SttFactory {
    pub fn groq(config: GroqSttConfig) -> Arc<dyn SttEngine> {
        Arc::new(GroqSttEngine::new(config))
    }

    pub fn whisper_cpp(config: WhisperCppConfig) -> Arc<dyn SttEngine> {
        Arc::new(WhisperCppEngine::new(config))
    }

    pub fn create(
        kind: SttEngineKind,
        groq: GroqSttConfig,
        whisper_cpp: WhisperCppConfig,
    ) -> Arc<dyn SttEngine> {
        match kind {
            SttEngineKind::Groq => Self::groq(groq),
            SttEngineKind::WhisperCpp => Self::whisper_cpp(whisper_cpp),
        }
    }
}
