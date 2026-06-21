//! Small shared helpers used across modules.

use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;

use crate::error::{AppError, ErrorCode};

/// Process-wide shared `reqwest` client.
///
/// Reusing one client (and its connection pool / TLS state) across requests
/// avoids the cost of rebuilding it for every recording. Cloning a `Client` is
/// cheap (it is an `Arc` internally).
pub fn http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default()
        })
        .clone()
}

/// Whether an error is worth retrying (transient network/server issues).
///
/// Auth failures and missing keys are *not* retried — they will never succeed.
pub fn is_retryable(err: &AppError) -> bool {
    matches!(
        err.code,
        ErrorCode::NetworkError
            | ErrorCode::Timeout
            | ErrorCode::SttApiError
            | ErrorCode::LlmApiError
            | ErrorCode::LlmConnectionRefused
    )
}

/// Run an async operation with exponential backoff.
///
/// Retries up to `max_retries` times (so `max_retries + 1` total attempts),
/// sleeping `base_delay * 2^attempt` between retries. Only retryable errors
/// trigger a retry; everything else fails fast.
pub async fn retry_with_backoff<T, F, Fut>(
    max_retries: u32,
    base_delay: Duration,
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
                if attempt >= max_retries || !is_retryable(&e) {
                    return Err(e);
                }
                let delay = base_delay * (1u32 << attempt);
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
}
