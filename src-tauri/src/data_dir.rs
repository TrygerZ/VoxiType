//! Resolves the application data directory from the default-directory marker.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

const MIN_FREE_SPACE_BYTES: u64 = 50 * 1024 * 1024;

/// Marker filename stored in the default application data directory.
pub const DATA_DIR_MARKER_FILE: &str = "data_dir.txt";

/// Failure diagnostic filename stored in the default application data directory.
pub const DATA_DIR_ERROR_FILE: &str = "data_dir_error.txt";

/// Data directory status exposed to IPC clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirectoryStatus {
    pub active: String,
    pub default: String,
    pub pending: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DataDirMarker {
    current: PathBuf,
    #[serde(default)]
    pending: Option<PathBuf>,
}

/// Reads the last recorded data directory failure diagnostic if present.
pub fn read_error(default_dir: &Path) -> Option<String> {
    let error_path = default_dir.join(DATA_DIR_ERROR_FILE);
    std::fs::read_to_string(error_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Records a data directory failure diagnostic message.
pub fn record_error(default_dir: &Path, message: &str) {
    let error_path = default_dir.join(DATA_DIR_ERROR_FILE);
    let _ = write_atomic(&error_path, message.as_bytes());
}

/// Clears any recorded data directory failure diagnostic.
pub fn clear_error(default_dir: &Path) {
    let _ = std::fs::remove_file(default_dir.join(DATA_DIR_ERROR_FILE));
}

/// Reads any pending target directory awaiting restart from the marker.
pub fn read_pending(default_dir: &Path) -> Option<String> {
    let marker_path = default_dir.join(DATA_DIR_MARKER_FILE);
    let contents = std::fs::read_to_string(marker_path).ok()?;
    let marker = serde_json::from_str::<DataDirMarker>(&contents).ok()?;
    marker.pending.map(|p| p.to_string_lossy().into_owned())
}

/// Returns the current data directory status for frontend consumption.
pub fn get_status(default_dir: &Path, active_dir: &Path) -> DataDirectoryStatus {
    let pending = read_pending(default_dir).filter(|p| Path::new(p) != active_dir);
    let last_error = read_error(default_dir);
    DataDirectoryStatus {
        active: active_dir.to_string_lossy().into_owned(),
        default: default_dir.to_string_lossy().into_owned(),
        pending,
        last_error,
    }
}

/// Formats an error log message when custom data directory initialization,
/// resolution, or migration fails and falls back to another directory.
///
/// Ensures both the failed path and the fallback path are logged, and explicitly
/// warns that settings and history may appear empty.
pub fn fallback_error_message(
    failed_path: &Path,
    fallback_path: &Path,
    error: &dyn std::fmt::Display,
) -> String {
    format!(
        "Data directory failure for '{}': {error}; falling back to '{}'. Settings and history may appear empty.",
        failed_path.display(),
        fallback_path.display()
    )
}

/// Resolves a valid custom data directory, falling back to `default_dir`.
pub fn resolve_app_data_dir(default_dir: PathBuf) -> PathBuf {
    resolve_app_data_dir_checked(default_dir).0
}

/// Resolves a valid custom data directory, returning whether fallback occurred.
pub fn resolve_app_data_dir_checked(default_dir: PathBuf) -> (PathBuf, bool) {
    match resolve_custom_dir(&default_dir) {
        Ok(Some(custom_dir)) => (custom_dir, false),
        Ok(None) => (default_dir, false),
        Err(error) => {
            let marker_path = default_dir.join(DATA_DIR_MARKER_FILE);
            let failed_target =
                read_marker_target(&marker_path).unwrap_or_else(|| marker_path.clone());
            let message = fallback_error_message(&failed_target, &default_dir, &error);
            tracing::error!("{message}");
            record_error(&default_dir, &message);
            let _ = std::fs::remove_file(&marker_path);
            (default_dir, true)
        }
    }
}

fn read_marker_target(marker_path: &Path) -> Option<PathBuf> {
    let contents = std::fs::read_to_string(marker_path).ok()?;
    if let Ok(marker) = serde_json::from_str::<DataDirMarker>(&contents) {
        return marker.pending.or(Some(marker.current));
    }
    let trimmed = contents.trim();
    if !trimmed.is_empty() {
        return Some(PathBuf::from(trimmed));
    }
    None
}

pub fn migration_source(default_dir: &Path, target_dir: &Path) -> PathBuf {
    if target_dir == default_dir {
        return default_dir.to_path_buf();
    }
    std::fs::read_to_string(default_dir.join(DATA_DIR_MARKER_FILE))
        .ok()
        .and_then(|contents| serde_json::from_str::<DataDirMarker>(&contents).ok())
        .and_then(|marker| marker.pending.map(|_| marker.current))
        .or_else(|| {
            std::fs::read_to_string(default_dir.join(DATA_DIR_MARKER_FILE))
                .ok()
                .map(|contents| PathBuf::from(contents.trim()))
        })
        .unwrap_or_else(|| default_dir.to_path_buf())
}

/// Completes the first-use copy into a marker-selected directory.
///
/// A failed copy removes the marker and returns the legacy directory. Critical
/// files use temp-file + rename and content verification; logs are best effort.
pub fn migrate_data_if_needed(default_dir: &Path, target_dir: &Path) -> PathBuf {
    if default_dir == target_dir {
        return target_dir.to_path_buf();
    }

    match migrate_data(default_dir, target_dir) {
        Ok(()) => target_dir.to_path_buf(),
        Err(error) => {
            let message = fallback_error_message(target_dir, default_dir, &error);
            tracing::error!("{message}");
            record_error(default_dir, &message);
            let _ = std::fs::remove_file(default_dir.join(DATA_DIR_MARKER_FILE));
            default_dir.to_path_buf()
        }
    }
}

pub(crate) fn graduate_marker(default_dir: &Path, target_dir: &Path) -> Result<()> {
    let marker_path = default_dir.join(DATA_DIR_MARKER_FILE);
    let contents = std::fs::read_to_string(&marker_path).map_err(|error| {
        AppError::data_directory(format!("failed to read data directory marker: {error}"))
    })?;
    serde_json::from_str::<DataDirMarker>(&contents).map_err(|error| {
        AppError::data_directory(format!("failed to parse data directory marker: {error}"))
    })?;
    let marker = DataDirMarker {
        current: target_dir.to_path_buf(),
        pending: None,
    };
    write_atomic(&marker_path, &serde_json::to_vec(&marker)?).map_err(|error| {
        AppError::data_directory(format!("failed to graduate data directory marker: {error}"))
    })?;
    Ok(())
}

/// Completes startup data directory processing: graduates marker if needed and
/// clears stale failure diagnostics on healthy startup.
pub fn finish_startup(default_dir: &Path, active_dir: &Path, fallback_occurred: bool) {
    if active_dir != default_dir {
        if let Err(error) = graduate_marker(default_dir, active_dir) {
            tracing::warn!("Failed to graduate data directory marker: {error}");
            return;
        }
    }
    if !fallback_occurred {
        clear_error(default_dir);
    }
}

fn migrate_data(default_dir: &Path, target_dir: &Path) -> Result<()> {
    let source_db = default_dir.join("data").join("voxitype.db");
    let target_db = target_dir.join("data").join("voxitype.db");
    let source_key = default_dir.join("master.key");
    let target_key = target_dir.join("master.key");

    if target_db.exists() {
        if is_valid_sqlite_file(&target_db) {
            if target_key.is_file() {
                tracing::info!("Data migration skipped; target database already exists");
                return Ok(());
            }
            tracing::info!(
                "Recovering interrupted data migration by removing target database without master.key"
            );
            std::fs::remove_file(&target_db)?;
        } else {
            return Err(AppError::data_directory("target database is invalid"));
        }
    }
    if !source_db.is_file() || !source_key.is_file() {
        tracing::info!(
            "No existing application data to migrate; target will initialize on first use"
        );
        return Ok(());
    }
    if target_key.exists() {
        // Remove an interrupted prior attempt; the marker still points here.
        std::fs::remove_file(&target_key)?;
    }

    std::fs::create_dir_all(
        target_db
            .parent()
            .ok_or_else(|| AppError::data_directory("target database has no parent directory"))?,
    )?;
    copy_verified(&source_db, &target_db)?;
    if let Err(error) = copy_verified(&source_key, &target_key) {
        let _ = std::fs::remove_file(&target_db);
        return Err(error);
    }
    copy_logs_best_effort(default_dir, target_dir);
    tracing::info!("Migrated application data to {}", target_dir.display());
    Ok(())
}

fn copy_verified(source: &Path, destination: &Path) -> Result<()> {
    let temporary = destination.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| {
        std::fs::copy(source, &temporary)?;
        let source_meta = std::fs::metadata(source)?;
        let destination_meta = std::fs::metadata(&temporary)?;
        if source_meta.len() != destination_meta.len()
            || hash_file(source)? != hash_file(&temporary)?
        {
            return Err(AppError::data_directory(format!(
                "copy verification failed for {}",
                source.display()
            )));
        }
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn hash_file(path: &Path) -> std::io::Result<[u8; 32]> {
    let bytes = std::fs::read(path)?;
    Ok(Sha256::digest(bytes).into())
}

fn is_valid_sqlite_file(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.len() >= 16 && &bytes[..16] == b"SQLite format 3\0"
}

fn copy_logs_best_effort(source_dir: &Path, target_dir: &Path) {
    let source = source_dir.join("logs");
    let target = target_dir.join("logs");
    let Ok(entries) = std::fs::read_dir(source) else {
        return;
    };
    if let Err(error) = std::fs::create_dir_all(&target) {
        tracing::warn!("Log migration skipped: {error}");
        return;
    }
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let destination = target.join(entry.file_name());
            if let Err(error) = std::fs::copy(path, destination) {
                tracing::warn!("Log migration failed: {error}");
            }
        }
    }
}

fn write_atomic(destination: &Path, content: &[u8]) -> std::io::Result<()> {
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
        std::fs::write(&temporary, content)?;
        std::fs::rename(&temporary, destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn resolve_custom_dir(default_dir: &Path) -> Result<Option<PathBuf>> {
    let marker_path = default_dir.join(DATA_DIR_MARKER_FILE);
    let marker = match std::fs::read_to_string(&marker_path) {
        Ok(marker) => marker,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(AppError::internal(format!(
                "failed to read {}: {error}",
                marker_path.display()
            )))
        }
    };

    let target = match serde_json::from_str::<DataDirMarker>(&marker) {
        Ok(marker) => match marker.pending {
            Some(pending) => Some(pending),
            None if marker.current != default_dir => Some(marker.current),
            None => None,
        },
        Err(_) => {
            let path = PathBuf::from(marker.trim());
            (path != default_dir).then_some(path)
        }
    };
    let Some(custom_dir) = target else {
        return Ok(None);
    };
    if !custom_dir.is_absolute() {
        return Err(AppError::internal(
            "data directory marker must contain an absolute path",
        ));
    }

    validate_data_dir(&custom_dir)?;
    Ok(Some(custom_dir))
}

pub(crate) fn write_marker(default_dir: &Path, current: &Path, pending: &Path) -> Result<()> {
    validate_data_dir(pending)?;
    let marker = DataDirMarker {
        current: current.to_path_buf(),
        pending: Some(pending.to_path_buf()),
    };
    let marker_path = default_dir.join(DATA_DIR_MARKER_FILE);
    let bytes = serde_json::to_vec(&marker)?;
    write_atomic(&marker_path, &bytes).map_err(|error| {
        AppError::data_directory(format!("failed to write data directory marker: {error}"))
    })
}

pub(crate) fn validate_data_dir(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(AppError::data_directory(
            "data directory must be an absolute path",
        ));
    }
    if is_network_path(path) {
        return Err(AppError::data_directory("network drives are not supported"));
    }
    if path.exists() {
        if !path.is_dir() {
            return Err(AppError::data_directory(format!(
                "data directory target is not a directory: {}",
                path.display()
            )));
        }
    } else {
        std::fs::create_dir_all(path).map_err(|error| {
            AppError::data_directory(format!("failed to create data directory target: {error}"))
        })?;
    }

    let probe_path = path.join(format!(".voxitype-write-probe-{}", std::process::id()));
    std::fs::write(&probe_path, []).map_err(|error| {
        AppError::data_directory(format!("data directory target is not writable: {error}"))
    })?;
    std::fs::remove_file(&probe_path).map_err(|error| {
        AppError::data_directory(format!(
            "failed to remove data directory write probe: {error}"
        ))
    })?;
    ensure_free_space(path)
}

fn is_network_path(path: &Path) -> bool {
    if path.to_string_lossy().starts_with(r"\\") {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        use windows::Win32::System::WindowsProgramming::{DRIVE_REMOTE, DRIVE_REMOVABLE};
        let value = path.as_os_str().encode_wide().collect::<Vec<_>>();
        if value.get(1) != Some(&(':' as u16)) {
            return false;
        }
        let mut root = value[..value.len().min(3)].to_vec();
        root.push(0);
        let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(root.as_ptr())) };
        drive_type == DRIVE_REMOTE || drive_type == DRIVE_REMOVABLE
    }
    #[cfg(not(windows))]
    false
}

#[cfg(windows)]
fn ensure_free_space(path: &Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let mut free = 0u64;
    let result = unsafe {
        GetDiskFreeSpaceExW(
            windows::core::PCWSTR(wide.as_ptr()),
            None,
            None,
            Some(&mut free),
        )
    };
    if result.is_err() {
        return Err(AppError::data_directory(
            "unable to determine free disk space",
        ));
    }
    (free >= MIN_FREE_SPACE_BYTES)
        .then_some(())
        .ok_or_else(|| AppError::data_directory("at least 50 MB free space is required"))
}

#[cfg(not(windows))]
fn ensure_free_space(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn fallback_error_message_contains_both_paths_and_warning() {
        let failed = Path::new("D:\\custom\\path");
        let fallback = Path::new("C:\\Users\\default\\path");
        let message = fallback_error_message(failed, fallback, &"permission denied");
        assert!(message.contains("D:\\custom\\path"));
        assert!(message.contains("C:\\Users\\default\\path"));
        assert!(message.contains("Settings and history may appear empty"));
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let path = std::env::temp_dir().join(format!("voxitype-data-dir-{nonce}"));
            std::fs::create_dir_all(&path).expect("test temp directory should be creatable");
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn marker_absent_uses_default() {
        let default_dir = TempDir::new();
        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), default_dir.0);
    }

    #[test]
    fn valid_marker_uses_custom_directory() {
        let default_dir = TempDir::new();
        let custom_dir = default_dir.0.join("custom");
        std::fs::write(
            default_dir.0.join(DATA_DIR_MARKER_FILE),
            custom_dir.to_string_lossy().as_bytes(),
        )
        .expect("test marker should be writable");

        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), custom_dir);
    }

    #[test]
    fn invalid_marker_content_uses_default() {
        let default_dir = TempDir::new();
        std::fs::write(default_dir.0.join(DATA_DIR_MARKER_FILE), "relative/path")
            .expect("test marker should be writable");

        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), default_dir.0);
    }

    #[test]
    fn unwritable_target_uses_default() {
        let default_dir = TempDir::new();
        let target_file = default_dir.0.join("not-a-directory");
        std::fs::write(&target_file, []).expect("test target should be writable");
        std::fs::write(
            default_dir.0.join(DATA_DIR_MARKER_FILE),
            target_file.to_string_lossy().as_bytes(),
        )
        .expect("test marker should be writable");

        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), default_dir.0);
    }

    #[test]
    fn rejects_relative_and_network_paths() {
        assert!(validate_data_dir(Path::new("relative")).is_err());
        assert!(validate_data_dir(Path::new(r"\\server\share")).is_err());
    }

    #[test]
    fn marker_write_read_roundtrip() {
        let default_dir = TempDir::new();
        let target = TempDir::new();
        write_marker(&default_dir.0, &default_dir.0, &target.0).expect("marker should be writable");
        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), target.0);
    }

    #[test]
    fn atomic_write_marker_produces_valid_marker() {
        let default_dir = TempDir::new();
        let target = TempDir::new();
        write_marker(&default_dir.0, &default_dir.0, &target.0).expect("marker should be writable");

        let marker_file = default_dir.0.join(DATA_DIR_MARKER_FILE);
        assert!(marker_file.exists());
        let contents = std::fs::read_to_string(&marker_file).expect("marker should be readable");
        let parsed: serde_json::Value =
            serde_json::from_str(&contents).expect("marker should be valid json");
        assert_eq!(parsed["current"], default_dir.0.to_string_lossy().as_ref());
        assert_eq!(parsed["pending"], target.0.to_string_lossy().as_ref());

        let entries = std::fs::read_dir(&default_dir.0).unwrap();
        let tmp_files: Vec<_> = entries
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            tmp_files.is_empty(),
            "temporary marker files should be cleaned up"
        );
    }

    #[test]
    fn pending_resolve_roundtrip_and_status() {
        let default_dir = TempDir::new();
        let target = TempDir::new();
        write_marker(&default_dir.0, &default_dir.0, &target.0).expect("marker should be writable");

        assert_eq!(
            read_pending(&default_dir.0),
            Some(target.0.to_string_lossy().into_owned())
        );
        let status = get_status(&default_dir.0, &default_dir.0);
        assert_eq!(status.active, default_dir.0.to_string_lossy());
        assert_eq!(status.default, default_dir.0.to_string_lossy());
        assert_eq!(
            status.pending,
            Some(target.0.to_string_lossy().into_owned())
        );
        assert_eq!(status.last_error, None);

        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), target.0);
    }

    #[test]
    fn invalid_target_records_diagnostic_and_falls_back() {
        let default_dir = TempDir::new();
        let invalid_target = default_dir.0.join("not-a-directory");
        std::fs::write(&invalid_target, []).unwrap();
        std::fs::write(
            default_dir.0.join(DATA_DIR_MARKER_FILE),
            invalid_target.to_string_lossy().as_bytes(),
        )
        .unwrap();

        let resolved = resolve_app_data_dir(default_dir.0.clone());
        assert_eq!(resolved, default_dir.0);
        assert!(!default_dir.0.join(DATA_DIR_MARKER_FILE).exists());

        let err = read_error(&default_dir.0);
        assert!(err.is_some(), "diagnostic error should be recorded");
        let msg = err.unwrap();
        assert!(msg.contains("not-a-directory"));
        assert!(msg.contains("Settings and history may appear empty"));

        let status = get_status(&default_dir.0, &default_dir.0);
        assert_eq!(status.active, default_dir.0.to_string_lossy());
        assert_eq!(status.last_error, Some(msg));
    }

    #[test]
    fn pending_marker_graduates_to_stable_marker() {
        let default_dir = TempDir::new();
        let target = TempDir::new();
        write_marker(&default_dir.0, &default_dir.0, &target.0).expect("marker should be writable");

        graduate_marker(&default_dir.0, &target.0).expect("marker should graduate");

        let marker = std::fs::read_to_string(default_dir.0.join(DATA_DIR_MARKER_FILE))
            .expect("graduated marker should be readable");
        let parsed: serde_json::Value =
            serde_json::from_str(&marker).expect("marker should be JSON");
        assert_eq!(parsed["current"], target.0.to_string_lossy().as_ref());
        assert!(parsed.get("pending").is_none() || parsed["pending"].is_null());
    }

    #[test]
    fn resolver_accepts_pending_stable_and_plain_markers() {
        let default_dir = TempDir::new();
        let target = TempDir::new();
        for marker in [
            serde_json::json!({ "current": default_dir.0, "pending": target.0 }),
            serde_json::json!({ "current": target.0 }),
        ] {
            std::fs::write(
                default_dir.0.join(DATA_DIR_MARKER_FILE),
                serde_json::to_vec(&marker).expect("marker should serialize"),
            )
            .expect("marker should be writable");
            assert_eq!(resolve_app_data_dir(default_dir.0.clone()), target.0);
        }
        std::fs::write(
            default_dir.0.join(DATA_DIR_MARKER_FILE),
            target.0.to_string_lossy().as_bytes(),
        )
        .expect("marker should be writable");
        assert_eq!(resolve_app_data_dir(default_dir.0.clone()), target.0);
    }

    #[test]
    fn fresh_install_migration_leaves_target_for_initialization() {
        let source = TempDir::new();
        let target = TempDir::new();
        write_marker(&source.0, &source.0, &target.0).expect("marker should be writable");

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), target.0);
        assert!(!target.0.join("data/voxitype.db").exists());
        assert!(source.0.join(DATA_DIR_MARKER_FILE).exists());
    }

    #[test]
    fn migration_uses_previous_active_directory() {
        let default_dir = TempDir::new();
        let first = TempDir::new();
        let second = TempDir::new();
        std::fs::create_dir_all(default_dir.0.join("data")).unwrap();
        std::fs::write(
            default_dir.0.join("data/voxitype.db"),
            b"SQLite format 3\0default",
        )
        .unwrap();
        std::fs::write(default_dir.0.join("master.key"), [1u8; 32]).unwrap();

        write_marker(&default_dir.0, &default_dir.0, &first.0).unwrap();
        migrate_data_if_needed(&default_dir.0, &first.0);
        std::fs::write(first.0.join("data/voxitype.db"), b"SQLite format 3\0latest").unwrap();
        write_marker(&default_dir.0, &first.0, &second.0).unwrap();

        assert_eq!(migration_source(&default_dir.0, &second.0), first.0);
        migrate_data_if_needed(&first.0, &second.0);
        assert_eq!(
            std::fs::read(second.0.join("data/voxitype.db")).unwrap(),
            b"SQLite format 3\0latest"
        );
    }

    #[test]
    fn interrupted_target_without_key_is_recovered() {
        let source = TempDir::new();
        let target = TempDir::new();
        std::fs::create_dir_all(source.0.join("data")).unwrap();
        std::fs::write(
            source.0.join("data/voxitype.db"),
            b"SQLite format 3\0source",
        )
        .unwrap();
        std::fs::write(source.0.join("master.key"), [7u8; 32]).unwrap();
        std::fs::create_dir_all(target.0.join("data")).unwrap();
        std::fs::write(target.0.join("data/voxitype.db"), b"SQLite format 3\0stale").unwrap();

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), target.0);
        assert_eq!(
            std::fs::read(target.0.join("master.key")).unwrap(),
            vec![7u8; 32]
        );
    }

    #[test]
    fn fresh_target_copies_critical_files() {
        let source = TempDir::new();
        let target = TempDir::new();
        let db = source.0.join("data");
        std::fs::create_dir_all(&db).unwrap();
        std::fs::write(db.join("voxitype.db"), b"SQLite format 3\0payload").unwrap();
        std::fs::write(source.0.join("master.key"), [7u8; 32]).unwrap();

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), target.0);
        assert_eq!(
            std::fs::read(target.0.join("data/voxitype.db")).unwrap(),
            std::fs::read(source.0.join("data/voxitype.db")).unwrap()
        );
        assert_eq!(
            std::fs::read(target.0.join("master.key")).unwrap(),
            vec![7u8; 32]
        );
    }

    #[test]
    fn existing_target_database_is_noop() {
        let source = TempDir::new();
        let target = TempDir::new();
        std::fs::create_dir_all(target.0.join("data")).unwrap();
        std::fs::write(
            target.0.join("data/voxitype.db"),
            b"SQLite format 3\0existing",
        )
        .unwrap();
        std::fs::write(target.0.join("master.key"), [9u8; 32]).unwrap();
        std::fs::write(source.0.join("master.key"), [7u8; 32]).unwrap();
        std::fs::create_dir_all(source.0.join("data")).unwrap();
        std::fs::write(
            source.0.join("data/voxitype.db"),
            b"SQLite format 3\0source",
        )
        .unwrap();

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), target.0);
        assert_eq!(
            std::fs::read(target.0.join("master.key")).unwrap(),
            vec![9u8; 32]
        );
    }

    #[test]
    fn missing_source_keeps_marker_for_fresh_initialization() {
        let source = TempDir::new();
        let target = TempDir::new();
        std::fs::write(
            source.0.join(DATA_DIR_MARKER_FILE),
            target.0.to_string_lossy().as_bytes(),
        )
        .unwrap();

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), target.0);
        assert!(source.0.join(DATA_DIR_MARKER_FILE).exists());
    }

    #[test]
    fn clean_startup_after_fallback_clears_stale_error() {
        let default_dir = TempDir::new();
        let invalid_target = default_dir.0.join("not-a-directory");
        std::fs::write(&invalid_target, []).unwrap();
        std::fs::write(
            default_dir.0.join(DATA_DIR_MARKER_FILE),
            invalid_target.to_string_lossy().as_bytes(),
        )
        .unwrap();

        // 1. Initial failed run: resolve falls back and records diagnostic error.
        let (resolved, fallback) = resolve_app_data_dir_checked(default_dir.0.clone());
        assert_eq!(resolved, default_dir.0);
        assert!(fallback, "fallback should be detected on invalid target");
        assert!(!default_dir.0.join(DATA_DIR_MARKER_FILE).exists());
        finish_startup(&default_dir.0, &resolved, fallback);
        assert!(
            read_error(&default_dir.0).is_some(),
            "error should remain after failed startup"
        );

        // 2. Subsequent clean run without marker: resolve succeeds with default and no fallback.
        let (resolved_next, fallback_next) = resolve_app_data_dir_checked(default_dir.0.clone());
        assert_eq!(resolved_next, default_dir.0);
        assert!(!fallback_next, "clean startup should not trigger fallback");
        finish_startup(&default_dir.0, &resolved_next, fallback_next);
        assert_eq!(
            read_error(&default_dir.0),
            None,
            "stale error should be cleared on subsequent clean startup"
        );
        let status = get_status(&default_dir.0, &default_dir.0);
        assert_eq!(status.last_error, None);
    }
}
