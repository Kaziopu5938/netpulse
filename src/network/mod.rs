pub mod adapter;
pub mod monitor;

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

use crate::state::{Shared, WM_SPEED_UPDATED};

pub fn spawn_sampler(shared: Arc<Shared>) {
    if let Err(error) = std::thread::Builder::new()
        .name("netpulse-sampler".into())
        .spawn(move || run(shared))
    {
        eprintln!("netpulse: could not start the sampler thread: {error}");
    }
}

fn run(shared: Arc<Shared>) {
    let mut monitor = monitor::Monitor::new();
    loop {
        let interval = shared
            .config
            .read()
            .map(|config| config.update_interval_ms.clamp(250, 5000))
            .unwrap_or(1000);
        std::thread::sleep(Duration::from_millis(interval));

        if shared.exit.load(Ordering::Relaxed) {
            return;
        }

        let selected = shared
            .config
            .read()
            .ok()
            .and_then(|config| config.adapter.clone());
        let sample = monitor.sample(selected.as_deref());
        if let Ok(mut slot) = shared.sample.write() {
            *slot = sample;
        }

        post(shared.host.load(Ordering::Relaxed), WM_SPEED_UPDATED);
    }
}

fn post(target: isize, msg: u32) {
    if target != 0 {
        unsafe {
            let _ = PostMessageW(Some(HWND(target as *mut c_void)), msg, WPARAM(0), LPARAM(0));
        }
    }
}
