use std::path::PathBuf;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, MessageBoxW, PostMessageW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    MB_SETFOREGROUND, MESSAGEBOX_STYLE,
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
                // The mutex stays open for the process lifetime; the handle value
                // itself does not need to be closed explicitly.
                let _ = handle;
                is_first
            }
            Err(_) => true,
        }
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
