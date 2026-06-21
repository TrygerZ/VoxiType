//! LLM formatter factory.

use std::sync::Arc;

use super::fallback::FallbackFormatter;
use super::groq_llm::GroqLlmFormatter;
use super::ollama::OllamaFormatter;
use super::rule_based::RuleBasedFormatter;
use super::types::{GroqLlmConfig, LlmEngineKind, OllamaConfig, RuleBasedConfig};
use super::LlmFormatter;

pub struct LlmFactory;

impl LlmFactory {
    pub fn rule_based(config: RuleBasedConfig) -> Arc<dyn LlmFormatter> {
        Arc::new(RuleBasedFormatter::new(config))
    }

    pub fn ollama(config: OllamaConfig) -> Arc<dyn LlmFormatter> {
        Arc::new(OllamaFormatter::new(config))
    }

    pub fn groq(config: GroqLlmConfig) -> Arc<dyn LlmFormatter> {
        Arc::new(GroqLlmFormatter::new(config))
    }

    /// Build a formatter for the selected engine.
    ///
    /// `Off` maps to the rule-based cleaner. Network/local engines (Ollama,
    /// Groq) are wrapped so that any failure (server down, bad key, network)
    /// transparently falls back to the rule-based cleaner — the pipeline always
    /// produces usable output.
    pub fn create(
        kind: LlmEngineKind,
        ollama: OllamaConfig,
        groq: GroqLlmConfig,
        rule_based: RuleBasedConfig,
    ) -> Arc<dyn LlmFormatter> {
        match kind {
            LlmEngineKind::Off | LlmEngineKind::RuleBased => Self::rule_based(rule_based),
            LlmEngineKind::Ollama => Arc::new(FallbackFormatter::new(
                Self::ollama(ollama),
                Self::rule_based(rule_based),
            )),
            LlmEngineKind::Groq => Arc::new(FallbackFormatter::new(
                Self::groq(groq),
                Self::rule_based(rule_based),
            )),
        }
    }
}
