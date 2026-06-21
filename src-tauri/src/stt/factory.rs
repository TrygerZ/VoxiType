//! STT engine factory.

use std::sync::Arc;

use super::groq_stt::GroqSttEngine;
use super::types::{GroqSttConfig, SttEngineKind, WhisperCppConfig};
use super::SttEngine;
use crate::error::Result;

/// Configuration union for building any STT engine.
pub struct SttFactory;

impl SttFactory {
    /// Build the Groq Whisper engine.
    pub fn groq(config: GroqSttConfig) -> Arc<dyn SttEngine> {
        Arc::new(GroqSttEngine::new(config))
    }

    /// Build the local whisper.cpp engine (requires `local-stt`).
    #[cfg(feature = "local-stt")]
    pub fn whisper_cpp(config: WhisperCppConfig) -> Result<Arc<dyn SttEngine>> {
        Ok(Arc::new(super::whisper_cpp::WhisperCppEngine::new(config)?))
    }

    #[cfg(not(feature = "local-stt"))]
    pub fn whisper_cpp(_config: WhisperCppConfig) -> Result<Arc<dyn SttEngine>> {
        Err(crate::error::AppError::stt(
            "Local Whisper.cpp STT is not available in this build (rebuild with --features local-stt)",
        ))
    }

    /// Create an engine from a kind, falling back to Groq when local STT is
    /// unavailable.
    pub fn create(
        kind: SttEngineKind,
        whisper: WhisperCppConfig,
        groq: GroqSttConfig,
    ) -> Result<Arc<dyn SttEngine>> {
        match kind {
            SttEngineKind::Groq => Ok(Self::groq(groq)),
            SttEngineKind::WhisperCpp => match Self::whisper_cpp(whisper) {
                Ok(engine) => Ok(engine),
                Err(e) => {
                    if groq.api_key.trim().is_empty() {
                        return Err(crate::error::AppError::model_not_found(
                            "Local Whisper.cpp is not available in this build and no Groq API key is set. \
                             Add a Groq API key in Settings or use a build with --features local-stt.",
                        ));
                    }
                    tracing::warn!("Whisper.cpp unavailable ({e}); falling back to Groq");
                    Ok(Self::groq(groq))
                }
            },
        }
    }
}
