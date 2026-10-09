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

/// Whether the legacy raw (unwrapped) master.key format is accepted for a directory.
///
/// The raw format exists only for pre-DPAPI installs, whose key lived in the
/// default per-user app-data directory. A custom directory may be writable by
/// another local user, so a planted raw key there would silently become the key
/// every API key is later encrypted under (M-4). Only the default directory may
/// hold a raw key; a custom directory fails closed instead.
///
/// Non-Windows platforms never wrap the key, so raw is the only format and is
/// always accepted there.
pub fn legacy_raw_allowed(app_data_dir: &Path, default_app_data_dir: &Path) -> bool {
    #[cfg(windows)]
    {
        app_data_dir == default_app_data_dir
    }
    #[cfg(not(windows))]
    {
        let _ = (app_data_dir, default_app_data_dir);
        true
    }
}

/// Load or create the 32-byte master key under `app_data_dir`.
///
/// If the key file exists but cannot be read, is not exactly 32 legacy bytes,
/// and is not a valid DPAPI record, this returns an error rather than
/// overwriting it. Silently regenerating would permanently orphan every
/// already-encrypted API key, so a corrupt/locked key file must surface as a
/// failure the user can act on (e.g. restore a backup).
///
/// `default_app_data_dir` is compared against `app_data_dir` to decide whether
/// the legacy raw format is acceptable here (see [`legacy_raw_allowed`]).
pub fn get_master_key(app_data_dir: &Path, default_app_data_dir: &Path) -> Result<[u8; 32]> {
    let path = key_path(app_data_dir);
    let allow_legacy_raw = legacy_raw_allowed(app_data_dir, default_app_data_dir);
    match std::fs::read(&path) {
        Ok(bytes) => parse_and_migrate_key(&path, &bytes, allow_legacy_raw),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            generate_master_key(&path, allow_legacy_raw)
        }
        Err(e) => Err(AppError::internal(format!(
            "failed to read master.key: {e}"
        ))),
    }
}

/// Parse stored key bytes and migrate legacy unprotected keys if encountered.
///
/// A legacy raw key is migrated to the wrapped format only when the directory
/// allows it; otherwise it is rejected fail-closed, and never silently
/// regenerated (which would orphan every already-encrypted API key).
fn parse_and_migrate_key(path: &Path, bytes: &[u8], allow_legacy_raw: bool) -> Result<[u8; 32]> {
    match parse_stored_key_file(bytes)? {
        StoredKeyFile::LegacyRaw(key) => {
            if !allow_legacy_raw {
                return Err(legacy_raw_in_custom_dir_error());
            }
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

/// Error for a raw master.key found in a custom data directory.
///
/// Instructs the user to re-enter the API key rather than offering a re-wrap:
/// re-wrapping a planted key would trust attacker-supplied key material.
fn legacy_raw_in_custom_dir_error() -> AppError {
    AppError::api_key_decrypt_failed(
        "master.key in the custom data directory uses an untrusted legacy format; \
         re-enter the API key in Settings",
    )
}

/// Rewrap a legacy raw master.key at `path` with DPAPI, leaving other formats alone.
///
/// Called only by data-directory migration when copying a key from a trusted
/// source into a custom directory, so a legitimate pre-DPAPI install keeps
/// working after the move. A raw key found later at load time is still rejected
/// (see [`legacy_raw_allowed`]); this does not run on the load path.
pub fn rewrap_legacy_key_if_raw(path: &Path) -> Result<()> {
    let bytes = std::fs::read(path)?;
    if let Ok(StoredKeyFile::LegacyRaw(key)) = parse_stored_key_file(&bytes) {
        write_key_file(path, &key)?;
    }
    Ok(())
}

/// Read existing master key file from disk.
fn read_existing_key(path: &Path, allow_legacy_raw: bool) -> Result<[u8; 32]> {
    let bytes = std::fs::read(path)
        .map_err(|e| AppError::internal(format!("failed to read master.key: {e}")))?;
    parse_and_migrate_key(path, &bytes, allow_legacy_raw)
}

/// Retry reading key file with backoff when losing a creation race.
fn read_existing_key_with_retry(path: &Path, allow_legacy_raw: bool) -> Result<[u8; 32]> {
    let mut last_err = None;
    for _ in 0..MAX_READ_ATTEMPTS {
        match read_existing_key(path, allow_legacy_raw) {
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
fn generate_master_key(path: &Path, allow_legacy_raw: bool) -> Result<[u8; 32]> {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match create_exclusive_key_file(path, &key) {
        Ok(()) => Ok(key),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another instance won the race to create the key file.
            read_existing_key_with_retry(path, allow_legacy_raw)
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

/// Rewrite `master.key` atomically so a crash mid-write cannot truncate it and
/// orphan every encrypted API key. Uses the shared temp-file + rename helper.
fn write_key_file(path: &Path, key: &[u8; 32]) -> Result<()> {
    let payload = serialize_master_key(key)?;
    crate::util::write_atomic(path, &payload)?;
    restrict_key_file_permissions(path)
}

/// Keep `master.key` readable only by its owner on Unix. On Windows the payload
/// is DPAPI-wrapped, so file permissions add nothing.
#[cfg(unix)]
fn restrict_key_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_key_file_permissions(_path: &Path) -> Result<()> {
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
/// Plaintext values without the [`ENC_PREFIX`] are rejected — migration
/// at startup ensures all stored keys are encrypted at rest.
pub fn decrypt_api_key(stored: &str, master_key: &[u8; 32]) -> Result<String> {
    if stored.is_empty() {
        return Ok(String::new());
    }
    let Some(b64) = stored.strip_prefix(ENC_PREFIX) else {
        return Err(AppError::internal(
            "unencrypted API key rejected: expected encrypted 'enc:v1:' prefix",
        ));
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

/// Seal a short trusted string (for example a picker-issued binary path)
/// under the master key, reusing the API-key envelope format.
pub fn seal_token(plaintext: &str, master_key: &[u8; 32]) -> Result<String> {
    encrypt_api_key(plaintext, master_key)
}

/// Open a token produced by [`seal_token`].
///
/// Fails on tampering, a wrong master key, or a value that is not a sealed
/// token, so a caller can treat any error as "the token does not vouch for
/// this value".
pub fn open_token(token: &str, master_key: &[u8; 32]) -> Result<String> {
    decrypt_api_key(token, master_key)
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
    fn unencrypted_key_rejected() {
        let key = [1u8; 32];
        assert!(decrypt_api_key("plainkey", &key).is_err());
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
    fn token_seal_roundtrips() {
        let key = [13u8; 32];
        let path = r"C:\Program Files\whisper\whisper-cli.exe";
        let token = seal_token(path, &key).unwrap();
        assert!(is_encrypted(&token));
        assert_eq!(open_token(&token, &key).unwrap(), path);
    }

    #[test]
    fn token_open_fails_with_wrong_key() {
        let token = seal_token("C:\\whisper.exe", &[14u8; 32]).unwrap();
        assert!(open_token(&token, &[15u8; 32]).is_err());
    }

    #[test]
    fn generates_then_reloads_same_key() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_ok_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let first = get_master_key(&dir, &dir).unwrap();
        // Second call must load the persisted key, not regenerate a new one.
        let second = get_master_key(&dir, &dir).unwrap();
        assert_eq!(first, second);
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test]
    fn concurrent_create_reads_existing_key_on_conflict() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_race_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = key_path(&dir);
        let first = get_master_key(&dir, &dir).unwrap();
        // Direct generation on existing file must re-read rather than recreate.
        let second = generate_master_key(&path, true).unwrap();
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
        assert!(get_master_key(&dir, &dir).is_err());
        // The corrupt file is left intact for recovery.
        assert_eq!(std::fs::read(key_path(&dir)).unwrap().len(), 16);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_raw_allowed_only_in_default_dir() {
        let default_dir = std::env::temp_dir().join("voxitype-default");
        let custom_dir = std::env::temp_dir().join("voxitype-custom");
        #[cfg(windows)]
        {
            assert!(legacy_raw_allowed(&default_dir, &default_dir));
            assert!(!legacy_raw_allowed(&custom_dir, &default_dir));
        }
        #[cfg(not(windows))]
        {
            assert!(legacy_raw_allowed(&custom_dir, &default_dir));
        }
    }

    /// A raw key in a custom dir must fail closed; a wrapped key works anywhere.
    #[cfg(windows)]
    #[test]
    fn raw_key_rejected_in_custom_dir_but_wrapped_accepted() {
        let default_dir = std::env::temp_dir().join(format!("voxitype_def_{}", std::process::id()));
        let custom_dir = std::env::temp_dir().join(format!("voxitype_cus_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&default_dir);
        let _ = std::fs::remove_dir_all(&custom_dir);
        std::fs::create_dir_all(&custom_dir).unwrap();

        // Plant a raw key in the custom dir: rejected, and not overwritten.
        std::fs::write(key_path(&custom_dir), [42u8; MASTER_KEY_LEN]).unwrap();
        assert!(get_master_key(&custom_dir, &default_dir).is_err());
        assert_eq!(
            std::fs::read(key_path(&custom_dir)).unwrap(),
            [42u8; MASTER_KEY_LEN]
        );

        // A wrapped key (written by rewrap) loads fine in the same custom dir.
        rewrap_legacy_key_if_raw(&key_path(&custom_dir)).unwrap();
        assert_eq!(
            get_master_key(&custom_dir, &default_dir).unwrap(),
            [42u8; MASTER_KEY_LEN]
        );
        let _ = std::fs::remove_dir_all(&custom_dir);
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

    #[test]
    fn write_key_file_writes_content_without_leaving_temp_files() {
        let dir = std::env::temp_dir().join(format!("voxitype_key_atomic_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = key_path(&dir);
        // Seed an existing key so the rewrite must replace it atomically.
        write_key_file(&path, &[1u8; MASTER_KEY_LEN]).unwrap();
        write_key_file(&path, &[2u8; MASTER_KEY_LEN]).unwrap();

        // The DPAPI blob is non-deterministic, so compare the unwrapped key.
        match parse_stored_key_file(&std::fs::read(&path).unwrap()).unwrap() {
            StoredKeyFile::LegacyRaw(key) => assert_eq!(key, [2u8; MASTER_KEY_LEN]),
            StoredKeyFile::DpapiWrapped(blob) => {
                assert_eq!(load_protected_key(&blob).unwrap(), [2u8; MASTER_KEY_LEN])
            }
        }
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "atomic write must not leave temp files"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
