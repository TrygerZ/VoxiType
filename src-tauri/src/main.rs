// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Single-instance guard — prevents two VoxiType processes from starting
    // simultaneously. Two processes would contend for WebView2's per-user data
    // directory, causing HRESULT 0x8007139F ("group or resource not in the
    // correct state") on the second launch.
    ensure_single_instance();
    voxitype_lib::run()
}

/// Check that no other VoxiType instance is already running (named mutex).
/// If one is found, exit immediately — the user can restore the first instance
/// via the system tray.
fn ensure_single_instance() {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows::Win32::System::Threading::CreateMutexW;
        use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;

        let name: Vec<u16> = "VoxiType_SingleInstance_Mutex"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            // CreateMutexW succeeds even when the mutex already exists —
            // ERROR_ALREADY_EXISTS (via GetLastError) tells us it's a duplicate.
            let handle = match CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
                Ok(h) => h,
                Err(_) => return, // Real API failure — continue anyway.
            };

            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle).ok();
                let msg: Vec<u16> = "VoxiType is already running.\nCheck the system tray (near the clock) to restore the window."
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let title: Vec<u16> = "VoxiType"
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                MessageBoxW(
                    None,
                    PCWSTR(msg.as_ptr()),
                    PCWSTR(title.as_ptr()),
                    windows::Win32::UI::WindowsAndMessaging::MB_OK,
                );
                std::process::exit(0);
            }
            // First instance — leak mutex handle so the kernel mutex persists
            // for the lifetime of the process.
            let _ = Box::leak(Box::new(handle));
        }
    }
}
