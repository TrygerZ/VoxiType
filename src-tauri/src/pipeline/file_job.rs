//! File transcription orchestration: chunked STT, then optional segmented
//! LLM formatting and dictionary replacements.
//!
//! Runs outside the recording state machine so it can proceed in parallel
//! with hotkey dictation. Pure over engine traits for testability.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::error::{AppError, Result};
use crate::llm::{LlmFormatter, LlmMode};
use crate::stt::{SttConfig, SttEngine};

/// Words per LLM call. Groq caps completions at 500 tokens, so a segment's
/// formatted output must stay comfortably below that.
pub const LLM_SEGMENT_MAX_WORDS: usize = 250;
/// Prefer ending a segment at a sentence boundary once it holds this many words.
const LLM_SEGMENT_MIN_WORDS: usize = 150;
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SENTENCE_TERMINATORS: [char; 3] = ['.', '?', '!'];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Progress {
    Transcribing { done: u32, total: u32 },
    Formatting { done: u32, total: u32 },
}

pub struct RawTranscript {
    pub text: String,
    pub language: String,
    pub duration_ms: u64,
}

/// Optional post-processing chosen per job.
pub struct PostOptions {
    pub formatter: Option<Arc<dyn LlmFormatter>>,
    pub replacements: Option<Vec<(String, String)>>,
}

fn cancelled() -> AppError {
    AppError::cancelled("File transcription cancelled")
}

async fn until_cancelled(cancel: &AtomicBool) {
    while !cancel.load(Ordering::Relaxed) {
        tokio::time::sleep(CANCEL_POLL_INTERVAL).await;
    }
}

fn is_resolved_language(language: &str) -> bool {
    !matches!(language, "" | "auto" | "unknown")
}

/// Transcribe chunks as the decoder produces them. `total` is an estimate
/// (0 when the file reports no duration).
pub async fn transcribe_chunks(
    mut chunks: mpsc::Receiver<Vec<f32>>,
    stt: Arc<dyn SttEngine>,
    config: &SttConfig,
    cancel: &AtomicBool,
    total: u32,
    on_progress: impl Fn(Progress),
) -> Result<RawTranscript> {
    let mut parts = Vec::new();
    let mut language = config.language.clone();
    let mut duration_ms = 0;
    let mut done = 0;
    on_progress(Progress::Transcribing { done, total });
    while let Some(chunk) = chunks.recv().await {
        // ponytail: cancelling mid-chunk drops the request; a running
        // whisper.cpp child still finishes (bounded by its own timeout).
        let result = tokio::select! {
            r = stt.transcribe(&chunk, config) => r?,
            _ = until_cancelled(cancel) => return Err(cancelled()),
        };
        if !is_resolved_language(&language) && is_resolved_language(&result.language) {
            language = result.language.clone();
        }
        duration_ms += result.duration_ms;
        let text = result.text.trim();
        if !text.is_empty() {
            parts.push(text.to_string());
        }
        done += 1;
        on_progress(Progress::Transcribing {
            done,
            total: total.max(done),
        });
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(cancelled());
    }
    Ok(RawTranscript {
        text: parts.join(" "),
        language,
        duration_ms,
    })
}

/// Apply the selected post-processing. A segment whose formatting fails
/// keeps its raw text so a flaky LLM never loses transcript content.
pub async fn post_process(
    raw: &RawTranscript,
    options: &PostOptions,
    cancel: &AtomicBool,
    on_progress: impl Fn(Progress),
) -> Result<String> {
    let formatted = match &options.formatter {
        Some(formatter) => format_segments(raw, formatter.as_ref(), cancel, on_progress).await?,
        None => raw.text.clone(),
    };
    Ok(match &options.replacements {
        Some(pairs) => crate::storage::apply_replacements(&formatted, pairs),
        None => formatted,
    })
}

async fn format_segments(
    raw: &RawTranscript,
    formatter: &dyn LlmFormatter,
    cancel: &AtomicBool,
    on_progress: impl Fn(Progress),
) -> Result<String> {
    let segments = split_into_segments(&raw.text, LLM_SEGMENT_MAX_WORDS);
    let total = segments.len() as u32;
    let mut out = Vec::with_capacity(segments.len());
    on_progress(Progress::Formatting { done: 0, total });
    for (i, segment) in segments.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(cancelled());
        }
        match formatter
            .format(&segment, &LlmMode::Dictation, &raw.language)
            .await
        {
            Ok(text) => out.push(text),
            Err(e) => {
                tracing::warn!(
                    "File transcription: formatting segment {i} failed, keeping raw: {e}"
                );
                out.push(segment);
            }
        }
        on_progress(Progress::Formatting {
            done: i as u32 + 1,
            total,
        });
    }
    Ok(out.join("\n\n"))
}

/// Split text into segments of at most `max_words`, preferring to end each
/// at a sentence boundary once it reaches [`LLM_SEGMENT_MIN_WORDS`].
pub fn split_into_segments(text: &str, max_words: usize) -> Vec<String> {
    let min_words = LLM_SEGMENT_MIN_WORDS.min(max_words);
    let mut segments = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in text.split_whitespace() {
        current.push(word);
        let at_sentence_end = word.ends_with(SENTENCE_TERMINATORS);
        if current.len() >= max_words || (at_sentence_end && current.len() >= min_words) {
            segments.push(current.join(" "));
            current.clear();
        }
    }
    if !current.is_empty() {
        segments.push(current.join(" "));
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt::TranscriptionResult;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct EchoLenStt;
    #[async_trait]
    impl SttEngine for EchoLenStt {
        async fn transcribe(&self, audio: &[f32], _c: &SttConfig) -> Result<TranscriptionResult> {
            let mut r = TranscriptionResult::text_only(format!("chunk{}", audio.len()), "id");
            r.duration_ms = audio.len() as u64;
            Ok(r)
        }
        fn name(&self) -> &'static str {
            "echo"
        }
    }

    struct HangingStt;
    #[async_trait]
    impl SttEngine for HangingStt {
        async fn transcribe(&self, _a: &[f32], _c: &SttConfig) -> Result<TranscriptionResult> {
            std::future::pending().await
        }
        fn name(&self) -> &'static str {
            "hang"
        }
    }

    /// Upper-cases text; fails on segments containing "boom".
    struct UpperFormatter;
    #[async_trait]
    impl LlmFormatter for UpperFormatter {
        async fn format(&self, text: &str, _m: &LlmMode, _l: &str) -> Result<String> {
            if text.contains("boom") {
                return Err(AppError::llm("boom"));
            }
            Ok(text.to_uppercase())
        }
        async fn translate(&self, text: &str, _s: &str, _t: &str) -> Result<String> {
            Ok(text.to_string())
        }
        fn name(&self) -> &'static str {
            "upper"
        }
    }

    fn channel_with(chunks: Vec<Vec<f32>>) -> mpsc::Receiver<Vec<f32>> {
        let (tx, rx) = mpsc::channel(chunks.len().max(1));
        for c in chunks {
            tx.try_send(c).unwrap();
        }
        rx
    }

    fn raw(text: &str) -> RawTranscript {
        RawTranscript {
            text: text.to_string(),
            language: "en".to_string(),
            duration_ms: 0,
        }
    }

    #[tokio::test]
    async fn transcribes_chunks_in_order_and_reports_progress() {
        let progress = Mutex::new(Vec::new());
        let out = transcribe_chunks(
            channel_with(vec![vec![0.0; 3], vec![0.0; 5]]),
            Arc::new(EchoLenStt),
            &SttConfig::default(),
            &AtomicBool::new(false),
            2,
            |p| progress.lock().unwrap().push(p),
        )
        .await
        .unwrap();
        assert_eq!(out.text, "chunk3 chunk5");
        assert_eq!(out.language, "id");
        assert_eq!(out.duration_ms, 8);
        assert_eq!(
            progress.lock().unwrap().last(),
            Some(&Progress::Transcribing { done: 2, total: 2 })
        );
    }

    #[tokio::test]
    async fn cancel_aborts_in_flight_chunk() {
        let cancel = AtomicBool::new(true);
        let err = transcribe_chunks(
            channel_with(vec![vec![0.0; 3]]),
            Arc::new(HangingStt),
            &SttConfig::default(),
            &cancel,
            1,
            |_| {},
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err.code, crate::error::ErrorCode::Cancelled);
    }

    #[tokio::test]
    async fn failed_segment_keeps_raw_text_and_replacements_apply() {
        let options = PostOptions {
            formatter: Some(Arc::new(UpperFormatter)),
            replacements: Some(vec![("WORLD".to_string(), "Earth".to_string())]),
        };
        let text = format!(
            "{}hello world. boom.",
            "word ".repeat(LLM_SEGMENT_MAX_WORDS)
        );
        let out = post_process(&raw(&text), &options, &AtomicBool::new(false), |_| {})
            .await
            .unwrap();
        let parts: Vec<&str> = out.split("\n\n").collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].starts_with("WORD WORD"));
        assert_eq!(parts[1], "hello Earth. boom.");
    }

    #[tokio::test]
    async fn no_options_returns_raw_text() {
        let options = PostOptions {
            formatter: None,
            replacements: None,
        };
        let out = post_process(&raw("um hi"), &options, &AtomicBool::new(false), |_| {})
            .await
            .unwrap();
        assert_eq!(out, "um hi");
    }

    #[test]
    fn segments_prefer_sentence_boundaries_and_respect_max() {
        let sentence = format!("{}end.", "w ".repeat(LLM_SEGMENT_MIN_WORDS));
        let text = format!("{sentence} {}", "x ".repeat(LLM_SEGMENT_MAX_WORDS + 10));
        let segments = split_into_segments(&text, LLM_SEGMENT_MAX_WORDS);
        assert!(segments[0].ends_with("end."));
        assert!(segments
            .iter()
            .all(|s| s.split_whitespace().count() <= LLM_SEGMENT_MAX_WORDS));
        let words: usize = segments.iter().map(|s| s.split_whitespace().count()).sum();
        assert_eq!(words, text.split_whitespace().count());
    }
}
