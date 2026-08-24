//! Resolves the application data directory from the default-directory marker.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, Result};

const MIN_FREE_SPACE_BYTES: u64 = 50 * 1024 * 1024;

/// Marker filename stored in the default application data directory.
pub const DATA_DIR_MARKER_FILE: &str = "data_dir.txt";

#[derive(Debug, Deserialize, Serialize)]
struct DataDirMarker {
    current: PathBuf,
    pending: Option<PathBuf>,
}

/// Resolves a valid custom data directory, falling back to `default_dir`.
pub fn resolve_app_data_dir(default_dir: PathBuf) -> PathBuf {
    match resolve_custom_dir(&default_dir) {
        Ok(Some(custom_dir)) => custom_dir,
        Ok(None) => default_dir,
        Err(error) => {
            tracing::warn!("Data directory marker ignored: {error}");
            let _ = std::fs::remove_file(default_dir.join(DATA_DIR_MARKER_FILE));
            default_dir
        }
    }
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
            tracing::warn!("Data directory migration skipped; using default directory: {error}");
            let _ = std::fs::remove_file(default_dir.join(DATA_DIR_MARKER_FILE));
            default_dir.to_path_buf()
        }
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
    if !source_db.is_file() {
        return Err(AppError::data_directory("source database does not exist"));
    }
    if !source_key.is_file() {
        return Err(AppError::data_directory("source master.key does not exist"));
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

    let custom_dir = serde_json::from_str::<DataDirMarker>(&marker)
        .ok()
        .and_then(|marker| {
            marker
                .pending
                .or((marker.current != default_dir).then_some(marker.current))
        })
        .unwrap_or_else(|| PathBuf::from(marker.trim()));
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
    std::fs::write(
        default_dir.join(DATA_DIR_MARKER_FILE),
        serde_json::to_vec(&marker)?,
    )
    .map_err(|error| {
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
    fn missing_source_falls_back_and_removes_marker() {
        let source = TempDir::new();
        let target = TempDir::new();
        std::fs::write(
            source.0.join(DATA_DIR_MARKER_FILE),
            target.0.to_string_lossy().as_bytes(),
        )
        .unwrap();

        assert_eq!(migrate_data_if_needed(&source.0, &target.0), source.0);
        assert!(!source.0.join(DATA_DIR_MARKER_FILE).exists());
    }
}
