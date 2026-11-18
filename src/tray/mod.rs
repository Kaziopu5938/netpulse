use std::sync::atomic::Ordering;
use std::sync::Arc;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIconBuilder};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MsgWaitForMultipleObjectsEx, PeekMessageW, PostMessageW, TranslateMessage,
    MSG, MWMO_INPUTAVAILABLE, PM_REMOVE, QS_ALLINPUT,
};

use crate::network::adapter;
use crate::platform::{exe_path, info};
use crate::startup;
use crate::state::{Shared, WM_EXIT, WM_TOGGLE_VISIBLE};
use crate::taskbar::display;

const ID_SETTINGS: &str = "open-settings";
const ID_TOGGLE: &str = "toggle-meter";
const ID_NETWORK: &str = "network-info";
const ID_STARTUP: &str = "start-with-windows";
const ID_UPDATES: &str = "check-updates";
const ID_EXIT: &str = "exit";

pub fn spawn(shared: Arc<Shared>) {
    let _ = std::thread::Builder::new()
        .name("netpulse-tray".into())
        .spawn(move || run(shared));
}

fn run(shared: Arc<Shared>) {
    let menu = Menu::new();
    let settings = MenuItem::with_id(ID_SETTINGS, "Open Settings", true, None);
    let toggle = CheckMenuItem::with_id(ID_TOGGLE, "Show Speed Meter", true, true, None);
    let network = MenuItem::with_id(ID_NETWORK, "Network Information", true, None);
    let startup_item = CheckMenuItem::with_id(
        ID_STARTUP,
        "Start with Windows",
        true,
        startup::enabled(),
        None,
    );
    let updates = MenuItem::with_id(ID_UPDATES, "Check for Updates", true, None);
    let quit = MenuItem::with_id(ID_EXIT, "Exit", true, None);
    let separator = PredefinedMenuItem::separator();

    let items: [&dyn tray_icon::menu::IsMenuItem; 7] = [
        &settings,
        &toggle,
        &network,
        &startup_item,
        &updates,
        &separator,
        &quit,
    ];
    if menu.append_items(&items).is_err() {
        return;
    }

    let icon = match Icon::from_rgba(crate::icon::rgba(32), 32, 32) {
        Ok(icon) => icon,
        Err(_) => return,
    };

    let tray = match TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("NetPulse")
        .build()
    {
        Ok(tray) => tray,
        Err(_) => return,
    };

    let mut last_tooltip = String::new();
    let mut message = MSG::default();
    let mut startup_state = startup::enabled();
    let mut last_startup_check = std::time::Instant::now();

    loop {
        if shared.exit.load(Ordering::Relaxed) {
            break;
        }

        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                if message.message == windows::Win32::UI::WindowsAndMessaging::WM_QUIT {
                    let _ = tray.set_visible(false);
                    return;
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id().0.as_str() {
                ID_SETTINGS => {
                    let _ = std::process::Command::new(exe_path())
                        .arg("--settings")
                        .spawn();
                }
                ID_TOGGLE => {
                    post_host(WM_TOGGLE_VISIBLE);
                }
                ID_NETWORK => show_network_info(&shared),
                ID_STARTUP => {
                    let wanted = !startup_state;
                    if let Err(error) = startup::set_enabled(wanted) {
                        info(
                            "NetPulse",
                            &format!("Could not change the startup entry: {error}"),
                        );
                    }
                    startup_state = startup::enabled();
                    last_startup_check = std::time::Instant::now();
                    startup_item.set_checked(startup_state);
                }
                ID_UPDATES => {
                    info(
                        "Check for Updates",
                        &format!(
                            "NetPulse {}\n\nNo update channel is configured in this build.\n\
                             Download newer releases from the project page.",
                            env!("CARGO_PKG_VERSION")
                        ),
                    );
                }
                ID_EXIT => post_host(WM_EXIT),
                _ => {}
            }
        }

        let config = match shared.config.read() {
            Ok(config) => config.clone(),
            Err(_) => break,
        };
        let sample = match shared.sample.read() {
            Ok(sample) => sample.clone(),
            Err(_) => break,
        };
        toggle.set_checked(config.visible);
        if last_startup_check.elapsed() > std::time::Duration::from_secs(5) {
            startup_state = startup::enabled();
            last_startup_check = std::time::Instant::now();
        }
        startup_item.set_checked(startup_state);

        let tooltip = display::tooltip(&config, &sample);
        if tooltip != last_tooltip {
            let _ = tray.set_tooltip(Some(&tooltip));
            last_tooltip = tooltip;
        }

        unsafe {
            MsgWaitForMultipleObjectsEx(None, 500, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
    }

    let _ = tray.set_visible(false);
}

fn post_host(msg: u32) {
    if let Some(hwnd) = crate::platform::host_window() {
        unsafe {
            let _ = PostMessageW(Some(hwnd), msg, WPARAM(0), LPARAM(0));
        }
    }
}

fn show_network_info(shared: &Shared) {
    let config = match shared.config.read() {
        Ok(config) => config.clone(),
        Err(_) => return,
    };
    let sample = match shared.sample.read() {
        Ok(sample) => sample.clone(),
        Err(_) => return,
    };

    let mut text = String::new();
    let adapters = adapter::list();
    for entry in &adapters {
        text.push_str(&format!(
            "{} — {}\n    {}\n",
            entry.name,
            if entry.up { "connected" } else { "disconnected" },
            entry.description
        ));
    }
    if adapters.is_empty() {
        text.push_str("No Ethernet, Wi-Fi or mobile adapters found.\n");
    }

    let selected = match config.adapter.as_deref() {
        Some(guid) => adapters
            .iter()
            .find(|entry| entry.guid == guid)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| guid.to_string()),
        None => "All active adapters".to_string(),
    };
    let active = if sample.connected {
        sample.adapter.as_str()
    } else {
        "none"
    };

    text.push_str(&format!(
        "\nSelected: {selected}\nActive: {active}\n\n{down}   {up}\n",
        down = format!(
            "{} {}",
            display::DOWN_ARROW,
            display::format_speed(sample.down_bps, config.unit)
        ),
        up = format!(
            "{} {}",
            display::UP_ARROW,
            display::format_speed(sample.up_bps, config.unit)
        ),
    ));
    info("Network Information", &text);
}
