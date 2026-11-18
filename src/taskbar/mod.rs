pub mod display;
pub mod overlay;

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Once};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowExW, FindWindowW, GetMessageW,
    GetWindowLongPtrW, GetWindowRect, IsWindow, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SetTimer, SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW,
    GWLP_USERDATA, MSG, WM_CREATE, WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WM_TIMER, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::platform::wide;
use crate::settings::Config;
use crate::state::{
    Shared, HOST_CLASS, WM_CONFIG_CHANGED, WM_EXIT, WM_SPEED_UPDATED, WM_TOGGLE_VISIBLE,
};

const TIMER_ID: usize = 1;

pub fn taskbar_window() -> Option<HWND> {
    let class = wide("Shell_TrayWnd");
    unsafe { FindWindowW(PCWSTR(class.as_ptr()), None) }.ok()
}

pub fn tray_window_of(taskbar: HWND) -> Option<HWND> {
    let class = wide("TrayNotifyWnd");
    unsafe {
        FindWindowExW(
            Some(taskbar),
            None,
            PCWSTR(class.as_ptr()),
            PCWSTR::null(),
        )
    }
    .ok()
}

pub fn window_rect(hwnd: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
    Some(rect)
}

pub fn taskbar_is_vertical() -> bool {
    taskbar_window()
        .and_then(window_rect)
        .map(|rect| (rect.bottom - rect.top) > (rect.right - rect.left))
        .unwrap_or(false)
}

struct Host {
    shared: Arc<Shared>,
    meter: HWND,
    taskbar_created: u32,
}

impl Host {
    unsafe fn ensure_meter(&mut self) {
        if !self.meter.is_invalid() && IsWindow(Some(self.meter)).as_bool() {
            return;
        }
        self.meter = overlay::create(&self.shared).unwrap_or_default();
        overlay::sync(self.meter, &self.shared);
    }

    unsafe fn recreate_meter(&mut self) {
        overlay::destroy(self.meter);
        self.meter = overlay::create(&self.shared).unwrap_or_default();
        overlay::sync(self.meter, &self.shared);
    }
}

unsafe fn host_of(hwnd: HWND) -> Option<&'static mut Host> {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Host;
    if pointer.is_null() {
        None
    } else {
        Some(&mut *pointer)
    }
}

unsafe extern "system" fn host_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lparam.0 as *const CREATESTRUCTW);
        let host = create.lpCreateParams as *mut Host;
        if host.is_null() {
            return LRESULT(0);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, host as isize);
        let host_ref = &*host;
        host_ref.shared.host.store(hwnd.0 as isize, Ordering::Relaxed);
        return LRESULT(1);
    }

    let Some(host) = host_of(hwnd) else {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    };

    if msg == host.taskbar_created {
        host.recreate_meter();
        return LRESULT(0);
    }

    match msg {
        WM_CREATE => {
            host.ensure_meter();
            SetTimer(Some(hwnd), TIMER_ID, 1000, None);
            LRESULT(0)
        }
        WM_TIMER => {
            host.ensure_meter();
            overlay::sync(host.meter, &host.shared);
            LRESULT(0)
        }
        WM_SPEED_UPDATED => {
            host.ensure_meter();
            overlay::sync(host.meter, &host.shared);
            LRESULT(0)
        }
        WM_CONFIG_CHANGED => {
            let config = Config::load();
            let mode_changed = {
                let mut slot = host.shared.config.write().unwrap();
                let changed = slot.dock_mode != config.dock_mode;
                *slot = config;
                changed
            };
            if mode_changed {
                host.recreate_meter();
            }
            overlay::sync(host.meter, &host.shared);
            LRESULT(0)
        }
        WM_TOGGLE_VISIBLE => {
            {
                let mut config = host.shared.config.write().unwrap();
                config.visible = !config.visible;
                let _ = config.save();
            }
            overlay::sync(host.meter, &host.shared);
            LRESULT(0)
        }
        WM_EXIT => {
            host.shared.exit.store(true, Ordering::Relaxed);
            overlay::destroy(host.meter);
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Host;
            if !pointer.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(pointer));
            }
            PostQuitMessage(0);
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

static HOST_REGISTERED: Once = Once::new();

fn ensure_host_class() {
    HOST_REGISTERED.call_once(|| unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class_name = wide(HOST_CLASS);
        let class = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::WNDCLASS_STYLES(0),
            lpfnWndProc: Some(host_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: windows::Win32::Foundation::HINSTANCE(instance.0),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: Default::default(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
        };
        RegisterClassW(&class);
    });
}

/// Creates the hidden message window and runs the UI message loop.
pub fn run(shared: Arc<Shared>) {
    unsafe {
        ensure_host_class();
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let taskbar_created = {
            let name = wide("TaskbarCreated");
            RegisterWindowMessageW(PCWSTR(name.as_ptr()))
        };
        let host = Box::new(Host {
            shared: shared.clone(),
            meter: HWND::default(),
            taskbar_created,
        });
        let host = Box::into_raw(host);

        let class_name = wide(HOST_CLASS);
        let title = wide("NetPulse");
        let created = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            PCWSTR(class_name.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(windows::Win32::Foundation::HINSTANCE(instance.0)),
            Some(host as *const c_void),
        );

        if created.is_err() {
            drop(Box::from_raw(host));
            return;
        }

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
