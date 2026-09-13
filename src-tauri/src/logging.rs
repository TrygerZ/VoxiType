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

/// Initialize tracing. Returns a [`WorkerGuard`] that must be kept alive for
/// the lifetime of the app so buffered file logs are flushed on shutdown.
///
/// Idempotent. If called multiple times, subsequent calls return None without
/// panicking. If a file appender cannot be created, falls back to stderr only.
pub fn init(log_dir: &Path) -> Option<WorkerGuard> {
    let filter = || {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,voxitype_lib=debug"))
    };

    if std::fs::create_dir_all(log_dir).is_err() {
        let _ = tracing_subscriber::registry()
            .with(filter())
            .with(fmt::layer().with_target(false))
            .try_init();
        return None;
    }

    let file_appender = tracing_appender::rolling::daily(log_dir, "voxitype.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let installed = tracing_subscriber::registry()
        .with(filter())
        .with(fmt::layer().with_target(false))
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .with_writer(file_writer),
        )
        .try_init()
        .is_ok();

    if installed {
        tracing::info!("Logging initialized at {}", log_dir.display());
        Some(guard)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_idempotent() {
        let dir = std::env::temp_dir().join(format!("voxitype_test_logs_{}", uuid::Uuid::new_v4()));
        let _guard1 = init(&dir);
        // Second call must not panic even if subscriber is already installed.
        let guard2 = init(&dir);
        assert!(guard2.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
