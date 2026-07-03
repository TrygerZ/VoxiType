//! Active foreground window detection (for per-app modes).
//!
//! Returns the process executable name (e.g. "Code.exe", "chrome.exe") of the
//! window that currently has focus. Used to auto-switch the formatting mode
//! based on which application the user is dictating into.

/// The currently focused application's process name, lowercased without the
/// `.exe` suffix (e.g. "code", "chrome"). Returns `None` if it can't be
/// determined or on unsupported platforms.
pub fn foreground_process_name() -> Option<String> {
    platform::foreground_process_name()
}

/// Normalize a process executable name to the canonical stored form: lowercased
/// with a trailing `.exe` removed (case-insensitively). Shared by the
/// foreground detector and the per-app mode store so lookups and inserts always
/// agree on the same key.
pub fn normalize_process_name(name: &str) -> String {
    let lowered = name.to_lowercase();
    lowered
        .strip_suffix(".exe")
        .unwrap_or(&lowered)
        .to_string()
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::ProcessStatus::GetModuleBaseNameW;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    pub fn foreground_process_name() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0.is_null() {
                return None;
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return None;
            }

            let handle =
                OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

            let mut buf = [0u16; MAX_PATH as usize];
            let len = GetModuleBaseNameW(handle, None, &mut buf);
            let _ = CloseHandle(handle);

            if len == 0 {
                return None;
            }

            let name = String::from_utf16_lossy(&buf[..len as usize]);
            Some(super::normalize_process_name(&name))
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn foreground_process_name() -> Option<String> {
        None
    }
}
