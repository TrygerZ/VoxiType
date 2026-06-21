//! Formatter that wraps a primary backend and falls back to the rule-based
//! cleaner when the primary fails (e.g. Ollama not running, network down).
//!
//! This guarantees the pipeline always produces usable output instead of
//! erroring out when a cloud/local LLM is unavailable.

use std::sync::Arc;

use async_trait::async_trait;

use super::types::LlmMode;
use super::LlmFormatter;
use crate::error::Result;

pub struct FallbackFormatter {
    primary: Arc<dyn LlmFormatter>,
    fallback: Arc<dyn LlmFormatter>,
}

impl FallbackFormatter {
    pub fn new(primary: Arc<dyn LlmFormatter>, fallback: Arc<dyn LlmFormatter>) -> Self {
        Self { primary, fallback }
    }
}

#[async_trait]
impl LlmFormatter for FallbackFormatter {
    async fn format(&self, text: &str, mode: &LlmMode) -> Result<String> {
        match self.primary.format(text, mode).await {
            Ok(out) => Ok(out),
            Err(e) => {
                tracing::warn!(
                    "LLM '{}' failed ({e}); falling back to '{}'",
                    self.primary.name(),
                    self.fallback.name()
                );
                self.fallback.format(text, mode).await
            }
        }
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        match self.primary.translate(text, source, target).await {
            Ok(out) => Ok(out),
            Err(e) => {
                tracing::warn!(
                    "LLM '{}' translate failed ({e}); falling back to '{}'",
                    self.primary.name(),
                    self.fallback.name()
                );
                self.fallback.translate(text, source, target).await
            }
        }
    }

    fn name(&self) -> &'static str {
        self.primary.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    struct AlwaysFails;
    #[async_trait]
    impl LlmFormatter for AlwaysFails {
        async fn format(&self, _t: &str, _m: &LlmMode) -> Result<String> {
            Err(AppError::llm_connection_refused("down"))
        }
        async fn translate(&self, _t: &str, _s: &str, _d: &str) -> Result<String> {
            Err(AppError::llm_connection_refused("down"))
        }
        fn name(&self) -> &'static str {
            "always_fails"
        }
    }

    #[tokio::test]
    async fn falls_back_on_primary_error() {
        use super::super::{LlmFactory, RuleBasedConfig};
        let primary: Arc<dyn LlmFormatter> = Arc::new(AlwaysFails);
        let fallback = LlmFactory::rule_based(RuleBasedConfig::default());
        let f = FallbackFormatter::new(primary, fallback);
        let out = f
            .format("um halo dunia", &LlmMode::Dictation)
            .await
            .unwrap();
        assert_eq!(out, "Halo dunia.");
    }
}
