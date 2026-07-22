// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Single-instance guard — prevents two VoxiType processes from starting
    // simultaneously. Two processes would contend for WebView2's per-user data
    // directory, causing HRESULT 0x8007139F ("group or resource not in the
    // correct state") on the second launch. On macOS/Linux a second instance
    // would also fight for the same SQLite DB and global hotkey.
    ensure_single_instance();
    voxitype_lib::run()
}

/// Exit if another VoxiType instance is already running. The first instance
/// keeps a process-scoped lock; the user restores it via the system tray.
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

    // ponytail: flock on a temp lockfile. no tray popup on mac — stderr is enough
    // for `tauri dev`; packaged app still exits cleanly. upgrade to a native
    // dialog if dual-launch UX becomes a support issue.
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::os::unix::io::AsRawFd;

        let path = std::env::temp_dir().join("voxitype.singleinstance.lock");
        let file = match OpenOptions::new().create(true).write(true).open(&path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            eprintln!("VoxiType is already running. Restore it from the menu bar / system tray.");
            std::process::exit(0);
        }
        // Keep the FD open for the process lifetime so the lock is held.
        std::mem::forget(file);
    }
}
