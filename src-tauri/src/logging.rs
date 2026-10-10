//! Application logging / lightweight error tracking.
//!
//! Logs to both stderr (dev) and a daily-rotated file under the active data
//! directory's `logs/` folder. This is the open-source, self-hosted alternative
//! to a paid crash-reporting service: a user can attach the log file to a bug
//! report.
//!
//! Stderr logging starts first, before the data directory is resolved, so
//! bootstrap and data-directory failures stay visible. The file layer is
//! attached afterwards (see [`attach_file_layer`]) so files land in the active
//! directory even after a data-directory move.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::Registry;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, reload, EnvFilter};

const LOG_FILE_NAME: &str = "voxitype.log";
// Release builds log at info by default; set RUST_LOG to opt into debug
// (e.g. `RUST_LOG=voxitype_lib=debug`).
const DEFAULT_LOG_FILTER: &str = "info";

/// Number of daily log files kept before the oldest are pruned.
const MAX_LOG_FILES: usize = 14;

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

/// The file layer type wrapped in the reload handle: present once the active
/// data directory is known, absent before that (stderr only).
type FileLayer = fmt::Layer<
    Registry,
    fmt::format::DefaultFields,
    fmt::format::Format,
    tracing_appender::non_blocking::NonBlocking,
>;

/// Handle used to attach the file layer after the active data directory is known.
pub type FileLayerHandle = reload::Handle<Option<FileLayer>, Registry>;

/// Creates a non-blocking daily rolling file writer in `log_dir`, retaining
/// only the most recent [`MAX_LOG_FILES`] daily files.
pub fn create_file_writer(
    log_dir: &Path,
) -> Result<(tracing_appender::non_blocking::NonBlocking, WorkerGuard), LogInitError> {
    if let Err(err) = std::fs::create_dir_all(log_dir) {
        return Err(LogInitError::WriterFailed(err));
    }

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_NAME)
        .max_log_files(MAX_LOG_FILES)
        .build(log_dir)
        .map_err(|err| LogInitError::WriterFailed(std::io::Error::other(err.to_string())))?;
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    Ok((file_writer, guard))
}

/// Initialize tracing with stderr logging only, returning a handle used to
/// attach the file layer later via [`attach_file_layer`].
///
/// Starting stderr-only keeps bootstrap diagnostics visible before the data
/// directory (and thus the log directory) is known. The reload layer is added
/// first so its subscriber type parameter is `Registry`.
pub fn init() -> Result<FileLayerHandle, LogInitError> {
    let (file_layer, handle) = reload::Layer::<Option<FileLayer>, Registry>::new(None);
    match tracing_subscriber::registry()
        .with(file_layer)
        .with(default_filter())
        .with(fmt::layer().with_target(false))
        .try_init()
    {
        Ok(()) => Ok(handle),
        Err(_) => {
            tracing::warn!("Tracing subscriber already initialized; new subscriber ignored");
            Err(LogInitError::AlreadyInitialized)
        }
    }
}

/// Attach the daily-rotated file layer for `log_dir` to the running subscriber.
///
/// Returns a [`WorkerGuard`] that must be kept alive for the lifetime of the
/// application so buffered file logs are flushed on shutdown.
pub fn attach_file_layer(
    handle: &FileLayerHandle,
    log_dir: &Path,
) -> Result<WorkerGuard, LogInitError> {
    let (file_writer, guard) = create_file_writer(log_dir)?;
    let layer = fmt::layer()
        .with_ansi(false)
        .with_target(false)
        .with_writer(file_writer);
    handle
        .reload(Some(layer))
        .map_err(|err| LogInitError::WriterFailed(std::io::Error::other(err.to_string())))?;
    tracing::info!("File logging initialized at {}", log_dir.display());
    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_distinguishes_already_initialized() {
        let _handle = init();
        // Second call must return AlreadyInitialized error, not panic or fail silently.
        let result2 = init();
        assert!(matches!(result2, Err(LogInitError::AlreadyInitialized)));
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

        assert!(dir_contains(&dir, test_message));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The file layer attached after startup (the L-6 reload path) must receive
    /// events logged after the attach.
    #[test]
    fn test_attach_file_layer_receives_events_after_reload() {
        let dir =
            std::env::temp_dir().join(format!("voxitype_test_reload_{}", uuid::Uuid::new_v4()));
        let (file_layer, handle) = reload::Layer::<Option<FileLayer>, Registry>::new(None);
        let subscriber = tracing_subscriber::registry()
            .with(file_layer)
            .with(default_filter())
            .with(fmt::layer().with_target(false));

        let test_message = "reload attach: file logging entry after startup";
        let guard = tracing::subscriber::with_default(subscriber, || {
            let guard = attach_file_layer(&handle, &dir).expect("attach file layer");
            tracing::error!("{test_message}");
            guard
        });

        // Drop guard to flush the non-blocking worker thread.
        drop(guard);

        assert!(
            dir_contains(&dir, test_message),
            "events logged after attach must reach the file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn dir_contains(dir: &Path, needle: &str) -> bool {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return false;
        };
        entries.flatten().any(|entry| {
            entry.path().is_file()
                && std::fs::read_to_string(entry.path())
                    .unwrap_or_default()
                    .contains(needle)
        })
    }
}
