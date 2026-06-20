//! LLM formatter factory.

use std::sync::Arc;

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

    /// Build a formatter for the selected engine. `Off` maps to the rule-based
    /// cleaner so the pipeline always produces usable output.
    pub fn create(
        kind: LlmEngineKind,
        ollama: OllamaConfig,
        groq: GroqLlmConfig,
        rule_based: RuleBasedConfig,
    ) -> Arc<dyn LlmFormatter> {
        match kind {
            LlmEngineKind::Off | LlmEngineKind::RuleBased => Self::rule_based(rule_based),
            LlmEngineKind::Ollama => Self::ollama(ollama),
            LlmEngineKind::Groq => Self::groq(groq),
        }
    }
}
