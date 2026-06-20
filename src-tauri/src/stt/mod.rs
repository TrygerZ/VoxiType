//! Speech-to-text engines.
//!
//! - [`groq_stt::GroqSttEngine`] — Groq Whisper REST (always available).
//! - `whisper_cpp::WhisperCppEngine` — local whisper.cpp (feature `local-stt`).

pub mod factory;
pub mod groq_stt;
pub mod types;
#[cfg(feature = "local-stt")]
pub mod whisper_cpp;

pub use factory::SttFactory;
pub use types::{
    GroqSttConfig, SttConfig, SttEngineKind, TranscriptionResult, WhisperCppConfig, WordTiming,
};

use async_trait::async_trait;

use crate::error::Result;

/// Transcribes 16 kHz mono `f32` audio into text.
#[async_trait]
pub trait SttEngine: Send + Sync {
    async fn transcribe(&self, audio: &[f32], config: &SttConfig) -> Result<TranscriptionResult>;
    /// Human-readable engine name for logging / history.
    fn name(&self) -> &'static str;
}
