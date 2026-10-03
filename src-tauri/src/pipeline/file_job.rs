//! File transcription orchestration: chunked STT, then optional segmented
//! LLM formatting and dictionary replacements.
//!
//! Runs outside the recording state machine so it can proceed in parallel
//! with hotkey dictation. Pure over engine traits for testability.
//!
//! Transcript content is never discarded once produced: an STT failure after
//! some chunks keeps the finished part, and LLM failures or a user stop during
//! cleanup keep raw text for the affected segments. Each case is reported as a
//! [`StageIssue`] so the user knows the result is partial.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::mpsc;

use crate::error::{AppError, ErrorCode, Result};
use crate::llm::{LlmFormatter, LlmMode};
use crate::stt::{SttConfig, SttEngine};

/// Words per LLM call. Groq caps completions at 500 tokens and Indonesian
/// runs about 2 tokens per word, so 180 words keeps the formatted output
/// under the cap with headroom.
pub const LLM_SEGMENT_MAX_WORDS: usize = 180;
/// Prefer ending a segment at a sentence boundary once it holds this many words.
const LLM_SEGMENT_MIN_WORDS: usize = 120;
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(200);
const SENTENCE_TERMINATORS: [char; 3] = ['.', '?', '!'];
const HTTP_TOO_MANY_REQUESTS: u16 = 429;
/// Per-minute token quotas reset within a minute; the engine's own short
/// backoff has already been exhausted when a 429 reaches this layer.
const RATE_LIMIT_WAIT: Duration = Duration::from_secs(20);
const RATE_LIMIT_RETRIES: u32 = 3;
const STOPPED_BY_USER: &str = "Stopped by user";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "stage", rename_all = "snake_case")]
pub enum Progress {
    Transcribing { done: u32, total: u32 },
    Formatting { done: u32, total: u32 },
}

/// A stage that finished only partially. For STT, `count` is the number of
/// chunks transcribed before the failure; for LLM cleanup, `count` is the
/// number of segments left as raw text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StageIssue {
    pub count: u32,
    pub total: u32,
    pub reason: String,
}

pub struct RawTranscript {
    pub text: String,
    pub language: String,
    pub duration_ms: u64,
    pub stt_issue: Option<StageIssue>,
}

/// Optional post-processing chosen per job.
pub struct PostOptions {
    pub formatter: Option<Arc<dyn LlmFormatter>>,
    pub replacements: Option<Vec<(String, String)>>,
}

pub struct Processed {
    pub text: String,
    pub llm_issue: Option<StageIssue>,
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
/// (0 when the file reports no duration). Cancelling discards the job; an STT
/// failure after at least one chunk keeps what was transcribed.
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
    let mut stt_issue = None;
    on_progress(Progress::Transcribing { done, total });
    while let Some(chunk) = chunks.recv().await {
        // ponytail: cancelling mid-chunk drops the request; a running
        // whisper.cpp child still finishes (bounded by its own timeout).
        let outcome = tokio::select! {
            r = stt.transcribe(&chunk, config) => r,
            _ = until_cancelled(cancel) => return Err(cancelled()),
        };
        let result = match outcome {
            Ok(r) => r,
            Err(e) if done == 0 => return Err(e),
            Err(e) => {
                stt_issue = Some(StageIssue {
                    count: done,
                    total: total.max(done + 1),
                    reason: e.message,
                });
                break;
            }
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
        stt_issue,
    })
}

/// Apply the selected post-processing. Never fails: LLM problems degrade to
/// raw text for the affected segments and are reported in `llm_issue`.
pub async fn post_process(
    raw: &RawTranscript,
    options: &PostOptions,
    cancel: &AtomicBool,
    on_progress: impl Fn(Progress),
) -> Processed {
    let (formatted, llm_issue) = match &options.formatter {
        Some(formatter) => format_segments(raw, formatter.as_ref(), cancel, on_progress).await,
        None => (raw.text.clone(), None),
    };
    let text = match &options.replacements {
        Some(pairs) => crate::storage::apply_replacements(&formatted, pairs),
        None => formatted,
    };
    Processed { text, llm_issue }
}

/// Errors that will repeat for every remaining segment, so further calls
/// would only waste time.
fn stops_remaining_segments(e: &AppError) -> bool {
    matches!(
        e.code,
        ErrorCode::LlmConnectionRefused | ErrorCode::LlmApiKeyInvalid | ErrorCode::Cancelled
    )
}

async fn format_segments(
    raw: &RawTranscript,
    formatter: &dyn LlmFormatter,
    cancel: &AtomicBool,
    on_progress: impl Fn(Progress),
) -> (String, Option<StageIssue>) {
    let segments = split_into_segments(&raw.text, LLM_SEGMENT_MAX_WORDS);
    let total = segments.len() as u32;
    let mut out = Vec::with_capacity(segments.len());
    let mut skipped = 0;
    let mut last_reason = None;
    let mut stopped = false;
    on_progress(Progress::Formatting { done: 0, total });
    for (i, segment) in segments.into_iter().enumerate() {
        if !stopped && cancel.load(Ordering::Relaxed) {
            stopped = true;
            last_reason = Some(STOPPED_BY_USER.to_string());
        }
        if stopped {
            out.push(segment);
            skipped += 1;
            continue;
        }
        match format_with_rate_limit_retry(formatter, &segment, &raw.language, cancel).await {
            Ok(text) => out.push(text),
            Err(e) => {
                tracing::warn!("File transcription: segment {i} kept raw: {e}");
                stopped = stops_remaining_segments(&e);
                last_reason = Some(if e.code == ErrorCode::Cancelled {
                    STOPPED_BY_USER.to_string()
                } else {
                    e.message
                });
                out.push(segment);
                skipped += 1;
            }
        }
        on_progress(Progress::Formatting {
            done: i as u32 + 1,
            total,
        });
    }
    let issue = last_reason.map(|reason| StageIssue {
        count: skipped,
        total,
        reason,
    });
    (out.join("\n\n"), issue)
}

async fn format_with_rate_limit_retry(
    formatter: &dyn LlmFormatter,
    segment: &str,
    language: &str,
    cancel: &AtomicBool,
) -> Result<String> {
    let mut attempt = 0;
    loop {
        match formatter
            .format(segment, &LlmMode::Dictation, language)
            .await
        {
            Err(e)
                if e.http_status == Some(HTTP_TOO_MANY_REQUESTS)
                    && attempt < RATE_LIMIT_RETRIES =>
            {
                attempt += 1;
                tracing::info!("LLM rate limited; waiting {RATE_LIMIT_WAIT:?} (attempt {attempt})");
                tokio::select! {
                    _ = tokio::time::sleep(RATE_LIMIT_WAIT) => {}
                    _ = until_cancelled(cancel) => return Err(cancelled()),
                }
            }
            other => return other,
        }
    }
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
    use std::sync::atomic::AtomicU32;
    use std::sync::Mutex;

    /// Returns "chunk<len>"; fails on chunks whose length equals `fail_len`.
    struct EchoLenStt {
        fail_len: Option<usize>,
    }
    #[async_trait]
    impl SttEngine for EchoLenStt {
        async fn transcribe(&self, audio: &[f32], _c: &SttConfig) -> Result<TranscriptionResult> {
            if Some(audio.len()) == self.fail_len {
                return Err(AppError::stt_api("Groq STT error 503"));
            }
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

    /// Always refuses the connection and counts calls.
    struct DownFormatter(AtomicU32);
    #[async_trait]
    impl LlmFormatter for DownFormatter {
        async fn format(&self, _t: &str, _m: &LlmMode, _l: &str) -> Result<String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Err(AppError::llm_connection_refused(
                "Ollama connection refused",
            ))
        }
        async fn translate(&self, text: &str, _s: &str, _t: &str) -> Result<String> {
            Ok(text.to_string())
        }
        fn name(&self) -> &'static str {
            "down"
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
            stt_issue: None,
        }
    }

    fn options(formatter: Option<Arc<dyn LlmFormatter>>) -> PostOptions {
        PostOptions {
            formatter,
            replacements: None,
        }
    }

    /// Three segments: two plain, one containing "boom" in the middle.
    fn three_segments() -> String {
        let plain = "word ".repeat(LLM_SEGMENT_MAX_WORDS);
        format!("{plain}{} {plain}", "boom ".repeat(LLM_SEGMENT_MAX_WORDS))
    }

    #[tokio::test]
    async fn transcribes_chunks_in_order_and_reports_progress() {
        let progress = Mutex::new(Vec::new());
        let out = transcribe_chunks(
            channel_with(vec![vec![0.0; 3], vec![0.0; 5]]),
            Arc::new(EchoLenStt { fail_len: None }),
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
        assert!(out.stt_issue.is_none());
        assert_eq!(
            progress.lock().unwrap().last(),
            Some(&Progress::Transcribing { done: 2, total: 2 })
        );
    }

    #[tokio::test]
    async fn stt_failure_after_first_chunk_keeps_partial_transcript() {
        let out = transcribe_chunks(
            channel_with(vec![vec![0.0; 3], vec![0.0; 5], vec![0.0; 7]]),
            Arc::new(EchoLenStt { fail_len: Some(5) }),
            &SttConfig::default(),
            &AtomicBool::new(false),
            3,
            |_| {},
        )
        .await
        .unwrap();
        assert_eq!(out.text, "chunk3");
        let issue = out.stt_issue.unwrap();
        assert_eq!((issue.count, issue.total), (1, 3));
    }

    #[tokio::test]
    async fn stt_failure_on_first_chunk_is_an_error() {
        let result = transcribe_chunks(
            channel_with(vec![vec![0.0; 5]]),
            Arc::new(EchoLenStt { fail_len: Some(5) }),
            &SttConfig::default(),
            &AtomicBool::new(false),
            1,
            |_| {},
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cancel_aborts_in_flight_chunk() {
        let err = transcribe_chunks(
            channel_with(vec![vec![0.0; 3]]),
            Arc::new(HangingStt),
            &SttConfig::default(),
            &AtomicBool::new(true),
            1,
            |_| {},
        )
        .await
        .err()
        .unwrap();
        assert_eq!(err.code, ErrorCode::Cancelled);
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
        let out = post_process(&raw(&text), &options, &AtomicBool::new(false), |_| {}).await;
        let parts: Vec<&str> = out.text.split("\n\n").collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[0].starts_with("WORD WORD"));
        assert_eq!(parts[1], "hello Earth. boom.");
        let issue = out.llm_issue.unwrap();
        assert_eq!((issue.count, issue.total), (1, 2));
    }

    #[tokio::test]
    async fn unreachable_llm_stops_after_first_segment() {
        let down = Arc::new(DownFormatter(AtomicU32::new(0)));
        let out = post_process(
            &raw(&three_segments()),
            &options(Some(down.clone())),
            &AtomicBool::new(false),
            |_| {},
        )
        .await;
        assert_eq!(down.0.load(Ordering::SeqCst), 1);
        assert_eq!(out.text.split("\n\n").count(), 3);
        assert_eq!(out.llm_issue.unwrap().count, 3);
    }

    #[tokio::test]
    async fn stop_during_cleanup_keeps_full_raw_text() {
        let text = three_segments();
        let out = post_process(
            &raw(&text),
            &options(Some(Arc::new(UpperFormatter))),
            &AtomicBool::new(true),
            |_| {},
        )
        .await;
        assert_eq!(
            out.text.split_whitespace().count(),
            text.split_whitespace().count()
        );
        let issue = out.llm_issue.unwrap();
        assert_eq!(issue.count, 3);
        assert_eq!(issue.reason, STOPPED_BY_USER);
    }

    #[tokio::test]
    async fn no_options_returns_raw_text() {
        let out = post_process(
            &raw("um hi"),
            &options(None),
            &AtomicBool::new(false),
            |_| {},
        )
        .await;
        assert_eq!(out.text, "um hi");
        assert!(out.llm_issue.is_none());
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
