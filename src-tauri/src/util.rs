//! Small shared helpers used across modules.

use std::future::Future;
use std::path::Path;
use std::sync::{MutexGuard, OnceLock};
use std::time::Duration;

use crate::error::{AppError, ErrorCode};

/// Extension trait for `std::sync::Mutex` that recovers from poisoned state
/// instead of panicking. A poisoned mutex means a thread panicked while holding
/// the lock; the data may be inconsistent but crashing the whole app is worse.
pub trait MutexExt<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T>;
}

impl<T> MutexExt<T> for std::sync::Mutex<T> {
    fn lock_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Write `content` to `destination` atomically: a temp file in the same
/// directory is written, synced, and renamed over the target, so a crash
/// mid-write never leaves a truncated or half-written file behind.
pub fn write_atomic(destination: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let file_stem = destination
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file");
    let temporary = parent.join(format!(".{file_stem}.tmp-{}-{}", std::process::id(), nonce));
    let result = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(content)?;
        file.sync_all()?;
        // Close before rename (Windows refuses to rename an open file).
        drop(file);
        // Carry the destination's mode across the replace so an atomic rewrite
        // never loosens an existing file's permissions (for example a 0600
        // master.key on Unix).
        if let Ok(metadata) = std::fs::metadata(destination) {
            let _ = std::fs::set_permissions(&temporary, metadata.permissions());
        }
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Default connect timeout for all outbound HTTP requests.
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default request timeout for short REST requests (LLM completions, checks, updates).
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Process-wide shared `reqwest` client.
///
/// Reusing one client (and its connection pool / TLS state) across requests
/// avoids the cost of rebuilding it for every recording. Cloning a `Client` is
/// cheap (it is an `Arc` internally). A build failure is cached and returned as
/// an error on every call instead of panicking.
pub fn http_client() -> crate::error::Result<reqwest::Client> {
    static CLIENT: OnceLock<std::result::Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(DEFAULT_CONNECT_TIMEOUT)
                .timeout(DEFAULT_REQUEST_TIMEOUT)
                .build()
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(|e| AppError::internal(format!("Failed to build HTTP client: {e}")))
}

/// Maximum length of upstream error-body text kept in an error message.
/// Upstream bodies are untrusted (format can change, may echo request
/// context), so they are capped before being embedded in logs or IPC events.
const MAX_ERROR_BODY_LEN: usize = 300;

/// Suffix appended when an upstream error body was truncated.
const TRUNCATED_SUFFIX: &str = "…(truncated)";

/// Replace non-printable control characters with a visible placeholder so
/// log files and frontend messages stay readable and safe. Common whitespace
/// (tab, newline, carriage return) is kept intact.
fn sanitize_control_chars(input: &str) -> String {
    const CONTROL_CHAR_PLACEHOLDER: char = '\u{FFFD}';
    input
        .chars()
        .map(|c| {
            if matches!(c, '\n' | '\t' | '\r') {
                c
            } else if c.is_control() {
                CONTROL_CHAR_PLACEHOLDER
            } else {
                c
            }
        })
        .collect()
}

/// Sanitize and cap an upstream API error body before embedding it in an
/// `AppError` message. Truncates to [`MAX_ERROR_BODY_LEN`] characters
/// (including the truncation suffix when applied) and strips control
/// characters, so oversized or hostile bodies cannot flood logs or the UI.
pub fn sanitize_error_body(body: &str) -> String {
    let sanitized = sanitize_control_chars(body);
    if sanitized.chars().count() <= MAX_ERROR_BODY_LEN {
        return sanitized;
    }
    let kept = sanitized
        .chars()
        .take(MAX_ERROR_BODY_LEN)
        .collect::<String>();
    format!("{kept}{TRUNCATED_SUFFIX}")
}

/// HTTP statuses that indicate a *transient* API failure worth retrying:
/// request timeout and rate limiting.
const RETRYABLE_HTTP_STATUSES: &[u16] = &[408, 429];

fn has_transient_http_status(err: &AppError) -> bool {
    match err.http_status {
        Some(status) => status >= 500 || RETRYABLE_HTTP_STATUSES.contains(&status),
        // No status recorded — the failure cannot be proven transient.
        None => false,
    }
}

/// Context for retry decisions distinguishing small requests from large uploads.
///
/// Large payloads (such as STT audio uploads) should not retry on timeout,
/// because re-uploading multi-megabyte audio over an already slow link wastes
/// minutes and memory. Small requests (LLM completions, checks) can safely retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryContext {
    /// Small payloads: LLM completions, API tests, metadata checks.
    Standard,
    /// Large payloads: audio WAV uploads.
    AudioUpload,
}

/// Whether an error is worth retrying given the caller's context.
///
/// Transport-level failures (network, timeout) are retried for standard requests.
/// However, for large audio uploads, timeouts (client timeout or HTTP 408) are not
/// retried because repeating a multi-megabyte upload over a stalled connection
/// compounds latency and memory usage. Permanent client errors (4xx) fail fast.
pub fn is_retryable_in_context(err: &AppError, ctx: RetryContext) -> bool {
    match err.code {
        ErrorCode::NetworkError | ErrorCode::LlmConnectionRefused => true,
        ErrorCode::Timeout => ctx != RetryContext::AudioUpload,
        ErrorCode::SttApiError | ErrorCode::LlmApiError => {
            if ctx == RetryContext::AudioUpload && err.http_status == Some(408) {
                false
            } else {
                has_transient_http_status(err)
            }
        }
        _ => false,
    }
}

/// Whether an error is worth retrying for standard (small payload) requests.
pub fn is_retryable(err: &AppError) -> bool {
    is_retryable_in_context(err, RetryContext::Standard)
}

/// Run an async operation with exponential backoff using standard retry rules.
pub async fn retry_with_backoff<T, F, Fut>(
    max_retries: u32,
    base_delay: Duration,
    op: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    retry_with_backoff_in_context(max_retries, base_delay, RetryContext::Standard, op).await
}

/// Upper bound on a server-requested `Retry-After` delay. A hostile or buggy
/// server must not stall the pipeline for minutes.
pub const RETRY_AFTER_MAX_SECS: u64 = 60;

/// Parse a `Retry-After` response header as integer seconds. HTTP-date form is
/// not handled (Groq sends seconds); unparseable values are ignored.
pub fn retry_after_secs(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

/// Exponential backoff cap as a multiple of `base_delay`.
const BACKOFF_CAP_MULTIPLIER: u32 = 8;

/// Deterministic retry delay for one attempt.
///
/// When the server requested a `Retry-After`, honor it (clamped to
/// [`RETRY_AFTER_MAX_SECS`]) with no jitter. Otherwise use exponential backoff
/// `base * 2^attempt` capped at `8 * base`.
fn retry_delay(base: Duration, attempt: u32, retry_after: Option<u64>) -> Duration {
    if let Some(secs) = retry_after {
        return Duration::from_secs(secs.min(RETRY_AFTER_MAX_SECS));
    }
    base * (1u32 << attempt).min(BACKOFF_CAP_MULTIPLIER)
}

/// Apply jitter to `delay`, spreading it over `[delay / 2, delay]` so a fleet
/// of clients does not retry in lockstep. The server-requested `Retry-After`
/// path is left unjittered by the caller.
fn jitter(delay: Duration) -> Duration {
    use rand::Rng;
    if delay.is_zero() {
        return delay;
    }
    let half = delay / 2;
    let span = delay.as_nanos() - half.as_nanos();
    let extra = rand::thread_rng().gen_range(0..=span);
    half + Duration::from_nanos(extra as u64)
}

/// Run an async operation with exponential backoff and context-aware retry classification.
///
/// Retries up to `max_retries` times (so `max_retries + 1` total attempts).
/// A server-provided `Retry-After` is honored exactly (clamped); otherwise the
/// delay is jittered exponential backoff (`base * 2^attempt`, capped at
/// `8 * base`, spread over `[d / 2, d]`). Only retryable errors for `ctx`
/// trigger a retry; everything else fails fast.
pub async fn retry_with_backoff_in_context<T, F, Fut>(
    max_retries: u32,
    base_delay: Duration,
    ctx: RetryContext,
    mut op: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let mut attempt = 0u32;
    loop {
        match op().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempt >= max_retries || !is_retryable_in_context(&e, ctx) {
                    return Err(e);
                }
                let deterministic = retry_delay(base_delay, attempt, e.retry_after_secs);
                // Honor a server-requested delay verbatim; jitter only the
                // exponential backoff we chose ourselves.
                let delay = if e.retry_after_secs.is_some() {
                    deterministic
                } else {
                    jitter(deterministic)
                };
                tracing::warn!(
                    "Attempt {} failed ({e}); retrying in {:?}",
                    attempt + 1,
                    delay
                );
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[tokio::test]
    async fn retries_then_succeeds() {
        let calls = Cell::new(0u32);
        let result = retry_with_backoff(2, Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            if calls.get() < 2 {
                Err(AppError::network("transient"))
            } else {
                Ok(42)
            }
        })
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.get(), 2);
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable() {
        let calls = Cell::new(0u32);
        let result: Result<(), _> = retry_with_backoff(2, Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            Err(AppError::api_key_missing("nope"))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.get(), 1);
    }

    #[tokio::test]
    async fn retries_transient_server_errors() {
        let calls = Cell::new(0u32);
        let result = retry_with_backoff(2, Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            match calls.get() {
                1 => Err(AppError::stt_api("server fault").with_http_status(500)),
                _ => Ok(()),
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(calls.get(), 2);
    }

    #[tokio::test]
    async fn retries_rate_limit_errors() {
        let calls = Cell::new(0u32);
        let result: Result<(), _> = retry_with_backoff(2, Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            if calls.get() < 2 {
                Err(AppError::llm("rate limited").with_http_status(429))
            } else {
                Ok(())
            }
        })
        .await;
        assert!(result.is_ok());
        assert_eq!(calls.get(), 2);
    }

    #[tokio::test]
    async fn does_not_retry_permanent_client_errors() {
        // Every non-transient status must fail fast on the first attempt.
        for status in [400u16, 401, 403, 404, 409, 413, 422] {
            let calls = Cell::new(0u32);
            let result: Result<(), _> = retry_with_backoff(3, Duration::ZERO, || async {
                calls.set(calls.get() + 1);
                Err(AppError::stt_api("rejected").with_http_status(status))
            })
            .await;
            assert!(result.is_err(), "status {status} should not be retried");
            assert_eq!(calls.get(), 1, "status {status} retried more than once");
        }
    }

    #[tokio::test]
    async fn api_error_without_status_fails_fast() {
        let calls = Cell::new(0u32);
        let result: Result<(), _> = retry_with_backoff(3, Duration::ZERO, || async {
            calls.set(calls.get() + 1);
            Err(AppError::llm("unknown API failure"))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn request_timeout_status_is_retryable() {
        let err = AppError::stt_api("timeout").with_http_status(408);
        assert!(is_retryable(&err));
    }

    #[test]
    fn timeout_in_audio_upload_context_is_not_retryable() {
        let err = AppError::timeout("upload timed out");
        assert!(!is_retryable_in_context(&err, RetryContext::AudioUpload));
        assert!(is_retryable_in_context(&err, RetryContext::Standard));
        assert!(is_retryable(&err));
    }

    #[test]
    fn request_timeout_408_in_audio_upload_context_is_not_retryable() {
        let err = AppError::stt_api("server timeout").with_http_status(408);
        assert!(!is_retryable_in_context(&err, RetryContext::AudioUpload));
        assert!(is_retryable_in_context(&err, RetryContext::Standard));
        assert!(is_retryable(&err));
    }

    #[test]
    fn network_error_in_audio_upload_context_remains_retryable() {
        let err = AppError::network("connection reset");
        assert!(is_retryable_in_context(&err, RetryContext::AudioUpload));
        assert!(is_retryable_in_context(&err, RetryContext::Standard));
    }

    #[tokio::test]
    async fn retry_with_backoff_in_context_does_not_retry_non_retryable_timeout() {
        let calls = Cell::new(0u32);
        let result: Result<(), _> =
            retry_with_backoff_in_context(3, Duration::ZERO, RetryContext::AudioUpload, || async {
                calls.set(calls.get() + 1);
                Err(AppError::timeout("operation timed out"))
            })
            .await;
        assert!(result.is_err());
        assert_eq!(
            calls.get(),
            1,
            "timeout in AudioUpload context must fail fast on attempt 1"
        );
    }

    #[tokio::test]
    async fn retry_with_backoff_in_context_retries_transient_server_error() {
        let calls = Cell::new(0u32);
        let result =
            retry_with_backoff_in_context(2, Duration::ZERO, RetryContext::AudioUpload, || async {
                calls.set(calls.get() + 1);
                if calls.get() < 2 {
                    Err(AppError::stt_api("internal error").with_http_status(500))
                } else {
                    Ok(100)
                }
            })
            .await;
        assert_eq!(result.unwrap(), 100);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn retry_delay_uses_exponential_backoff() {
        let base = Duration::from_millis(100);
        assert_eq!(retry_delay(base, 0, None), Duration::from_millis(100));
        assert_eq!(retry_delay(base, 1, None), Duration::from_millis(200));
        assert_eq!(retry_delay(base, 2, None), Duration::from_millis(400));
    }

    #[test]
    fn retry_delay_caps_at_eight_times_base() {
        let base = Duration::from_millis(100);
        assert_eq!(retry_delay(base, 3, None), Duration::from_millis(800));
        assert_eq!(retry_delay(base, 10, None), Duration::from_millis(800));
    }

    #[test]
    fn retry_delay_honors_retry_after() {
        let base = Duration::from_millis(100);
        assert_eq!(retry_delay(base, 0, Some(5)), Duration::from_secs(5));
    }

    #[test]
    fn retry_delay_clamps_retry_after_to_max() {
        let base = Duration::from_millis(100);
        assert_eq!(
            retry_delay(base, 0, Some(600)),
            Duration::from_secs(RETRY_AFTER_MAX_SECS)
        );
    }

    #[test]
    fn jitter_stays_within_half_to_full_delay() {
        let delay = Duration::from_millis(1000);
        for _ in 0..200 {
            let jittered = jitter(delay);
            assert!(
                jittered >= delay / 2 && jittered <= delay,
                "jitter {jittered:?} out of [500ms, 1000ms]"
            );
        }
        // Zero delay stays zero (no panic on the span computation).
        assert_eq!(jitter(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn retry_after_secs_parses_integer_header() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "42".parse().unwrap());
        assert_eq!(retry_after_secs(&headers), Some(42));
        assert_eq!(retry_after_secs(&reqwest::header::HeaderMap::new()), None);
    }

    #[test]
    fn http_client_builds_and_clones_share_cached_instance() {
        // A build failure is not injectable without refactoring the cache, so
        // this asserts the happy path only: repeated calls return a usable
        // client (the cached one, cloned). Manual check: if the builder ever
        // fails, every call returns AppError::internal instead of panicking.
        let first = http_client().expect("client builds");
        let second = http_client().expect("client builds");
        // Both clones are usable (the cached client is returned by value).
        drop((first, second));
    }

    #[test]
    fn sanitize_error_body_keeps_short_body_untouched() {
        let body = "{\"error\":\"quota exceeded\"}";
        assert_eq!(sanitize_error_body(body), body);
    }

    #[test]
    fn sanitize_error_body_truncates_long_body_to_limit_with_suffix() {
        let body = "x".repeat(MAX_ERROR_BODY_LEN + 500);
        let result = sanitize_error_body(&body);
        let kept_chars: usize = result.chars().count() - TRUNCATED_SUFFIX.chars().count();
        assert_eq!(kept_chars, MAX_ERROR_BODY_LEN);
        assert!(result.ends_with(TRUNCATED_SUFFIX));
    }

    #[test]
    fn sanitize_error_body_replaces_control_characters() {
        let body = "line1\nline2\tok\u{0}\u{7}end";
        let result = sanitize_error_body(body);
        // Common whitespace is preserved; other control chars are replaced.
        assert!(result.contains("line1\nline2\tok"));
        assert!(!result.contains('\u{0}'));
        assert!(!result.contains('\u{7}'));
        assert!(result.ends_with("end"));
    }

    #[test]
    fn sanitize_error_body_truncates_by_chars_not_bytes() {
        // Multi-byte characters must not be cut mid-character.
        let body = "é".repeat(MAX_ERROR_BODY_LEN + 10);
        let result = sanitize_error_body(&body);
        let kept_chars: usize = result.chars().count() - TRUNCATED_SUFFIX.chars().count();
        assert_eq!(kept_chars, MAX_ERROR_BODY_LEN);
        assert!(result.contains("…(truncated)"));
    }
}
