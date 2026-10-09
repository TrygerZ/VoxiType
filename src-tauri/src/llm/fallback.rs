//! Formatter that wraps a primary backend and falls back to the rule-based
//! cleaner when the primary fails (e.g. Ollama not running, network down).
//!
//! This guarantees the pipeline always produces usable output instead of
//! erroring out when a cloud/local LLM is unavailable.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use super::types::LlmMode;
use super::LlmFormatter;
use crate::error::Result;

pub struct FallbackFormatter {
    primary: Arc<dyn LlmFormatter>,
    fallback: Arc<dyn LlmFormatter>,
    /// Whether the most recent completed call was produced by the fallback.
    used_fallback: AtomicBool,
}

impl FallbackFormatter {
    pub fn new(primary: Arc<dyn LlmFormatter>, fallback: Arc<dyn LlmFormatter>) -> Self {
        Self {
            primary,
            fallback,
            used_fallback: AtomicBool::new(false),
        }
    }

    fn record(&self, used_fallback: bool) {
        self.used_fallback.store(used_fallback, Ordering::Relaxed);
    }
}

#[async_trait]
impl LlmFormatter for FallbackFormatter {
    async fn format(&self, text: &str, mode: &LlmMode, language: &str) -> Result<String> {
        match self.primary.format(text, mode, language).await {
            Ok(out) => {
                self.record(false);
                Ok(out)
            }
            Err(e) => {
                tracing::warn!(
                    "LLM '{}' failed ({e}); falling back to '{}'",
                    self.primary.name(),
                    self.fallback.name()
                );
                let out = self.fallback.format(text, mode, language).await;
                self.record(out.is_ok());
                out
            }
        }
    }

    async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
        match self.primary.translate(text, source, target).await {
            Ok(out) => {
                self.record(false);
                Ok(out)
            }
            Err(e) => {
                tracing::warn!(
                    "LLM '{}' translate failed ({e}); falling back to '{}'",
                    self.primary.name(),
                    self.fallback.name()
                );
                let out = self.fallback.translate(text, source, target).await;
                self.record(out.is_ok());
                out
            }
        }
    }

    /// Name of the engine that produced the last completed `format`/`translate`
    /// output: the primary when it succeeded, the fallback when it took over.
    fn name(&self) -> &'static str {
        if self.used_fallback.load(Ordering::Relaxed) {
            self.fallback.name()
        } else {
            self.primary.name()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    struct AlwaysFails;
    #[async_trait]
    impl LlmFormatter for AlwaysFails {
        async fn format(&self, _t: &str, _m: &LlmMode, _l: &str) -> Result<String> {
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
            .format("um halo dunia", &LlmMode::Dictation, "id")
            .await
            .unwrap();
        assert_eq!(out, "Halo dunia.");
    }

    /// A primary that fails on its first call then succeeds, so the recorded
    /// winning engine can be observed flipping.
    struct FailsOnce {
        calls: std::sync::atomic::AtomicU32,
    }

    impl FailsOnce {
        fn new() -> Self {
            Self {
                calls: std::sync::atomic::AtomicU32::new(0),
            }
        }
    }

    #[async_trait]
    impl LlmFormatter for FailsOnce {
        async fn format(&self, text: &str, _m: &LlmMode, _l: &str) -> Result<String> {
            use std::sync::atomic::Ordering;
            if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
                Err(AppError::llm_connection_refused("down"))
            } else {
                Ok(text.to_string())
            }
        }
        async fn translate(&self, text: &str, _s: &str, _d: &str) -> Result<String> {
            Ok(text.to_string())
        }
        fn name(&self) -> &'static str {
            "flaky"
        }
    }

    #[tokio::test]
    async fn name_tracks_the_winning_engine() {
        use super::super::{LlmFactory, RuleBasedConfig};
        let primary: Arc<dyn LlmFormatter> = Arc::new(FailsOnce::new());
        let fallback = LlmFactory::rule_based(RuleBasedConfig::default());
        let f = FallbackFormatter::new(primary, fallback);

        // First call: primary fails, fallback wins.
        f.format("um halo dunia", &LlmMode::Dictation, "id")
            .await
            .unwrap();
        assert_eq!(f.name(), "rule_based");

        // Second call: primary succeeds, name flips back.
        f.format("halo dunia", &LlmMode::Dictation, "id")
            .await
            .unwrap();
        assert_eq!(f.name(), "flaky");
    }
}
