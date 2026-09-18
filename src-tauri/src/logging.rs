//! Application logging / lightweight error tracking.
//!
//! Logs to both stderr (dev) and a daily-rotated file under the app log dir
//! (`{app_data_dir}/logs/voxitype.log.<date>`). This is the open-source,
//! self-hosted alternative to a paid crash-reporting service: a user can attach
//! the log file to a bug report.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

const LOG_FILE_NAME: &str = "voxitype.log";
const DEFAULT_LOG_FILTER: &str = "info,voxitype_lib=debug";

/// Error initializing file logging.
#[derive(Debug)]
pub enum LogInitError {
    /// A global tracing subscriber is already initialized in this process.
    AlreadyInitialized,
    /// Failed to create the directory or file appender for logging.
    WriterFailed(std::io::Error),
}

impl std::fmt::Display for LogInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "tracing subscriber already initialized"),
            Self::WriterFailed(err) => write!(f, "failed to create log writer: {err}"),
        }
    }
}

impl std::error::Error for LogInitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyInitialized => None,
            Self::WriterFailed(err) => Some(err),
        }
    }
}

fn default_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER))
}

/// Creates a non-blocking daily rolling file writer in `log_dir`.
pub fn create_file_writer(
    log_dir: &Path,
) -> Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard), LogInitError> {
    if let Err(err) = std::fs::create_dir_all(log_dir) {
        return Err(LogInitError::WriterFailed(err));
    }

    let file_appender = tracing_appender::rolling::daily(log_dir, LOG_FILE_NAME);
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    Ok((file_writer, guard))
}

/// Initialize tracing with both stderr and daily-rotated file logging.
///
/// Returns a [`WorkerGuard`] that must be kept alive for the lifetime of
/// the application so buffered file logs are flushed on shutdown.
pub fn init(log_dir: &Path) -> Result<WorkerGuard, LogInitError> {
    let (file_writer, guard) = match create_file_writer(log_dir) {
        Ok(pair) => pair,
        Err(err) => {
            // Fall back to stderr subscriber so console logs still work when file logging fails.
            let _ = tracing_subscriber::registry()
                .with(default_filter())
                .with(fmt::layer().with_target(false))
                .try_init();
            return Err(err);
        }
    };

    match tracing_subscriber::registry()
        .with(default_filter())
        .with(fmt::layer().with_target(false))
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .with_writer(file_writer),
        )
        .try_init()
    {
        Ok(()) => {
            tracing::info!("Logging initialized at {}", log_dir.display());
            Ok(guard)
        }
        Err(_) => {
            tracing::warn!("Tracing subscriber already initialized; new subscriber ignored");
            Err(LogInitError::AlreadyInitialized)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_distinguishes_already_initialized() {
        let dir = std::env::temp_dir().join(format!("voxitype_test_logs_{}", uuid::Uuid::new_v4()));
        let _guard1 = init(&dir);
        // Second call must return AlreadyInitialized error, not panic or fail silently.
        let result2 = init(&dir);
        assert!(matches!(result2, Err(LogInitError::AlreadyInitialized)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_file_writer_fails_on_file_as_dir() {
        let temp_file =
            std::env::temp_dir().join(format!("voxitype_file_{}", uuid::Uuid::new_v4()));
        std::fs::write(&temp_file, b"not a dir").expect("failed to write dummy file");
        // Passing a file path as directory will cause create_dir_all to fail.
        let result = create_file_writer(&temp_file);
        assert!(matches!(result, Err(LogInitError::WriterFailed(_))));
        let _ = std::fs::remove_file(&temp_file);
    }

    #[test]
    fn test_file_writer_receives_log_events() {
        let dir =
            std::env::temp_dir().join(format!("voxitype_test_file_write_{}", uuid::Uuid::new_v4()));
        let (file_writer, guard) = create_file_writer(&dir).expect("failed to create file writer");

        let subscriber = tracing_subscriber::registry().with(
            fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .with_writer(file_writer),
        );

        let test_message = "fallback error: data directory recovery test entry";
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!("{test_message}");
        });

        // Drop guard to flush the non-blocking worker thread.
        drop(guard);

        let entries = std::fs::read_dir(&dir).expect("failed to read log dir");
        let mut found = false;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                if content.contains(test_message) {
                    found = true;
                    break;
                }
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(found, "Log file did not contain expected message");
    }
}
