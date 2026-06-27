//! STT engine factory — Groq only.

use std::sync::Arc;

use super::groq_stt::GroqSttEngine;
use super::types::{GroqSttConfig, SttEngineKind};
use super::SttEngine;

/// Builds Groq Whisper engine.
pub struct SttFactory;

impl SttFactory {
    pub fn groq(config: GroqSttConfig) -> Arc<dyn SttEngine> {
        Arc::new(GroqSttEngine::new(config))
    }

    pub fn create(_kind: SttEngineKind, groq: GroqSttConfig) -> Arc<dyn SttEngine> {
        Self::groq(groq)
    }
}
