//! LLM-based text formatting.
//!
//! - [`rule_based::RuleBasedFormatter`] — regex fallback, always available.
//! - [`ollama::OllamaFormatter`] — local Ollama HTTP.
//! - [`groq_llm::GroqLlmFormatter`] — Groq cloud (OpenAI-compatible).

pub mod factory;
pub mod fallback;
pub mod groq_llm;
pub mod ollama;
pub mod prompts;
pub mod rule_based;
pub mod types;

pub use factory::LlmFactory;
pub use fallback::FallbackFormatter;
pub use types::{GroqLlmConfig, LlmEngineKind, LlmMode, OllamaConfig, RuleBasedConfig};

use async_trait::async_trait;

use crate::error::Result;

/// Formats raw transcribed text according to a [`LlmMode`].
#[async_trait]
pub trait LlmFormatter: Send + Sync {
    async fn format(&self, text: &str, mode: &LlmMode, language: &str) -> Result<String>;
    /// Translate text between languages (used by translation mode).
    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String>;
    /// Name of the engine that produced the last completed `format`/`translate`
    /// output. A wrapper such as [`FallbackFormatter`] reports whichever engine
    /// actually won, so history and telemetry are not misattributed. Before any
    /// call it reports the default (primary) engine.
    fn name(&self) -> &'static str;
}
