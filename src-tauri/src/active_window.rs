//! Active foreground window detection (for per-app modes).
//!
//! Returns the process / app name of the window that currently has focus.
//! Used to auto-switch the formatting mode based on which application the
//! user is dictating into.

/// The currently focused application's process name, lowercased without a
/// trailing `.exe` (e.g. "code", "chrome", "notes"). Returns `None` if it
/// can't be determined or on unsupported platforms.
pub fn foreground_process_name() -> Option<String> {
    platform::foreground_process_name()
}

/// Normalize a process executable name to the canonical stored form: lowercased
/// with a trailing `.exe` removed (case-insensitively). Shared by the
/// foreground detector and the per-app mode store so lookups and inserts always
/// agree on the same key.
pub fn normalize_process_name(name: &str) -> String {
    let lowered = name.to_lowercase();
    lowered.strip_suffix(".exe").unwrap_or(&lowered).to_string()
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
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

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;

            let mut buf = [0u16; MAX_PATH as usize];
            let mut len = buf.len() as u32;
            let res = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR::from_raw(buf.as_mut_ptr()),
                &mut len,
            );
            let _ = CloseHandle(handle);

            if res.is_err() || len == 0 {
                return None;
            }

            let full_path = String::from_utf16_lossy(&buf[..len as usize]);
            let name = std::path::Path::new(&full_path)
                .file_name()
                .and_then(|n| n.to_str())?;
            Some(super::normalize_process_name(name))
        }
    }
}

// ponytail: osascript via System Events. Needs Automation permission the first
// time. Native objc2-app-kit if this becomes too slow/fragile.
#[cfg(target_os = "macos")]
mod platform {
    use std::process::Command;

    pub fn foreground_process_name() -> Option<String> {
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get name of first process whose frontmost is true",
            ])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if name.is_empty() {
            return None;
        }
        Some(super::normalize_process_name(&name))
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
mod platform {
    pub fn foreground_process_name() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_process_name;

    #[test]
    fn strips_exe_and_lowercases() {
        assert_eq!(normalize_process_name("Code.exe"), "code");
        assert_eq!(normalize_process_name("CHROME"), "chrome");
        assert_eq!(normalize_process_name("Notes"), "notes");
    }

    #[cfg(windows)]
    mod windows_smoke {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::Foundation::MAX_PATH;
        use windows::Win32::System::Threading::OpenProcess;
        use windows::Win32::System::Threading::QueryFullProcessImageNameW;
        use windows::Win32::System::Threading::PROCESS_NAME_FORMAT;
        use windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION;
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

        #[test]
        fn test_foreground_process() {
            unsafe {
                let hwnd = GetForegroundWindow();
                if hwnd.0.is_null() {
                    return;
                }

                let mut pid: u32 = 0;
                GetWindowThreadProcessId(hwnd, Some(&mut pid));
                if pid == 0 {
                    return;
                }

                let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
                    Ok(h) => h,
                    Err(_) => return,
                };

                let mut buf = [0u16; MAX_PATH as usize];
                let mut len = buf.len() as u32;
                let res = QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_FORMAT(0),
                    windows::core::PWSTR::from_raw(buf.as_mut_ptr()),
                    &mut len,
                );
                let _ = CloseHandle(handle);
                let _ = res;
            }
        }
    }
}
