//! Batch pipeline: transcribe -> format -> inject.
//!
//! Pure orchestration over the engine traits; no direct device or DB access so
//! it stays easy to test with mock engines.

use std::sync::Arc;

use crate::error::{AppError, Result};
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
    injector: Arc<dyn TextInjector>,
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
    injector: Arc<dyn TextInjector>,
) -> Result<BatchOutcome> {
    let formatted_text = if transcription.text.trim().is_empty() {
        String::new()
    } else {
        let formatted = llm
            .format(&transcription.text, mode, &transcription.language)
            .await?;
        let translated = match translate {
            Some(opts) if opts.target != transcription.language => {
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
                    if is_unresolved_language(&transcription.language) {
                        tracing::info!(
                            "STT returned unresolved language '{}'; \
                             translating with automatic source detection",
                            transcription.language
                        );
                    }
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
        // Injection blocks the OS event loop for hundreds of milliseconds
        // (clipboard propagation sleeps + enigo keystroke simulation), so it
        // must never run directly on a tokio worker thread.
        let text_to_inject = formatted_text.clone();
        tokio::task::spawn_blocking(move || injector.inject(&text_to_inject))
            .await
            .map_err(|e| AppError::injection(format!("Injection task failed: {e}")))??
    };

    Ok(BatchOutcome {
        transcription,
        formatted_text,
        inject,
    })
}

/// Languages that mean STT could not determine the spoken language.
/// whisper.cpp maps "auto" to "unknown"; empty means the engine reported none.
fn is_unresolved_language(language: &str) -> bool {
    language.is_empty() || language == "auto" || language == "unknown"
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

    /// Formatter that records every `translate` call so tests can assert
    /// whether translation actually ran, and tags translated output with
    /// the requested source/target pair.
    struct RecordingLlm {
        translate_calls: std::sync::Mutex<Vec<(String, String)>>,
    }

    impl RecordingLlm {
        fn new() -> Self {
            Self {
                translate_calls: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn translate_call_count(&self) -> usize {
            self.translate_calls
                .lock()
                .map(|calls| calls.len())
                .unwrap_or(0)
        }
    }

    #[async_trait]
    impl LlmFormatter for RecordingLlm {
        async fn format(&self, text: &str, _mode: &LlmMode, _language: &str) -> Result<String> {
            Ok(text.to_string())
        }

        async fn translate(&self, text: &str, source: &str, target: &str) -> Result<String> {
            if let Ok(mut calls) = self.translate_calls.lock() {
                calls.push((source.to_string(), target.to_string()));
            }
            Ok(format!("[{source}->{target}] {text}"))
        }

        fn name(&self) -> &'static str {
            "recording"
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
    async fn translation_runs_when_source_language_is_unknown() {
        let llm = Arc::new(RecordingLlm::new());
        let transcription =
            TranscriptionResult::text_only("this is a test of the system", "unknown");
        let out = run_batch_with_transcription(
            transcription,
            llm.clone(),
            &LlmMode::Dictation,
            &PostProcess::default(),
            Some(&TranslateOpts {
                target: "id".into(),
            }),
            Arc::new(MockInjector),
        )
        .await
        .unwrap();
        // Unresolved source language must still reach the translator.
        assert_eq!(llm.translate_call_count(), 1);
        assert_eq!(
            out.formatted_text,
            "[unknown->id] this is a test of the system"
        );
    }

    #[tokio::test]
    async fn translation_skipped_when_text_already_in_target_language() {
        let llm = Arc::new(RecordingLlm::new());
        // Text markers clearly Indonesian while STT language is unresolved:
        // the anti-leak guard must keep skipping translation.
        let transcription =
            TranscriptionResult::text_only("saya pergi ke pasar dengan mereka", "unknown");
        let out = run_batch_with_transcription(
            transcription,
            llm.clone(),
            &LlmMode::Dictation,
            &PostProcess::default(),
            Some(&TranslateOpts {
                target: "id".into(),
            }),
            Arc::new(MockInjector),
        )
        .await
        .unwrap();
        assert_eq!(llm.translate_call_count(), 0);
        assert_eq!(out.formatted_text, "saya pergi ke pasar dengan mereka");
    }

    #[tokio::test]
    async fn translation_uses_detected_source_for_known_language() {
        let llm = Arc::new(RecordingLlm::new());
        let transcription = TranscriptionResult::text_only("this is a test of the system", "en");
        let out = run_batch_with_transcription(
            transcription,
            llm.clone(),
            &LlmMode::Dictation,
            &PostProcess::default(),
            Some(&TranslateOpts {
                target: "id".into(),
            }),
            Arc::new(MockInjector),
        )
        .await
        .unwrap();
        // Existing behavior for a resolved source language is unchanged.
        assert_eq!(llm.translate_call_count(), 1);
        assert_eq!(out.formatted_text, "[en->id] this is a test of the system");
    }

    #[tokio::test]
    async fn batch_runs_end_to_end() {
        use crate::llm::{LlmFactory, RuleBasedConfig};
        let stt: Arc<dyn SttEngine> = Arc::new(MockStt);
        let llm = LlmFactory::rule_based(RuleBasedConfig::default());
        let out = run_batch(
            &[0.0; 16],
            stt,
            &SttConfig::default(),
            llm,
            &LlmMode::Dictation,
            &PostProcess::default(),
            None,
            Arc::new(MockInjector),
        )
        .await
        .unwrap();
        // "um" filler removed, capitalized, period added.
        assert_eq!(out.formatted_text, "Halo dunia.");
        assert!(out.inject.success);
    }
}
