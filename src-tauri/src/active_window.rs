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

#[cfg(not(windows))]
mod platform {
    pub fn foreground_process_name() -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
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
            println!("hwnd: {:?}", hwnd);
            if hwnd.0.is_null() {
                println!("hwnd is null");
                return;
            }

            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            println!("pid: {}", pid);
            if pid == 0 {
                println!("pid is 0");
                return;
            }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
            println!("handle: {:?}", handle);
            let handle = match handle {
                Ok(h) => h,
                Err(e) => {
                    println!("OpenProcess failed: {:?}", e);
                    return;
                }
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
            println!("QueryFullProcessImageNameW result: {:?}, len: {}", res, len);
            if let Err(e) = res {
                println!("error: {:?}", e);
                return;
            }

            let full_path = String::from_utf16_lossy(&buf[..len as usize]);
            println!("full_path: {}", full_path);
            let name = std::path::Path::new(&full_path)
                .file_name()
                .and_then(|n| n.to_str());
            println!("name: {:?}", name);
        }
    }
}
