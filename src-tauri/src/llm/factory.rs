//! LLM formatter factory.

use std::sync::Arc;

use async_trait::async_trait;

use super::fallback::FallbackFormatter;
use super::groq_llm::GroqLlmFormatter;
use super::ollama::OllamaFormatter;
use super::rule_based::RuleBasedFormatter;
use super::types::{GroqLlmConfig, LlmEngineKind, OllamaConfig, RuleBasedConfig};
use super::LlmFormatter;
use crate::error::Result;

struct OffFormatter;

#[async_trait]
impl LlmFormatter for OffFormatter {
    async fn format(
        &self,
        text: &str,
        _mode: &super::types::LlmMode,
        _language: &str,
    ) -> Result<String> {
        Ok(text.to_string())
    }

    async fn translate(&self, text: &str, _source: &str, _target: &str) -> Result<String> {
        Ok(text.to_string())
    }

    fn name(&self) -> &'static str {
        "off"
    }

    fn supports_translation(&self) -> bool {
        false
    }
}

pub struct LlmFactory;

impl LlmFactory {
    pub fn off() -> Arc<dyn LlmFormatter> {
        Arc::new(OffFormatter)
    }

    pub fn rule_based(config: RuleBasedConfig) -> Arc<dyn LlmFormatter> {
        Arc::new(RuleBasedFormatter::new(config))
    }

    pub fn ollama(config: OllamaConfig) -> Result<Arc<dyn LlmFormatter>> {
        Ok(Arc::new(OllamaFormatter::new(config)?))
    }

    pub fn groq(config: GroqLlmConfig) -> Result<Arc<dyn LlmFormatter>> {
        Ok(Arc::new(GroqLlmFormatter::new(config)?))
    }

    /// Build a formatter for the selected engine.
    ///
    /// `Off` leaves STT text unchanged. Network/local engines (Ollama,
    /// Groq) are wrapped so that any failure (server down, bad key, network)
    /// transparently falls back to the rule-based cleaner — the pipeline always
    /// produces usable output.
    pub fn create(
        kind: LlmEngineKind,
        ollama: OllamaConfig,
        groq: GroqLlmConfig,
        rule_based: RuleBasedConfig,
    ) -> Result<Arc<dyn LlmFormatter>> {
        match kind {
            LlmEngineKind::Off => Ok(Self::off()),
            LlmEngineKind::RuleBased => Ok(Self::rule_based(rule_based)),
            LlmEngineKind::Ollama => Ok(Arc::new(FallbackFormatter::new(
                Self::ollama(ollama)?,
                Self::rule_based(rule_based),
            ))),
            LlmEngineKind::Groq => Ok(Arc::new(FallbackFormatter::new(
                Self::groq(groq)?,
                Self::rule_based(rule_based),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::LlmMode;

    #[tokio::test]
    async fn off_engine_leaves_text_unchanged() {
        let formatter = LlmFactory::create(
            LlmEngineKind::Off,
            OllamaConfig::default(),
            GroqLlmConfig::default(),
            RuleBasedConfig::default(),
        )
        .unwrap();

        let out = formatter
            .format("um halo dunia", &LlmMode::Dictation, "id")
            .await
            .unwrap();

        assert_eq!(out, "um halo dunia");
        assert_eq!(formatter.name(), "off");
    }
}
