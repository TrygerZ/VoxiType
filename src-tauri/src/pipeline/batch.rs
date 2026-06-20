//! Batch pipeline: transcribe -> format -> inject.
//!
//! Pure orchestration over the engine traits; no direct device or DB access so
//! it stays easy to test with mock engines.

use std::sync::Arc;

use crate::error::Result;
use crate::injection::{InjectResult, TextInjector};
use crate::llm::{LlmFormatter, LlmMode};
use crate::stt::{SttConfig, SttEngine, TranscriptionResult};

/// Outcome of a full batch run.
pub struct BatchOutcome {
    pub transcription: TranscriptionResult,
    pub formatted_text: String,
    pub inject: InjectResult,
}

/// Run STT -> LLM -> injection for one captured utterance.
pub async fn run_batch(
    audio: &[f32],
    stt: Arc<dyn SttEngine>,
    stt_config: &SttConfig,
    llm: Arc<dyn LlmFormatter>,
    mode: &LlmMode,
    injector: &dyn TextInjector,
) -> Result<BatchOutcome> {
    let transcription = stt.transcribe(audio, stt_config).await?;

    let formatted_text = if transcription.text.trim().is_empty() {
        String::new()
    } else {
        llm.format(&transcription.text, mode).await?
    };

    let inject = if formatted_text.is_empty() {
        InjectResult {
            success: true,
            strategy: crate::injection::InjectStrategy::Manual,
            chars_injected: 0,
            duration_ms: 0,
        }
    } else {
        injector.inject(&formatted_text)?
    };

    Ok(BatchOutcome {
        transcription,
        formatted_text,
        inject,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::injection::{InjectStrategy, InjectResult};
    use async_trait::async_trait;

    struct MockStt;
    #[async_trait]
    impl SttEngine for MockStt {
        async fn transcribe(
            &self,
            _audio: &[f32],
            _config: &SttConfig,
        ) -> Result<TranscriptionResult> {
            Ok(TranscriptionResult::text_only("um halo dunia", "id"))
        }
        fn name(&self) -> &'static str {
            "mock"
        }
    }

    struct MockInjector;
    impl TextInjector for MockInjector {
        fn inject(&self, text: &str) -> Result<InjectResult> {
            Ok(InjectResult {
                success: true,
                strategy: InjectStrategy::Clipboard,
                chars_injected: text.chars().count() as u32,
                duration_ms: 1,
            })
        }
        fn inject_keystroke(&self, text: &str) -> Result<InjectResult> {
            self.inject(text)
        }
        fn inject_clipboard(&self, text: &str) -> Result<InjectResult> {
            self.inject(text)
        }
    }

    #[tokio::test]
    async fn batch_runs_end_to_end() {
        use crate::llm::{LlmFactory, RuleBasedConfig};
        let stt: Arc<dyn SttEngine> = Arc::new(MockStt);
        let llm = LlmFactory::rule_based(RuleBasedConfig::default());
        let injector = MockInjector;
        let out = run_batch(
            &[0.0; 16],
            stt,
            &SttConfig::default(),
            llm,
            &LlmMode::Dictation,
            &injector,
        )
        .await
        .unwrap();
        // "um" filler removed, capitalized, period added.
        assert_eq!(out.formatted_text, "Halo dunia.");
        assert!(out.inject.success);
    }
}
