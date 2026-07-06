//! Speech-to-text engines — Groq Whisper REST.

pub mod factory;
pub mod groq_stt;
pub mod types;
pub mod whisper_cpp;

pub use factory::SttFactory;
pub use types::{GroqSttConfig, SttConfig, SttEngineKind, TranscriptionResult, WhisperCppConfig};

use async_trait::async_trait;

use crate::error::Result;

/// Transcribes 16 kHz mono `f32` audio into text.
#[async_trait]
pub trait SttEngine: Send + Sync {
    async fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<TranscriptionResult>;
    /// Human-readable engine name for logging / history.
    fn name(&self) -> &'static str;
}
