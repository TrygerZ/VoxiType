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

/// Optional translation step applied after formatting.
pub struct TranslateOpts {
    pub target: String,
}

/// Post-formatting text transforms applied before injection.
#[derive(Default)]
pub struct PostProcess {
    /// Dictionary `(word -> replacement)` pairs.
    pub replacements: Vec<(String, String)>,
    /// Snippet `(trigger_phrase -> content)` pairs.
    pub snippets: Vec<(String, String)>,
}

/// Run STT -> LLM format -> (optional) translate -> replacements -> snippets ->
/// injection.
///
/// Replacements and snippet expansion run after formatting/translation so
/// custom spellings and shortcuts always win over the LLM's output.
#[allow(clippy::too_many_arguments)]
pub async fn run_batch(
    audio: &[f32],
    stt: Arc<dyn SttEngine>,
    stt_config: &SttConfig,
    llm: Arc<dyn LlmFormatter>,
    mode: &LlmMode,
    post: &PostProcess,
    translate: Option<&TranslateOpts>,
    injector: &dyn TextInjector,
) -> Result<BatchOutcome> {
    let transcription = stt.transcribe(audio, stt_config).await?;
    run_batch_with_transcription(transcription, llm, mode, post, translate, injector).await
}

/// Like [`run_batch`] but skips STT, reusing an already-computed transcription.
///
/// Used by command-mode, which must transcribe up front to detect editing
/// commands; on a non-command phrase it falls through to normal injection
/// without paying for a second transcription.
pub async fn run_batch_with_transcription(
    transcription: TranscriptionResult,
    llm: Arc<dyn LlmFormatter>,
    mode: &LlmMode,
    post: &PostProcess,
    translate: Option<&TranslateOpts>,
    injector: &dyn TextInjector,
) -> Result<BatchOutcome> {
    let formatted_text = if transcription.text.trim().is_empty() {
        String::new()
    } else {
        let formatted = llm
            .format(&transcription.text, mode, &transcription.language)
            .await?;
        let translated = match translate {
            Some(opts)
                if opts.target != transcription.language
                    && !transcription.language.is_empty()
                    && transcription.language != "auto"
                    && transcription.language != "unknown" =>
            {
                // Guard against translation when the formatted text clearly
                // matches the target language already (e.g. STT misdetected
                // the language but the text is already in the target). This
                // prevents a "leaking" translation that converts e.g. English
                // to Indonesian when the user did not intend it.
                let text_lang = detect_text_language(&formatted);
                if text_lang == opts.target {
                    tracing::info!(
                        "Skipping translation: text already in target '{}'",
                        opts.target
                    );
                    formatted
                } else {
                    llm.translate(&formatted, &transcription.language, &opts.target)
                        .await?
                }
            }
            _ => formatted,
        };
        let replaced = crate::storage::apply_replacements(&translated, &post.replacements);
        crate::storage::expand_snippets(&replaced, &post.snippets)
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

/// Detect the dominant language of a text by counting language-specific
/// function-word markers. Returns "en", "id", or "unknown".
fn detect_text_language(text: &str) -> String {
    let lower = format!(" {} ", text.to_lowercase());
    let en_markers = [
        " the ", " and ", " is ", " to ", " of ", " a ", " in ", " for ", " it ", " with ",
        " you ", " that ", " have ", " this ", " from ", " are ", " was ", " will ",
    ];
    let id_markers = [
        " yang ", " dan ", " di ", " ini ", " itu ", " dari ", " dengan ", " untuk ", " tidak ",
        " adalah ", " juga ", " karena ", " pada ", " saya ", " mereka ", " bisa ", " akan ",
        " sudah ",
    ];
    let en_count = en_markers.iter().filter(|&&m| lower.contains(m)).count();
    let id_count = id_markers.iter().filter(|&&m| lower.contains(m)).count();
    if en_count > id_count && en_count > 0 {
        "en".to_string()
    } else if id_count > en_count && id_count > 0 {
        "id".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::injection::{InjectResult, InjectStrategy};
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

    #[test]
    fn detect_text_language_identifies_english() {
        assert_eq!(detect_text_language("this is a test of the system"), "en");
    }

    #[test]
    fn detect_text_language_identifies_indonesian() {
        assert_eq!(
            detect_text_language("saya pergi ke pasar dengan mereka"),
            "id"
        );
    }

    #[test]
    fn detect_text_language_returns_unknown_for_ambiguous() {
        assert_eq!(detect_text_language("hello world"), "unknown");
    }

    #[tokio::test]
    async fn translation_skipped_when_source_language_is_unknown() {
        use crate::llm::{LlmFactory, RuleBasedConfig};
        let llm = LlmFactory::rule_based(RuleBasedConfig::default());
        let injector = MockInjector;
        let transcription = TranscriptionResult::text_only("hello world", "unknown");
        let out = run_batch_with_transcription(
            transcription,
            llm,
            &LlmMode::Dictation,
            &PostProcess::default(),
            Some(&TranslateOpts {
                target: "id".into(),
            }),
            &injector,
        )
        .await
        .unwrap();
        // Rule-based formatter capitalizes and adds period, no translation.
        assert_eq!(out.formatted_text, "Hello world.");
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
            &PostProcess::default(),
            None,
            &injector,
        )
        .await
        .unwrap();
        // "um" filler removed, capitalized, period added.
        assert_eq!(out.formatted_text, "Halo dunia.");
        assert!(out.inject.success);
    }
}
