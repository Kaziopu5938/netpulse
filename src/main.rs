#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod icon;
mod network;
mod platform;
mod settings;
mod startup;
mod state;
mod taskbar;
mod tray;
mod ui;

fn main() {
    platform::enable_dpi_awareness();

    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.iter().any(|arg| arg == "--enable-startup") {
        let _ = startup::set_enabled(true);
        return;
    }
    if arguments.iter().any(|arg| arg == "--disable-startup") {
        let _ = startup::set_enabled(false);
        return;
    }
    if arguments.iter().any(|arg| arg == "--settings") {
        ui::run();
        return;
    }

    if !platform::ensure_single_instance("Local\\NetPulseInstance") {
        return;
    }

    let config = settings::Config::load();
    let _ = startup::sync(config.start_with_windows);
    let shared = state::Shared::new(config);
    tray::spawn(shared.clone());
    network::spawn_sampler(shared.clone());
    taskbar::run(shared);

    // The tray thread removes its icon on exit; give it a moment before the
    // process disappears so no ghost icon is left in the notification area.
    std::thread::sleep(std::time::Duration::from_millis(700));
}
