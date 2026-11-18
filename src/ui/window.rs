use std::time::{Duration, Instant};

use eframe::egui;
use windows::core::PCWSTR;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::network::adapter;
use crate::platform::{self, wide};
use crate::settings::{config_path, Background, Config, DockMode, Position, Theme, Unit};
use crate::startup;
use crate::state::WM_CONFIG_CHANGED;

pub fn run() -> eframe::Result {
    if focus_existing() {
        return Ok(());
    }
    let config = Config::load();
    let theme = config.theme;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NetPulse Settings")
            .with_inner_size([580.0, 640.0])
            .with_min_inner_size([500.0, 500.0])
            .with_icon(window_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "NetPulse Settings",
        options,
        Box::new(move |cc| {
            apply_theme(&cc.egui_ctx, theme);
            Ok(Box::new(SettingsApp::new(config)))
        }),
    )
}

fn window_icon() -> egui::IconData {
    let size = 64;
    egui::IconData {
        rgba: crate::icon::rgba(size),
        width: size,
        height: size,
    }
}

fn focus_existing() -> bool {
    let title = wide("NetPulse Settings");
    match unsafe { FindWindowW(PCWSTR::null(), PCWSTR(title.as_ptr())) } {
        Ok(hwnd) => {
            unsafe {
                let _ = ShowWindow(hwnd, SW_RESTORE);
                let _ = SetForegroundWindow(hwnd);
            }
            true
        }
        Err(_) => false,
    }
}

fn apply_theme(context: &egui::Context, theme: Theme) {
    context.set_theme(match theme {
        Theme::System => egui::ThemePreference::System,
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
    });
}

fn parse_color(hex: &str) -> [u8; 3] {
    let value = u32::from_str_radix(hex.trim().trim_start_matches('#'), 16).unwrap_or(0xFFFFFF);
    [
        ((value >> 16) & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        (value & 0xFF) as u8,
    ]
}

fn format_color(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

struct SettingsApp {
    draft: Config,
    saved: Config,
    adapters: Vec<adapter::Adapter>,
    autostart: bool,
    status: String,
    last_save: Instant,
}

impl SettingsApp {
    fn new(config: Config) -> Self {
        Self {
            adapters: adapter::list(),
            autostart: startup::enabled(),
            status: format!("Settings file: {}", config_path().display()),
            draft: config.clone(),
            saved: config,
            last_save: Instant::now(),
        }
    }

    fn flush(&mut self, context: &egui::Context) {
        if self.draft == self.saved || self.last_save.elapsed() < Duration::from_millis(250) {
            return;
        }
        match self.draft.save() {
            Ok(()) => {
                self.saved = self.draft.clone();
                self.status = "Saved".to_string();
                platform::post_host(WM_CONFIG_CHANGED);
            }
            Err(error) => self.status = format!("Could not save settings: {error}"),
        }
        self.last_save = Instant::now();
        context.request_repaint();
    }

    fn reload_if_changed(&mut self) {
        if self.draft != self.saved || self.last_save.elapsed() < Duration::from_millis(1000) {
            return;
        }
        let disk = Config::load();
        if disk != self.saved {
            self.draft = disk.clone();
            self.saved = disk;
            self.autostart = startup::enabled();
        }
    }

    fn adapter_label(&self) -> String {
        let selected = self.draft.adapter.as_deref();
        self.adapters
            .iter()
            .find(|entry| Some(entry.guid.as_str()) == selected)
            .map(|entry| format!("{} ({})", entry.name, entry.kind))
            .unwrap_or_else(|| "All active adapters".to_string())
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("NetPulse");
        ui.label("Real-time download and upload speed next to the system tray.");
        ui.add_space(6.0);
        ui.separator();

        ui.heading("Display");
        ui.checkbox(&mut self.draft.visible, "Show speed meter");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.draft.show_download, "Download");
            ui.checkbox(&mut self.draft.show_upload, "Upload");
        });
        ui.horizontal(|ui| {
            ui.label("Units");
            egui::ComboBox::from_id_salt("units")
                .selected_text(match self.draft.unit {
                    Unit::Bits => "Bits per second",
                    Unit::Bytes => "Bytes per second",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.unit, Unit::Bits, "Bits (Kbps / Mbps)");
                    ui.selectable_value(&mut self.draft.unit, Unit::Bytes, "Bytes (KB/s / MB/s)");
                });
        });
        ui.add(
            egui::Slider::new(&mut self.draft.update_interval_ms, 250..=5000)
                .step_by(250.0)
                .text("Update interval (ms)"),
        );
        ui.horizontal(|ui| {
            ui.label("Position");
            egui::ComboBox::from_id_salt("position")
                .selected_text(match self.draft.position {
                    Position::LeftOfTray => "Left of the tray",
                    Position::RightOfTray => "Right of the tray",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.draft.position,
                        Position::LeftOfTray,
                        "Left of the tray",
                    );
                    ui.selectable_value(
                        &mut self.draft.position,
                        Position::RightOfTray,
                        "Right of the tray",
                    );
                });
        });
        ui.add(egui::Slider::new(&mut self.draft.offset, 0..=40).text("Gap from tray (px)"));

        ui.add_space(4.0);
        ui.separator();
        ui.heading("Appearance");
        ui.horizontal(|ui| {
            ui.label("Theme");
            let before = self.draft.theme;
            egui::ComboBox::from_id_salt("theme")
                .selected_text(match self.draft.theme {
                    Theme::System => "System",
                    Theme::Light => "Light",
                    Theme::Dark => "Dark",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.theme, Theme::System, "System");
                    ui.selectable_value(&mut self.draft.theme, Theme::Light, "Light");
                    ui.selectable_value(&mut self.draft.theme, Theme::Dark, "Dark");
                });
            if self.draft.theme != before {
                apply_theme(ui.ctx(), self.draft.theme);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Font");
            ui.text_edit_singleline(&mut self.draft.font_family);
        });
        ui.add(egui::Slider::new(&mut self.draft.font_size, 8..=24).text("Font size (px)"));

        let mut text_rgb = parse_color(&self.draft.text_color);
        if ui
            .color_edit_button_srgb(&mut text_rgb)
            .changed()
        {
            self.draft.text_color = format_color(text_rgb);
        }
        ui.label("Text colour");

        let mut solid = self.draft.background == Background::Solid;
        if ui.checkbox(&mut solid, "Opaque background").changed() {
            self.draft.background = if solid {
                Background::Solid
            } else {
                Background::Transparent
            };
        }
        if solid {
            let mut background_rgb = parse_color(&self.draft.background_color);
            if ui.color_edit_button_srgb(&mut background_rgb).changed() {
                self.draft.background_color = format_color(background_rgb);
            }
            ui.label("Background colour");
            ui.add(
                egui::Slider::new(&mut self.draft.background_opacity, 0..=255)
                    .text("Background opacity"),
            );
        }

        ui.add_space(4.0);
        ui.separator();
        ui.heading("Taskbar integration");
        ui.radio_value(
            &mut self.draft.dock_mode,
            DockMode::Overlay,
            "Overlay above the taskbar (recommended)",
        );
        ui.radio_value(
            &mut self.draft.dock_mode,
            DockMode::Taskbar,
            "Taskbar dock (embeds into Explorer)",
        );
        ui.small(
            "Windows 10/11 have no supported third-party taskbar toolbar API. Overlay draws a \
             click-through, always-on-top window in the taskbar band. Taskbar dock reparents a \
             child window into Explorer's taskbar: closer to native, but unsupported and may stop \
             working after Windows updates.",
        );

        ui.add_space(4.0);
        ui.separator();
        ui.heading("Network");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("adapter")
                .width(360.0)
                .selected_text(self.adapter_label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.draft.adapter,
                        None,
                        "All active adapters",
                    );
                    for entry in &self.adapters {
                        let label = format!(
                            "{} — {} ({})",
                            entry.name,
                            entry.kind,
                            if entry.up { "connected" } else { "disconnected" }
                        );
                        ui.selectable_value(
                            &mut self.draft.adapter,
                            Some(entry.guid.clone()),
                            label,
                        );
                    }
                });
            if ui.button("Refresh").clicked() {
                self.adapters = adapter::list();
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.heading("System");
        let mut autostart = self.autostart;
        if ui.checkbox(&mut autostart, "Start with Windows").changed() {
            if let Err(error) = startup::set_enabled(autostart) {
                self.status = format!("Could not update startup entry: {error}");
            }
            self.autostart = startup::enabled();
        }
        ui.horizontal(|ui| {
            if ui.button("Reset to defaults").clicked() {
                self.draft = Config::default();
                apply_theme(ui.ctx(), self.draft.theme);
                self.last_save = Instant::now() - Duration::from_secs(1);
            }
            if ui.button("Open settings folder").clicked() {
                if let Some(folder) = config_path().parent() {
                    let _ = std::process::Command::new("explorer").arg(folder).spawn();
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.small(&self.status);
        ui.small(format!(
            "NetPulse {} — changes are applied immediately and saved to disk.",
            env!("CARGO_PKG_VERSION")
        ));
    }
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.controls(ui);
            });
        });
        self.flush(&context);
    }

    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.reload_if_changed();
        context.request_repaint_after(Duration::from_secs(1));
    }
}
