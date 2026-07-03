//! API key encryption at rest (AES-256-GCM).
//!
//! API keys are stored encrypted in the settings table. The 256-bit master key
//! is generated once with a CSPRNG and persisted to `{app_data_dir}/master.key`
//! (0600 where supported). This keeps plaintext keys off disk; for a local,
//! single-user open-source app this is a reasonable threat model without
//! pulling in an OS keychain dependency.

use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::RngCore;

use crate::error::{AppError, Result};

/// Prefix marking a value as encrypted, to distinguish from legacy plaintext.
const ENC_PREFIX: &str = "enc:v1:";
const NONCE_LEN: usize = 12;

/// Load or create the 32-byte master key under `app_data_dir`.
///
/// If the key file exists but cannot be read or is not exactly 32 bytes, this
/// returns an error rather than overwriting it. Silently regenerating would
/// permanently orphan every already-encrypted API key, so a corrupt/locked key
/// file must surface as a failure the user can act on (e.g. restore a backup).
pub fn get_master_key(app_data_dir: &Path) -> Result<[u8; 32]> {
    let path = key_path(app_data_dir);
    match std::fs::read(&path) {
        Ok(bytes) => {
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                Ok(key)
            } else {
                Err(AppError::internal(format!(
                    "master.key is corrupt: expected 32 bytes, found {}",
                    bytes.len()
                )))
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => generate_master_key(&path),
        Err(e) => Err(AppError::internal(format!(
            "failed to read master.key: {e}"
        ))),
    }
}

/// Generate, persist, and return a fresh 32-byte master key at `path`.
fn generate_master_key(path: &Path) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, key)?;
    restrict_permissions(path);
    Ok(key)
}

fn key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("master.key")
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) {}

/// Whether a stored value is in our encrypted envelope format.
pub fn is_encrypted(value: &str) -> bool {
    value.starts_with(ENC_PREFIX)
}

/// Encrypt `plaintext` into a self-describing `enc:v1:<base64(nonce|ct)>` string.
pub fn encrypt_api_key(plaintext: &str, master_key: &[u8; 32]) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| AppError::internal(format!("encrypt failed: {e}")))?;

    let mut blob = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);
    Ok(format!("{ENC_PREFIX}{}", B64.encode(blob)))
}

/// Decrypt a value produced by [`encrypt_api_key`]. Plaintext (no prefix) is
/// returned unchanged for backward compatibility / migration.
pub fn decrypt_api_key(stored: &str, master_key: &[u8; 32]) -> Result<String> {
    if stored.is_empty() {
        return Ok(String::new());
    }
    let Some(b64) = stored.strip_prefix(ENC_PREFIX) else {
        return Ok(stored.to_string());
    };
    let blob = B64
        .decode(b64)
        .map_err(|e| AppError::internal(format!("base64 decode failed: {e}")))?;
    if blob.len() <= NONCE_LEN {
        return Err(AppError::internal("ciphertext too short"));
    }
    let (nonce_bytes, ciphertext) = blob.split_at(NONCE_LEN);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|e| AppError::internal(format!("decrypt failed: {e}")))?;
    String::from_utf8(plaintext).map_err(|e| AppError::internal(format!("utf8 error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = [7u8; 32];
        let enc = encrypt_api_key("gsk_secret_123", &key).unwrap();
        assert!(is_encrypted(&enc));
        assert_ne!(enc, "gsk_secret_123");
        let dec = decrypt_api_key(&enc, &key).unwrap();
        assert_eq!(dec, "gsk_secret_123");
    }

    #[test]
    fn plaintext_passthrough() {
        let key = [1u8; 32];
        // Legacy plaintext without prefix decrypts to itself.
        assert_eq!(decrypt_api_key("plainkey", &key).unwrap(), "plainkey");
    }

    #[test]
    fn empty_is_empty() {
        let key = [0u8; 32];
        assert_eq!(encrypt_api_key("", &key).unwrap(), "");
        assert_eq!(decrypt_api_key("", &key).unwrap(), "");
    }

    #[test]
    fn wrong_key_fails() {
        let enc = encrypt_api_key("secret", &[2u8; 32]).unwrap();
        assert!(decrypt_api_key(&enc, &[3u8; 32]).is_err());
    }

    #[test]
    fn generates_then_reloads_same_key() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_ok_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = get_master_key(&dir).unwrap();
        // Second call must load the persisted key, not regenerate a new one.
        let second = get_master_key(&dir).unwrap();
        assert_eq!(first, second);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_key_file_errors_instead_of_overwriting() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_bad_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Write a too-short key file: must NOT be silently regenerated.
        std::fs::write(key_path(&dir), [1u8; 16]).unwrap();
        assert!(get_master_key(&dir).is_err());
        // The corrupt file is left intact for recovery.
        assert_eq!(std::fs::read(key_path(&dir)).unwrap().len(), 16);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
