//! ACL validation for a custom data directory (H-2).
//!
//! A custom data directory is only as private as its DACL. If another local
//! user can write to it, they can plant a database or `master.key` the app
//! later trusts (see H-2/M-4). `validate_custom_dir_acl` rejects a directory
//! whose effective DACL grants write access to anyone other than the current
//! user, SYSTEM, Administrators, CREATOR OWNER, or OWNER RIGHTS.
//!
//! Only custom directories are checked. The default per-user app-data
//! directory keeps its current behaviour so a misconfigured profile ACL can
//! never brick startup.

use std::path::Path;

use crate::error::Result;

/// Verify the resolved directory grants write access only to trusted principals.
///
/// No-op on non-Windows platforms (the custom-directory feature is Windows-only
/// in practice; the base checks still run elsewhere).
pub(crate) fn validate_custom_dir_acl(path: &Path) -> Result<()> {
    platform::validate(path)
}

/// Rewrite the directory DACL to grant only the current user, SYSTEM, and
/// Administrators, then re-validate.
///
/// A fresh folder on any non-system drive inherits `Authenticated Users:
/// Modify` and `Users: Read` from the volume root, so a user who picks a
/// folder there is rejected with no way to proceed. Restricting is always
/// tightening: the result is strictly less access than before.
///
/// Recursive on purpose. `(OI)(CI)` never applies to an existing file, and
/// `icacls /inheritance:r` strips the inherited ACEs first, so a non-recursive
/// run would leave every existing file with an empty DACL. That is not
/// theoretical: it is the state that made a valid database report as invalid.
pub(crate) fn restrict_to_current_user(path: &Path) -> Result<()> {
    platform::restrict(path)?;
    platform::validate(path)
}

/// The current user's SID rendered as a string (for example `S-1-5-21-...`).
///
/// Used by tests to pin a scratch directory to the current user. Windows only.
#[cfg(all(windows, test))]
pub(crate) fn current_user_sid_string() -> Result<String> {
    platform::current_user_sid_string()
}

#[cfg(not(windows))]
mod platform {
    use std::path::Path;

    use crate::error::Result;

    pub(super) fn validate(_path: &Path) -> Result<()> {
        Ok(())
    }

    /// No-op: the ACL model this repairs is Windows-specific.
    pub(super) fn restrict(_path: &Path) -> Result<()> {
        Ok(())
    }
}

#[cfg(windows)]
mod platform {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, LocalFree, ERROR_SUCCESS, HANDLE, HLOCAL};
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    #[cfg(test)]
    use windows::Win32::Security::INHERIT_ONLY_ACE;
    use windows::Win32::Security::{
        CreateWellKnownSid, EqualSid, GetAce, GetLengthSid, GetTokenInformation, LookupAccountSidW,
        TokenUser, WinBuiltinAdministratorsSid, WinCreatorOwnerRightsSid, WinCreatorOwnerSid,
        WinLocalSystemSid, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, DACL_SECURITY_INFORMATION,
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE,
        SID_NAME_USE, TOKEN_QUERY, TOKEN_USER, WELL_KNOWN_SID_TYPE,
    };
    use windows::Win32::Storage::FileSystem::{
        DELETE, FILE_DELETE_CHILD, FILE_GENERIC_WRITE, WRITE_DAC, WRITE_OWNER,
    };
    #[cfg(test)]
    use windows::Win32::Storage::FileSystem::{FILE_READ_ATTRIBUTES, FILE_READ_DATA, FILE_READ_EA};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    use crate::error::{AppError, Result};

    /// ACE type tags from the Windows headers (plain numbers, kept local to
    /// avoid pulling an extra crate feature).
    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
    /// Same mask and trustee layout as a plain allowed ACE; callback data
    /// follows the SID.
    const ACCESS_ALLOWED_CALLBACK_ACE_TYPE: u8 = 9;
    /// Object ACE types. The trustee SID follows a variable-length header, so a
    /// write grant of this type cannot be attributed to a SID here.
    const ACCESS_ALLOWED_OBJECT_ACE_TYPE: u8 = 5;
    const ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE: u8 = 11;

    /// Access rights that let a principal modify or replace the directory
    /// contents, take ownership, or rewrite the DACL itself.
    ///
    /// `FILE_GENERIC_WRITE` also carries `READ_CONTROL` and `SYNCHRONIZE`, so
    /// real-world read grants (for example `icacls (R)`) overlap this mask and
    /// are rejected too. That is intentional: the database and the wrapped key
    /// file must not be readable by other local accounts either.
    const WRITE_RIGHTS: u32 =
        FILE_GENERIC_WRITE.0 | DELETE.0 | WRITE_DAC.0 | WRITE_OWNER.0 | FILE_DELETE_CHILD.0;

    /// Byte offset of the trustee SID inside an `ACCESS_ALLOWED_ACE`.
    const ACE_SID_OFFSET: usize = std::mem::offset_of!(ACCESS_ALLOWED_ACE, SidStart);

    pub(super) fn validate(path: &Path) -> Result<()> {
        let descriptor = read_security_descriptor(path)?;
        let current_sid = current_user_sid()?;
        let allowed = allowed_sids(&current_sid);

        if !is_allowed_sid(descriptor.owner, &allowed) {
            return Err(acl_violation(&format!(
                "owner '{}'",
                describe_sid(descriptor.owner)
            )));
        }
        check_dacl(descriptor.dacl, &allowed)
    }

    /// A security descriptor and the owner/DACL pointers it owns. The pointers
    /// stay valid until the descriptor is freed on drop.
    struct Descriptor {
        owner: PSID,
        dacl: *mut ACL,
        raw: PSECURITY_DESCRIPTOR,
    }

    impl Drop for Descriptor {
        fn drop(&mut self) {
            if !self.raw.0.is_null() {
                // SAFETY: the descriptor was allocated by GetNamedSecurityInfoW
                // with LocalAlloc and is freed exactly once here.
                unsafe {
                    let _ = LocalFree(HLOCAL(self.raw.0));
                }
            }
        }
    }

    fn read_security_descriptor(path: &Path) -> Result<Descriptor> {
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.push(0);
        let mut owner = PSID::default();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut raw = PSECURITY_DESCRIPTOR::default();
        // SAFETY: `wide` is a live NUL-terminated path; the out-pointers are
        // valid for the duration of the call.
        let status = unsafe {
            GetNamedSecurityInfoW(
                PCWSTR(wide.as_ptr()),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                Some(&mut owner),
                None,
                Some(&mut dacl),
                None,
                &mut raw,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(AppError::data_directory(format!(
                "failed to read data directory security descriptor (error {})",
                status.0
            )));
        }
        Ok(Descriptor { owner, dacl, raw })
    }

    /// The current user's SID rendered as a string.
    pub(super) fn current_user_sid_string() -> Result<String> {
        let sid = current_user_sid()?;
        sid_to_string(PSID(sid.as_ptr().cast_mut().cast()))
            .ok_or_else(|| acl_error("failed to render the current user SID"))
    }

    /// Rewrite the DACL with `icacls`, naming trustees by SID so the command
    /// works on any display language.
    ///
    /// Two passes, both required:
    ///
    /// 1. `/reset /T` drops every explicit ACE and re-inherits from the parent.
    ///    `/grant:r` alone replaces only the trustees it names, so an explicit
    ///    grant someone added earlier (for example `icacls /grant Users:(M)`)
    ///    would survive and keep the directory rejected.
    /// 2. `/inheritance:r` plus explicit grants, recursively. Each trustee needs
    ///    two entries: `(OI)(CI)` for the directory and a bare `F` for the files
    ///    already inside it. Without the second, `/T` leaves every existing file
    ///    with an empty DACL and the user cannot read their own data.
    pub(super) fn restrict(path: &Path) -> Result<()> {
        let user_sid = current_user_sid_string()?;
        run_icacls(path, &["/reset", "/T"])?;

        let trustees = [user_sid.as_str(), "S-1-5-18", "S-1-5-32-544"];
        let mut args: Vec<String> = vec!["/inheritance:r".into(), "/grant:r".into()];
        args.extend(trustees.iter().map(|sid| format!("*{sid}:(OI)(CI)F")));
        args.extend(trustees.iter().map(|sid| format!("*{sid}:F")));
        args.push("/T".into());
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        run_icacls(path, &borrowed)
    }

    fn run_icacls(path: &Path, args: &[&str]) -> Result<()> {
        let output = std::process::Command::new("icacls")
            .arg(path)
            .args(args)
            .output()
            .map_err(|error| AppError::data_directory(format!("failed to run icacls: {error}")))?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(AppError::data_directory(format!(
            "failed to restrict the data directory permissions: {}",
            stderr.trim()
        )))
    }

    /// The current process user's SID as owned bytes.
    fn current_user_sid() -> Result<Vec<u8>> {
        let mut token = HANDLE::default();
        // SAFETY: GetCurrentProcess returns a pseudo-handle; the token handle
        // is closed by the guard on every return path.
        unsafe {
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
                .map_err(|e| acl_error(&format!("failed to open process token: {e}")))?;
        }
        let _guard = TokenGuard(token);
        read_token_user_sid(token)
    }

    fn read_token_user_sid(token: HANDLE) -> Result<Vec<u8>> {
        let mut needed = 0u32;
        // First call sizes the buffer; an error here is expected and ignored.
        let _ = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut needed) };
        if needed == 0 {
            return Err(acl_error("failed to size the process user token"));
        }
        let mut buffer = vec![0u8; needed as usize];
        // SAFETY: `buffer` is at least `needed` bytes, as reported above.
        unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(buffer.as_mut_ptr() as *mut c_void),
                needed,
                &mut needed,
            )
            .map_err(|e| acl_error(&format!("failed to read the process user token: {e}")))?;
            let user = &*(buffer.as_ptr() as *const TOKEN_USER);
            let sid = user.User.Sid;
            let len = GetLengthSid(sid) as usize;
            Ok(std::slice::from_raw_parts(sid.0 as *const u8, len).to_vec())
        }
    }

    /// The current user's SID plus the well-known SIDs always trusted to own
    /// application data.
    fn allowed_sids(current: &[u8]) -> Vec<Vec<u8>> {
        let mut sids = vec![current.to_vec()];
        let well_known = [
            WinLocalSystemSid,
            WinBuiltinAdministratorsSid,
            WinCreatorOwnerSid,
            WinCreatorOwnerRightsSid,
        ];
        sids.extend(well_known.into_iter().filter_map(well_known_sid));
        sids
    }

    fn well_known_sid(kind: WELL_KNOWN_SID_TYPE) -> Option<Vec<u8>> {
        let mut buffer = [0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut len = SECURITY_MAX_SID_SIZE;
        // SAFETY: `buffer` is `SECURITY_MAX_SID_SIZE` bytes, the documented
        // maximum for any SID CreateWellKnownSid can produce.
        unsafe {
            CreateWellKnownSid(
                kind,
                PSID::default(),
                PSID(buffer.as_mut_ptr().cast()),
                &mut len,
            )
            .ok()?;
        }
        Some(buffer[..len as usize].to_vec())
    }

    fn check_dacl(dacl: *mut ACL, allowed: &[Vec<u8>]) -> Result<()> {
        if dacl.is_null() {
            return Err(acl_violation(
                "has a null access control list (grants everyone full access)",
            ));
        }
        let count = unsafe { (*dacl).AceCount };
        for index in 0..u32::from(count) {
            let mut ace: *mut c_void = std::ptr::null_mut();
            // SAFETY: `index` is within `AceCount`; GetAce yields a pointer
            // into the DACL that stays valid while the descriptor is alive.
            if unsafe { GetAce(dacl, index, &mut ace) }.is_err() {
                return Err(acl_error(
                    "failed to read a data directory access control entry",
                ));
            }
            if let Some(trustee) = write_trustee(ace) {
                if !is_allowed_sid(trustee, allowed) {
                    return Err(acl_violation(&format!(
                        "grants write or control access to '{}'",
                        describe_sid(trustee)
                    )));
                }
            } else if is_unattributable_write_ace(ace) {
                return Err(acl_violation(
                    "contains a write grant this check cannot attribute to an account",
                ));
            }
        }
        Ok(())
    }

    /// Return the trustee SID of a write-granting allow ACE, or `None` for
    /// other ACE types or ACEs without write rights.
    ///
    /// Inherit-only ACEs are checked too. They skip the directory itself but
    /// apply to the database and key files created inside it, which is exactly
    /// what this check protects.
    fn write_trustee(ace: *mut c_void) -> Option<PSID> {
        // SAFETY: `ace` points to a full ACE header, valid while the DACL lives.
        let header = unsafe { &*(ace as *const ACE_HEADER) };
        if header.AceType != ACCESS_ALLOWED_ACE_TYPE
            && header.AceType != ACCESS_ALLOWED_CALLBACK_ACE_TYPE
        {
            return None;
        }
        // SAFETY: allowed and callback ACEs share the mask layout.
        let mask = unsafe { (*(ace as *const ACCESS_ALLOWED_ACE)).Mask };
        if mask & WRITE_RIGHTS == 0 {
            return None;
        }
        // SAFETY: the trustee SID follows the fixed ACE header fields.
        Some(PSID(unsafe {
            (ace as *const u8).add(ACE_SID_OFFSET) as *mut c_void
        }))
    }

    /// Whether an ACE grants write rights but its trustee cannot be resolved
    /// here (object ACEs carry GUID fields before the SID). Fail closed: an
    /// unattributable write grant is treated as a violation.
    fn is_unattributable_write_ace(ace: *mut c_void) -> bool {
        // SAFETY: `ace` points to a full ACE header, valid while the DACL lives.
        let header = unsafe { &*(ace as *const ACE_HEADER) };
        if header.AceType != ACCESS_ALLOWED_OBJECT_ACE_TYPE
            && header.AceType != ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE
        {
            return false;
        }
        // SAFETY: the access mask sits right after the ACE header in every
        // ACCESS_ALLOWED_* layout.
        let mask = unsafe { (*(ace as *const ACCESS_ALLOWED_ACE)).Mask };
        mask & WRITE_RIGHTS != 0
    }

    fn is_allowed_sid(sid: PSID, allowed: &[Vec<u8>]) -> bool {
        allowed.iter().any(|candidate| {
            // SAFETY: both SIDs are valid for the duration of the comparison.
            unsafe { EqualSid(sid, PSID(candidate.as_ptr().cast_mut().cast())).is_ok() }
        })
    }

    /// Resolve a trustee SID to a human-readable account name, falling back to
    /// its string form and then a placeholder.
    fn describe_sid(sid: PSID) -> String {
        lookup_account_name(sid)
            .or_else(|| sid_to_string(sid))
            .unwrap_or_else(|| "unknown account".to_string())
    }

    fn lookup_account_name(sid: PSID) -> Option<String> {
        let mut name = [0u16; MAX_ACCOUNT_CHARS];
        let mut domain = [0u16; MAX_ACCOUNT_CHARS];
        let mut name_len = name.len() as u32;
        let mut domain_len = domain.len() as u32;
        let mut usage = SID_NAME_USE::default();
        // SAFETY: both buffers are live and their lengths are passed correctly.
        unsafe {
            LookupAccountSidW(
                PCWSTR::null(),
                sid,
                PWSTR(name.as_mut_ptr()),
                &mut name_len,
                PWSTR(domain.as_mut_ptr()),
                &mut domain_len,
                &mut usage,
            )
            .ok()?;
        }
        let account = String::from_utf16_lossy(&name[..name_len as usize]);
        (!account.is_empty()).then_some(account)
    }

    fn sid_to_string(sid: PSID) -> Option<String> {
        let mut text = PWSTR::null();
        // SAFETY: the out-pointer receives a LocalAlloc'd string, freed below.
        unsafe {
            ConvertSidToStringSidW(sid, &mut text).ok()?;
        }
        let rendered = unsafe { text.to_string().ok() };
        // SAFETY: ConvertSidToStringSidW allocated `text` with LocalAlloc.
        unsafe {
            let _ = LocalFree(HLOCAL(text.0.cast()));
        }
        rendered
    }

    /// Longest account or domain name Windows documents, plus margin.
    const MAX_ACCOUNT_CHARS: usize = 256;

    fn acl_violation(detail: &str) -> AppError {
        // A typed code lets the frontend offer the one-click repair instead of
        // showing this sentence to a user who cannot act on it.
        AppError::data_directory_acl(format!(
            "data directory is accessible by another local account: it {detail}"
        ))
    }

    fn acl_error(detail: &str) -> AppError {
        AppError::data_directory(format!("data directory ACL check failed: {detail}"))
    }

    struct TokenGuard(HANDLE);

    impl Drop for TokenGuard {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by OpenProcessToken and is closed
            // exactly once here.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::Path;

        /// The well-known SID for BUILTIN\Users, resolved by SID so the test is
        /// independent of the machine's display language.
        const USERS_GROUP_SID: &str = "*S-1-5-32-545";

        struct TempDir(std::path::PathBuf);

        impl TempDir {
            fn new(name: &str) -> Self {
                let path = std::env::temp_dir()
                    .join(format!("voxitype-acl-{name}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&path);
                std::fs::create_dir_all(&path).expect("test temp directory should be creatable");
                Self(path)
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        /// Restrict the directory to the current user only, then optionally
        /// grant another principal. Returns false when ACL editing is refused.
        fn set_acl(dir: &Path, extra_grant: Option<&str>) -> bool {
            let sid_text = current_user_sid_string().expect("current user SID should resolve");
            let mut command = std::process::Command::new("icacls");
            command
                .arg(dir)
                .arg("/inheritance:r")
                .arg("/grant:r")
                // icacls requires a leading `*` to treat the argument as a SID.
                .arg(format!("*{sid_text}:(OI)(CI)F"));
            if let Some(grant) = extra_grant {
                command.arg("/grant").arg(grant);
            }
            command.status().map(|s| s.success()).unwrap_or(false)
        }

        /// A file whose DACL is protected and empty: exactly what
        /// `icacls /inheritance:r` alone produces on an existing file.
        fn strip_file_dacl(path: &Path) {
            let status = std::process::Command::new("icacls")
                .arg(path)
                .arg("/inheritance:r")
                .status()
                .expect("icacls should run");
            assert!(status.success(), "could not strip the file DACL");
        }

        #[test]
        fn restrict_repairs_a_directory_another_account_can_write() {
            let dir = TempDir::new("restrict-repair");
            let data = dir.0.join("data");
            std::fs::create_dir_all(&data).expect("data dir");
            let database = data.join("voxitype.db");
            std::fs::write(&database, b"SQLite format 3\0payload").expect("seed db");

            let grant = format!("{USERS_GROUP_SID}:(OI)(CI)(M)");
            if !set_acl(&dir.0, Some(&grant)) {
                eprintln!("skipping: could not grant write to BUILTIN\\Users");
                return;
            }
            assert!(
                validate(&dir.0).is_err(),
                "precondition: the directory must start rejected"
            );

            crate::data_dir_acl::restrict_to_current_user(&dir.0).expect("restrict should succeed");

            assert!(
                validate(&dir.0).is_ok(),
                "the repaired directory must pass validation"
            );
            assert_eq!(
                std::fs::read(&database).expect("existing file must stay readable"),
                b"SQLite format 3\0payload",
                "restricting must not lock the user out of existing files"
            );
        }

        #[test]
        fn restrict_recovers_a_file_left_with_an_empty_dacl() {
            let dir = TempDir::new("restrict-empty-file-dacl");
            let data = dir.0.join("data");
            std::fs::create_dir_all(&data).expect("data dir");
            let database = data.join("voxitype.db");
            std::fs::write(&database, b"SQLite format 3\0payload").expect("seed db");
            strip_file_dacl(&database);
            assert!(
                std::fs::read(&database).is_err(),
                "precondition: the stripped file must be unreadable"
            );

            crate::data_dir_acl::restrict_to_current_user(&dir.0).expect("restrict should succeed");

            assert_eq!(
                std::fs::read(&database).expect("restrict must restore read access"),
                b"SQLite format 3\0payload"
            );
        }

        #[test]
        fn current_user_only_acl_passes() {
            let dir = TempDir::new("default");
            if !set_acl(&dir.0, None) {
                eprintln!("skipping: could not set ACL");
                return;
            }
            assert!(
                validate(&dir.0).is_ok(),
                "a directory writable only by the current user must pass"
            );
        }

        #[test]
        fn write_granted_to_users_group_is_rejected() {
            let dir = TempDir::new("users-write");
            let grant = format!("{USERS_GROUP_SID}:(OI)(CI)(M)");
            if !set_acl(&dir.0, Some(&grant)) {
                eprintln!("skipping: could not grant write to BUILTIN\\Users");
                return;
            }
            let result = validate(&dir.0);
            assert!(
                result.is_err(),
                "a directory writable by BUILTIN\\Users must be rejected"
            );
        }

        #[test]
        fn inherit_only_write_grant_to_users_group_is_rejected() {
            let dir = TempDir::new("users-write-inherit-only");
            let grant = format!("{USERS_GROUP_SID}:(OI)(CI)(IO)(M)");
            if !set_acl(&dir.0, Some(&grant)) {
                eprintln!("skipping: could not grant inherit-only write to BUILTIN\\Users");
                return;
            }
            let result = validate(&dir.0);
            assert!(
                result.is_err(),
                "an inherit-only write grant lets created files inherit write and must be rejected"
            );
        }

        /// Build a minimal synthetic ACE: header, access mask, then the SID
        /// bytes. Lets the ACE-classification logic be tested without a real
        /// DACL.
        fn synthetic_ace(ace_type: u8, ace_flags: u8, mask: u32, sid: &[u8]) -> Vec<u8> {
            let size = ACE_SID_OFFSET + sid.len();
            let mut ace = vec![0u8; size];
            ace[0] = ace_type;
            ace[1] = ace_flags;
            ace[2..4].copy_from_slice(&(size as u16).to_le_bytes());
            ace[4..8].copy_from_slice(&mask.to_le_bytes());
            ace[ACE_SID_OFFSET..].copy_from_slice(sid);
            ace
        }

        /// S-1-5-18 (LocalSystem), a valid well-formed SID.
        const LOCAL_SYSTEM_SID: [u8; 12] = [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];

        fn ace_ptr(ace: &[u8]) -> *mut c_void {
            ace.as_ptr().cast_mut().cast()
        }

        #[test]
        fn allowed_write_aces_are_attributed() {
            let plain = synthetic_ace(
                ACCESS_ALLOWED_ACE_TYPE,
                0,
                FILE_GENERIC_WRITE.0,
                &LOCAL_SYSTEM_SID,
            );
            assert!(write_trustee(ace_ptr(&plain)).is_some());

            let inherit_only = synthetic_ace(
                ACCESS_ALLOWED_ACE_TYPE,
                INHERIT_ONLY_ACE.0 as u8,
                FILE_GENERIC_WRITE.0,
                &LOCAL_SYSTEM_SID,
            );
            assert!(write_trustee(ace_ptr(&inherit_only)).is_some());

            let callback = synthetic_ace(
                ACCESS_ALLOWED_CALLBACK_ACE_TYPE,
                0,
                FILE_GENERIC_WRITE.0,
                &LOCAL_SYSTEM_SID,
            );
            assert!(write_trustee(ace_ptr(&callback)).is_some());
        }

        #[test]
        fn object_write_aces_fail_closed() {
            for ace_type in [
                ACCESS_ALLOWED_OBJECT_ACE_TYPE,
                ACCESS_ALLOWED_CALLBACK_OBJECT_ACE_TYPE,
            ] {
                let ace = synthetic_ace(ace_type, 0, FILE_GENERIC_WRITE.0, &[]);
                assert!(
                    write_trustee(ace_ptr(&ace)).is_none(),
                    "object ACE type {ace_type} has no attributable trustee here"
                );
                assert!(
                    is_unattributable_write_ace(ace_ptr(&ace)),
                    "object ACE type {ace_type} with write rights must fail closed"
                );
            }
        }

        /// Read-only mask without the `READ_CONTROL` and `SYNCHRONIZE` bits
        /// that `FILE_GENERIC_WRITE` also carries.
        const READ_ONLY_RIGHTS: u32 = FILE_READ_DATA.0 | FILE_READ_ATTRIBUTES.0 | FILE_READ_EA.0;

        #[test]
        fn non_write_aces_are_ignored() {
            let read_ace = synthetic_ace(
                ACCESS_ALLOWED_ACE_TYPE,
                0,
                READ_ONLY_RIGHTS,
                &LOCAL_SYSTEM_SID,
            );
            assert!(write_trustee(ace_ptr(&read_ace)).is_none());
            assert!(!is_unattributable_write_ace(ace_ptr(&read_ace)));

            let object_read =
                synthetic_ace(ACCESS_ALLOWED_OBJECT_ACE_TYPE, 0, READ_ONLY_RIGHTS, &[]);
            assert!(!is_unattributable_write_ace(ace_ptr(&object_read)));
        }
    }
}
