//! API key encryption at rest (AES-256-GCM).
//!
//! API keys are stored encrypted in the settings table. The 256-bit master key
//! is generated once with a CSPRNG and persisted to `{app_data_dir}/master.key`
//! (0600 where supported). This keeps plaintext keys off disk; for a local,
//! single-user open-source app this is a reasonable threat model without
//! pulling in an OS keychain dependency.
//!
//! Threat-model note: on Windows the master key is wrapped with DPAPI
//! (`CryptProtectData`, CurrentUser scope) before it touches disk, so
//! master.key holds user-bound ciphertext instead of usable key material —
//! inheriting the parent directory ACL no longer exposes the key to other
//! local users or processes. The Unix branch still relies on 0600 file
//! permissions. Files in the legacy 32-byte raw format are transparently
//! migrated to the DPAPI-wrapped format on first load; a corrupt or
//! undecryptable file fails fast and is never silently regenerated.

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

/// Prefix marking a master.key file whose payload is DPAPI-protected.
const DPAPI_KEY_PREFIX: &str = "dpapi:v1:";
const MASTER_KEY_LEN: usize = 32;

/// The two on-disk formats `master.key` may have.
#[derive(Debug, PartialEq, Eq)]
enum StoredKeyFile {
    /// Legacy format: the bare 32-byte key in cleartext (pre-S02 fix).
    LegacyRaw([u8; MASTER_KEY_LEN]),
    /// Current Windows format: a base64-encoded DPAPI blob that still has to
    /// be unwrapped via `CryptUnprotectData` to yield the raw key bytes.
    DpapiWrapped(Vec<u8>),
}

/// Parse raw `master.key` file contents into a [`StoredKeyFile`].
///
/// Pure byte/string logic — no I/O, no DPAPI — so it is testable on any
/// platform. Anything that is neither 32 raw bytes nor a well-formed
/// `{DPAPI_KEY_PREFIX}<base64>` record is treated as corruption.
fn parse_stored_key_file(bytes: &[u8]) -> Result<StoredKeyFile> {
    if bytes.len() == MASTER_KEY_LEN {
        let mut key = [0u8; MASTER_KEY_LEN];
        key.copy_from_slice(bytes);
        return Ok(StoredKeyFile::LegacyRaw(key));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| corrupt_key_file_error(bytes.len()))?;
    let Some(b64) = text.trim().strip_prefix(DPAPI_KEY_PREFIX) else {
        return Err(corrupt_key_file_error(bytes.len()));
    };
    let blob = B64.decode(b64.trim()).map_err(|e| {
        AppError::internal(format!("master.key has an invalid base64 payload: {e}"))
    })?;
    Ok(StoredKeyFile::DpapiWrapped(blob))
}

fn corrupt_key_file_error(actual_len: usize) -> AppError {
    AppError::internal(format!(
        "master.key is corrupt: expected either {MASTER_KEY_LEN} raw bytes \
         or a '{DPAPI_KEY_PREFIX}' record, found {actual_len} bytes"
    ))
}

/// Enforce that an unprotected payload really is a 32-byte key.
fn decode_raw_key(raw: &[u8]) -> Result<[u8; MASTER_KEY_LEN]> {
    let key: [u8; MASTER_KEY_LEN] = raw.try_into().map_err(|_| {
        AppError::internal(format!(
            "decrypted master.key payload has wrong length: \
             expected {MASTER_KEY_LEN}, found {}",
            raw.len()
        ))
    })?;
    Ok(key)
}

/// Serialize a DPAPI blob into the self-describing file format.
fn encode_dpapi_key_file(blob: &[u8]) -> String {
    format!("{DPAPI_KEY_PREFIX}{}", B64.encode(blob))
}

const MAX_READ_ATTEMPTS: u32 = 10;
const RETRY_DELAY_MS: u64 = 10;

/// Load or create the 32-byte master key under `app_data_dir`.
///
/// If the key file exists but cannot be read, is not exactly 32 legacy bytes,
/// and is not a valid DPAPI record, this returns an error rather than
/// overwriting it. Silently regenerating would permanently orphan every
/// already-encrypted API key, so a corrupt/locked key file must surface as a
/// failure the user can act on (e.g. restore a backup).
pub fn get_master_key(app_data_dir: &Path) -> Result<[u8; 32]> {
    let path = key_path(app_data_dir);
    match std::fs::read(&path) {
        Ok(bytes) => parse_and_migrate_key(&path, &bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => generate_master_key(&path),
        Err(e) => Err(AppError::internal(format!(
            "failed to read master.key: {e}"
        ))),
    }
}

/// Parse stored key bytes and migrate legacy unprotected keys if encountered.
fn parse_and_migrate_key(path: &Path, bytes: &[u8]) -> Result<[u8; 32]> {
    match parse_stored_key_file(bytes)? {
        StoredKeyFile::LegacyRaw(key) => {
            tracing::info!(
                "master.key uses the legacy unprotected format; \
                 migrating to the DPAPI-wrapped format"
            );
            write_key_file(path, &key)?;
            Ok(key)
        }
        StoredKeyFile::DpapiWrapped(blob) => load_protected_key(&blob),
    }
}

/// Read existing master key file from disk.
fn read_existing_key(path: &Path) -> Result<[u8; 32]> {
    let bytes = std::fs::read(path)
        .map_err(|e| AppError::internal(format!("failed to read master.key: {e}")))?;
    parse_and_migrate_key(path, &bytes)
}

/// Retry reading key file with backoff when losing a creation race.
fn read_existing_key_with_retry(path: &Path) -> Result<[u8; 32]> {
    let mut last_err = None;
    for _ in 0..MAX_READ_ATTEMPTS {
        match read_existing_key(path) {
            Ok(key) => return Ok(key),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| {
        AppError::internal("failed to read master.key after concurrent creation")
    }))
}

/// Generate, persist exclusively, and return a fresh 32-byte master key at `path`.
fn generate_master_key(path: &Path) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match create_exclusive_key_file(path, &key) {
        Ok(()) => Ok(key),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another instance won the race to create the key file.
            read_existing_key_with_retry(path)
        }
        Err(e) => Err(AppError::internal(format!(
            "failed to create master.key: {e}"
        ))),
    }
}

/// Serialize master key for disk storage based on target platform.
fn serialize_master_key(key: &[u8; 32]) -> Result<Vec<u8>> {
    #[cfg(windows)]
    {
        let protected = dpapi::protect(key)?;
        Ok(encode_dpapi_key_file(&protected).into_bytes())
    }
    #[cfg(not(windows))]
    {
        Ok(key.to_vec())
    }
}

/// Exclusively create and write key file, failing if it already exists.
fn create_exclusive_key_file(
    path: &Path,
    key: &[u8; 32],
) -> std::result::Result<(), std::io::Error> {
    use std::io::Write;
    let payload = serialize_master_key(key).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    if let Err(e) = file.write_all(&payload).and_then(|_| file.sync_all()) {
        let _ = std::fs::remove_file(path);
        return Err(e);
    }
    Ok(())
}

/// Unwrap a DPAPI blob into the raw 32-byte master key.
///
/// Only reachable in practice on Windows (where DPAPI-wrapped files are
/// written); elsewhere it fails fast instead of guessing.
fn load_protected_key(blob: &[u8]) -> Result<[u8; MASTER_KEY_LEN]> {
    #[cfg(windows)]
    {
        let raw = dpapi::unprotect(blob)?;
        decode_raw_key(&raw)
    }
    #[cfg(not(windows))]
    {
        let _ = blob;
        Err(AppError::internal(
            "master.key uses the Windows DPAPI format, which is \
             unavailable on this platform",
        ))
    }
}

#[cfg(unix)]
fn write_key_file(path: &Path, key: &[u8; 32]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let payload = serialize_master_key(key)?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&payload)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_key_file(path: &Path, key: &[u8; 32]) -> Result<()> {
    let payload = serialize_master_key(key)?;
    std::fs::write(path, payload)?;
    Ok(())
}

/// Thin DPAPI wrapper (Windows Data Protection API, CurrentUser scope).
///
/// Contains only the FFI calls and blob plumbing; all file-format and
/// parsing logic lives above so it remains testable without real DPAPI.
#[cfg(windows)]
mod dpapi {
    use std::ptr::null_mut;

    use windows::core::{Result as WinResult, PCWSTR};
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    use crate::error::{AppError, Result};

    /// Sanity cap on accepted DPAPI output so a bogus `cbData` can never
    /// drive a huge allocation.
    const MAX_BLOB_BYTES: u32 = 1024 * 1024;

    pub(super) fn protect(plaintext: &[u8]) -> Result<Vec<u8>> {
        run_dpapi(
            |input, output| {
                // SAFETY: both blobs point to live, correctly sized memory
                // for the duration of the call.
                unsafe {
                    CryptProtectData(
                        input,
                        PCWSTR::null(),
                        None,
                        None,
                        None,
                        CRYPTPROTECT_UI_FORBIDDEN,
                        output,
                    )
                }
            },
            plaintext,
            "protect",
        )
    }

    pub(super) fn unprotect(blob: &[u8]) -> Result<Vec<u8>> {
        run_dpapi(
            |input, output| {
                // SAFETY: both blobs point to live, correctly sized memory
                // for the duration of the call.
                unsafe {
                    CryptUnprotectData(
                        input,
                        None,
                        None,
                        None,
                        None,
                        CRYPTPROTECT_UI_FORBIDDEN,
                        output,
                    )
                }
            },
            blob,
            "unprotect",
        )
    }

    /// Shared shape of `CryptProtectData` / `CryptUnprotectData`.
    ///
    /// Both projections are `unsafe fn`s returning `windows::core::Result<()>`;
    /// the callers above own the `unsafe` blocks.
    fn run_dpapi(
        f: impl Fn(*const CRYPT_INTEGER_BLOB, *mut CRYPT_INTEGER_BLOB) -> WinResult<()>,
        input: &[u8],
        op: &'static str,
    ) -> Result<Vec<u8>> {
        let data_in = CRYPT_INTEGER_BLOB {
            cbData: input.len() as u32,
            pbData: input.as_ptr() as *mut u8,
        };
        let mut data_out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: null_mut(),
        };
        f(&data_in, &mut data_out)
            .map_err(|e| AppError::internal(format!("DPAPI {op} failed: {e}")))?;
        take_blob_output(data_out, op)
    }

    /// Copy the DPAPI-owned output buffer into an owned `Vec`, freeing the
    /// original via `LocalFree` (RAII guard covers early-return paths).
    fn take_blob_output(blob: CRYPT_INTEGER_BLOB, op: &'static str) -> Result<Vec<u8>> {
        struct LocalAllocated(HLOCAL);
        impl Drop for LocalAllocated {
            fn drop(&mut self) {
                // SAFETY: the handle wraps the pointer DPAPI allocated with
                // LocalAlloc; freeing it exactly once here is required.
                let _ = unsafe { LocalFree(self.0) };
            }
        }

        if blob.pbData.is_null() {
            return Err(AppError::internal(format!(
                "DPAPI {op} returned a null buffer"
            )));
        }
        if blob.cbData > MAX_BLOB_BYTES {
            return Err(AppError::internal(format!(
                "DPAPI {op} output of {} bytes exceeds the \
                 {MAX_BLOB_BYTES}-byte sanity limit",
                blob.cbData
            )));
        }
        // SAFETY: non-null pointer with `cbData` valid bytes per DPAPI contract.
        let bytes =
            unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec() };
        drop(LocalAllocated(HLOCAL(blob.pbData.cast())));
        Ok(bytes)
    }
}

fn key_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("master.key")
}

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

/// Decrypt a value produced by [`encrypt_api_key`].
///
/// Legacy plaintext values (no prefix) are still returned unchanged as a
/// defensive fallback so a failed migration never bricks an existing install,
/// but every hit is logged loudly: post-migration this should never happen.
pub fn decrypt_api_key(stored: &str, master_key: &[u8; 32]) -> Result<String> {
    if stored.is_empty() {
        return Ok(String::new());
    }
    let Some(b64) = stored.strip_prefix(ENC_PREFIX) else {
        tracing::warn!(
            "decrypt_api_key got a value without '{ENC_PREFIX}' prefix; \
             treating as legacy plaintext — migrate_legacy_api_key should have re-encrypted it"
        );
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

/// One-shot upgrade of a legacy plaintext API key to the encrypted envelope.
///
/// Returns `Ok(Some(encrypted))` when `stored` is a non-empty plaintext value,
/// `Ok(None)` when nothing needs doing (empty, or already `enc:v1:` prefixed),
/// or an error when re-encryption fails. Pure — persistence is the caller's
/// job so this stays trivially testable and idempotent.
pub fn migrate_plaintext_key(stored: &str, master_key: &[u8; 32]) -> Result<Option<String>> {
    if stored.is_empty() || is_encrypted(stored) {
        return Ok(None);
    }
    encrypt_api_key(stored, master_key).map(Some)
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
    fn migration_encrypts_plaintext() {
        let key = [9u8; 32];
        let migrated = migrate_plaintext_key("gsk_legacy_plain", &key)
            .unwrap()
            .expect("plaintext must migrate");
        assert!(is_encrypted(&migrated));
        assert_ne!(migrated, "gsk_legacy_plain");
    }

    #[test]
    fn migration_roundtrips_to_original_plaintext() {
        let key = [10u8; 32];
        let original = "gsk_roundtrip_ключ 🔑";
        let migrated = migrate_plaintext_key(original, &key)
            .unwrap()
            .expect("plaintext must migrate");
        assert_eq!(decrypt_api_key(&migrated, &key).unwrap(), original);
    }

    #[test]
    fn migration_skips_already_encrypted() {
        let key = [11u8; 32];
        let enc = encrypt_api_key("already_done", &key).unwrap();
        assert_eq!(
            migrate_plaintext_key(&enc, &key).unwrap(),
            None,
            "idempotent: encrypted values must not be re-encrypted"
        );
    }

    #[test]
    fn migration_skips_empty() {
        let key = [12u8; 32];
        assert_eq!(migrate_plaintext_key("", &key).unwrap(), None);
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
    fn concurrent_create_reads_existing_key_on_conflict() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_race_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = key_path(&dir);
        let first = get_master_key(&dir).unwrap();
        // Direct generation on existing file must re-read rather than recreate.
        let second = generate_master_key(&path).unwrap();
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

    // --- master.key file-format logic (pure, platform-independent) ---

    #[test]
    fn parses_legacy_raw_key_file() {
        assert_eq!(
            parse_stored_key_file(&[7u8; MASTER_KEY_LEN]).unwrap(),
            StoredKeyFile::LegacyRaw([7u8; MASTER_KEY_LEN])
        );
    }

    #[test]
    fn parses_dpapi_wrapped_key_file() {
        let blob = vec![1u8, 2, 3, 4];
        let text = encode_dpapi_key_file(&blob);
        assert!(
            text.starts_with(DPAPI_KEY_PREFIX),
            "encoded file must carry the marker prefix"
        );
        assert_eq!(
            parse_stored_key_file(text.as_bytes()).unwrap(),
            StoredKeyFile::DpapiWrapped(blob)
        );
    }

    #[test]
    fn encoded_file_roundtrips_through_parser() {
        let blob: Vec<u8> = (0..48).collect();
        let parsed = parse_stored_key_file(encode_dpapi_key_file(&blob).as_bytes()).unwrap();
        assert_eq!(parsed, StoredKeyFile::DpapiWrapped(blob));
    }

    #[test]
    fn parser_tolerates_trailing_whitespace() {
        let blob = [9u8; 8];
        let text = format!("{}\n", encode_dpapi_key_file(&blob));
        assert_eq!(
            parse_stored_key_file(text.as_bytes()).unwrap(),
            StoredKeyFile::DpapiWrapped(blob.to_vec())
        );
    }

    #[test]
    fn rejects_raw_file_of_wrong_length() {
        assert!(parse_stored_key_file(&[0u8; 16]).is_err());
        assert!(parse_stored_key_file(&[0u8; MASTER_KEY_LEN + 1]).is_err());
        assert!(parse_stored_key_file(&[]).is_err());
    }

    #[test]
    fn rejects_unrecognized_content() {
        assert!(parse_stored_key_file(b"not-a-key").is_err());
        assert!(parse_stored_key_file(b"\xff\xfe\x00garbage").is_err());
    }

    #[test]
    fn rejects_corrupt_base64_payload() {
        let text = format!("{DPAPI_KEY_PREFIX}!!!not-base64!!!");
        assert!(parse_stored_key_file(text.as_bytes()).is_err());
    }

    #[test]
    fn decode_raw_key_enforces_length() {
        assert!(decode_raw_key(&[0u8; MASTER_KEY_LEN - 1]).is_err());
        assert!(decode_raw_key(&[0u8; MASTER_KEY_LEN + 1]).is_err());
        assert_eq!(
            decode_raw_key(&[5u8; MASTER_KEY_LEN]).unwrap(),
            [5u8; MASTER_KEY_LEN]
        );
    }
}
