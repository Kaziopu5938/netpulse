use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::{
    SetDefaultDllDirectories, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_SYSTEM32,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, MessageBoxW, PostMessageW, SetForegroundWindow, ShowWindow, MB_ICONERROR,
    MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND, MESSAGEBOX_STYLE, SW_RESTORE,
};

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn message(title: &str, text: &str, icon: MESSAGEBOX_STYLE) {
    let text = wide(text);
    let title = wide(title);
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | icon | MB_SETFOREGROUND,
        );
    }
}

pub fn info(title: &str, text: &str) {
    message(title, text, MB_ICONINFORMATION);
}

pub fn error(title: &str, text: &str) {
    message(title, text, MB_ICONERROR);
}

pub fn host_window() -> Option<HWND> {
    let class = wide(crate::state::HOST_CLASS);
    unsafe { FindWindowW(PCWSTR(class.as_ptr()), None) }.ok()
}

/// Posts a message to the running background instance, if there is one.
pub fn post_host(msg: u32) {
    if let Some(hwnd) = host_window() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), msg, WPARAM(0), LPARAM(0));
        }
    }
}

pub fn exe_path() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("netpulse.exe"))
}

/// Returns `true` when this process is the first instance of `name`.
pub fn ensure_single_instance(name: &str) -> bool {
    let name = wide(name);
    unsafe {
        match CreateMutexW(None, true, PCWSTR(name.as_ptr())) {
            Ok(handle) => {
                let is_first = GetLastError() != ERROR_ALREADY_EXISTS;
                if !is_first {
                    // A second launch should surface the running instance
                    // instead of exiting silently with no explanation.
                    if let Some(hwnd) = host_window() {
                        let _ = ShowWindow(hwnd, SW_RESTORE);
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
                // Keep the handle alive for the process lifetime; Windows
                // closes it when the process exits.
                let _ = handle;
                is_first
            }
            // Fail closed: if the mutex cannot be created, assume another
            // instance is running rather than starting a second one.
            Err(_) => false,
        }
    }
}

/// Removes the current working directory from the DLL search order. The app
/// can be installed in (or run from) a user-writable folder, so a bare DLL name
/// must never resolve against whatever directory the process started in.
pub fn harden_dll_search() {
    unsafe {
        let _ = SetDefaultDllDirectories(
            LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
    }
}

pub fn enable_dpi_awareness() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}
