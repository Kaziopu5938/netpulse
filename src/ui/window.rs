use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Layout as EguiLayout, Margin, Rect, RichText, Sense, Stroke, TextStyle, Vec2,
};
use windows::core::PCWSTR;
use windows::Win32::System::SystemInformation::GetWindowsDirectoryW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

use crate::network::adapter;
use crate::platform::{self, wide};
use crate::settings::{
    config_path, system_uses_light_theme, Background, Config, DockMode, Layout, Position, Theme,
    Unit,
};
use crate::startup;
use crate::state::{Sample, WM_CONFIG_CHANGED};
use crate::taskbar::display;

const RAIL_WIDTH: f32 = 196.0;
const DISPLAY_FAMILY: &str = "NetPulse Display";

pub fn run() -> eframe::Result {
    if focus_existing() {
        return Ok(());
    }
    let config = Config::load();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NetPulse Settings")
            .with_inner_size([660.0, 540.0])
            .with_min_inner_size([600.0, 500.0])
            .with_icon(window_icon()),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "NetPulse Settings",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
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

/// Resolves Explorer by absolute path. Launching a bare `explorer` from a
/// user-writable / portable directory would let a local `explorer.exe` be
/// picked up instead of the system one.
fn explorer_path() -> Option<PathBuf> {
    let mut buffer = [0u16; 260];
    let length = unsafe { GetWindowsDirectoryW(Some(&mut buffer)) } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }
    let directory = String::from_utf16_lossy(&buffer[..length]);
    let path = PathBuf::from(directory).join("explorer.exe");
    path.is_file().then_some(path)
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

// ---------------------------------------------------------------------------
// Visual language
// ---------------------------------------------------------------------------

struct Palette {
    canvas: Color32,
    rail: Color32,
    surface: Color32,
    input: Color32,
    hairline: Color32,
    ink: Color32,
    muted: Color32,
    accent: Color32,
    good: Color32,
}

fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            canvas: Color32::from_rgb(10, 16, 27),
            rail: Color32::from_rgb(13, 21, 35),
            surface: Color32::from_rgb(19, 30, 47),
            input: Color32::from_rgb(15, 24, 39),
            hairline: Color32::from_rgb(30, 45, 66),
            ink: Color32::from_rgb(233, 241, 250),
            muted: Color32::from_rgb(136, 157, 180),
            accent: Color32::from_rgb(57, 196, 240),
            good: Color32::from_rgb(61, 220, 151),
        }
    } else {
        Palette {
            canvas: Color32::from_rgb(244, 247, 251),
            rail: Color32::WHITE,
            surface: Color32::from_rgb(233, 239, 246),
            input: Color32::WHITE,
            hairline: Color32::from_rgb(217, 227, 238),
            ink: Color32::from_rgb(15, 27, 42),
            muted: Color32::from_rgb(92, 113, 136),
            accent: Color32::from_rgb(11, 127, 168),
            good: Color32::from_rgb(16, 145, 96),
        }
    }
}

fn display_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(DISPLAY_FAMILY.into()))
}

fn install_fonts(context: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut loaded = false;
    for (name, path) in [
        ("segoe", r"C:\Windows\Fonts\segoeui.ttf"),
        ("segoe-semibold", r"C:\Windows\Fonts\seguisb.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, name.to_owned());
            loaded = true;
        }
    }
    if loaded {
        fonts.families.insert(
            FontFamily::Name(DISPLAY_FAMILY.into()),
            vec!["segoe-semibold".to_owned(), "segoe".to_owned()],
        );
        context.set_fonts(fonts);
    }
}

fn apply_style(context: &egui::Context, dark: bool) {
    let p = palette(dark);
    let mut style = (*context.style_of(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    }))
    .clone();

    style.text_styles = [
        (TextStyle::Heading, display_font(19.0)),
        (TextStyle::Body, FontId::new(13.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(13.5, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
        (TextStyle::Monospace, FontId::monospace(12.0)),
    ]
    .into();

    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(11.0, 5.0);
    style.spacing.interact_size = Vec2::new(40.0, 25.0);
    style.spacing.slider_width = 185.0;
    style.spacing.combo_width = 210.0;
    style.spacing.indent = 16.0;

    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = p.canvas;
    visuals.window_fill = p.canvas;
    visuals.faint_bg_color = p.surface;
    visuals.extreme_bg_color = p.input;
    visuals.override_text_color = Some(p.ink);
    visuals.hyperlink_color = p.accent;
    visuals.window_stroke = Stroke::new(1.0, p.hairline);
    visuals.window_corner_radius = CornerRadius::same(10);
    visuals.selection.bg_fill = p.accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, p.ink);

    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(7);
    }
    visuals.widgets.noninteractive.bg_fill = p.surface;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.hairline);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, p.ink);
    visuals.widgets.inactive.bg_fill = p.input;
    visuals.widgets.inactive.weak_bg_fill = p.input;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.hairline);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, p.ink);
    visuals.widgets.hovered.bg_fill = p.surface;
    visuals.widgets.hovered.weak_bg_fill = p.surface;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, p.accent.gamma_multiply(0.7));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.ink);
    visuals.widgets.active.bg_fill = p.accent;
    visuals.widgets.active.weak_bg_fill = p.accent;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, p.canvas);
    visuals.widgets.open.bg_fill = p.surface;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, p.hairline);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, p.ink);

    style.visuals = visuals;
    let theme = if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    context.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    context.set_style_of(theme, style);
}

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Display,
    Appearance,
    Network,
    System,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Display => "Display",
            Section::Appearance => "Appearance",
            Section::Network => "Network",
            Section::System => "System",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Section::Display => "What the meter shows and how often it updates.",
            Section::Appearance => "Type, colour and where the meter sits in the taskbar.",
            Section::Network => "Which interfaces feed the counter.",
            Section::System => "Startup, taskbar integration and the files behind the app.",
        }
    }
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
    status: String,
    status_ok: bool,
    last_save: Option<Instant>,
    section: Section,
    applied_dark: Option<bool>,
    applied_fonts: bool,
}

impl SettingsApp {
    fn new(config: Config) -> Self {
        Self {
            adapters: adapter::list(),
            status: "Saved".to_string(),
            status_ok: true,
            draft: config.clone(),
            saved: config,
            last_save: None,
            section: Section::Display,
            applied_dark: None,
            applied_fonts: false,
        }
    }

    fn dark(&self) -> bool {
        match self.draft.theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => !system_uses_light_theme(),
        }
    }

    fn ensure_style(&mut self, context: &egui::Context) {
        let dark = self.dark();
        if self.applied_dark != Some(dark) {
            apply_style(context, dark);
            self.applied_dark = Some(dark);
        }
        if !self.applied_fonts {
            install_fonts(context);
            self.applied_fonts = true;
        }
    }

    /// The background process can write two fields independently of this
    /// window (tray visibility toggle and Start-with-Windows toggle). Merge
    /// those changes before saving so an edit here cannot silently undo them.
    fn merge_external_changes(&mut self) {
        let disk = Config::load();
        if disk == self.saved {
            return;
        }
        if self.draft.visible == self.saved.visible {
            self.draft.visible = disk.visible;
        }
        if self.draft.start_with_windows == self.saved.start_with_windows {
            self.draft.start_with_windows = disk.start_with_windows;
        }
        self.saved = disk;
    }

    fn flush(&mut self, context: &egui::Context) {
        self.merge_external_changes();
        if self.draft == self.saved
            || self
                .last_save
                .is_some_and(|at| at.elapsed() < Duration::from_millis(250))
        {
            return;
        }
        match self.draft.save() {
            Ok(()) => {
                self.saved = self.draft.clone();
                self.status = "Saved".to_string();
                self.status_ok = true;
                platform::post_host(WM_CONFIG_CHANGED);
            }
            Err(error) => {
                self.status = format!("Could not save: {error}");
                self.status_ok = false;
            }
        }
        self.last_save = Some(Instant::now());
        context.request_repaint();
    }

    fn reload_if_changed(&mut self) {
        if self.draft != self.saved
            || self
                .last_save
                .is_some_and(|at| at.elapsed() < Duration::from_millis(1000))
        {
            return;
        }
        let disk = Config::load();
        if disk != self.saved {
            self.draft = disk.clone();
            self.saved = disk;
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

    // -- rail ---------------------------------------------------------------

    fn rail(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.label(
            RichText::new("NetPulse")
                .font(display_font(19.0))
                .color(p.ink),
        );
        ui.label(
            RichText::new("Taskbar speed meter")
                .size(11.5)
                .color(p.muted),
        );
        ui.add_space(22.0);

        let sections = [
            Section::Display,
            Section::Appearance,
            Section::Network,
            Section::System,
        ];
        let mut chosen = self.section;
        for section in sections {
            if nav_item(ui, section.label(), self.section == section, p).clicked() {
                chosen = section;
            }
        }
        self.section = chosen;

        ui.with_layout(EguiLayout::bottom_up(Align::Min), |ui| {
            ui.label(
                RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .size(11.0)
                    .color(p.muted),
            );
            ui.horizontal(|ui| {
                let dot = if self.status_ok { p.good } else { p.accent };
                let (rect, _) = ui.allocate_exact_size(Vec2::new(7.0, 7.0), Sense::hover());
                ui.painter().circle_filled(rect.center(), 3.5, dot);
                ui.label(RichText::new(&self.status).size(11.5).color(p.muted));
            });
        });
    }

    // -- content ------------------------------------------------------------

    fn content(&mut self, ui: &mut egui::Ui, p: &Palette) {
        self.preview(ui, p);
        ui.add_space(16.0);

        ui.label(
            RichText::new(self.section.label())
                .font(display_font(17.0))
                .color(p.ink),
        );
        ui.label(
            RichText::new(self.section.subtitle())
                .size(12.0)
                .color(p.muted),
        );
        ui.add_space(10.0);

        let width = ui.available_width();
        let (rule, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
        ui.painter()
            .rect_filled(rule, CornerRadius::ZERO, p.hairline);
        ui.add_space(14.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.section {
                Section::Display => self.display_section(ui, p),
                Section::Appearance => self.appearance_section(ui, p),
                Section::Network => self.network_section(ui, p),
                Section::System => self.system_section(ui, p),
            });
    }

    fn preview(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 84.0), Sense::hover());
        let painter = ui.painter().clone();

        let strip = if p.canvas.r() > 128 {
            Color32::from_rgb(226, 232, 240)
        } else {
            Color32::from_rgb(8, 13, 23)
        };
        painter.rect_filled(rect, CornerRadius::same(12), strip);
        painter.rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, p.hairline),
            egui::StrokeKind::Inside,
        );

        let centre = rect.center();
        let chevron_x = rect.right() - 20.0;
        let chevron = Stroke::new(1.6, p.muted);
        painter.line_segment(
            [
                egui::pos2(chevron_x - 4.0, centre.y - 3.0),
                egui::pos2(chevron_x, centre.y + 3.0),
            ],
            chevron,
        );
        painter.line_segment(
            [
                egui::pos2(chevron_x + 4.0, centre.y - 3.0),
                egui::pos2(chevron_x, centre.y + 3.0),
            ],
            chevron,
        );

        let sample = Sample {
            down_bps: 3_840_000.0,
            up_bps: 645_000.0,
            connected: true,
            adapter: String::new(),
        };
        let lines = display::meter_lines(&self.draft, &sample);
        let texts = match self.draft.layout {
            Layout::Horizontal => vec![lines.join(display::GAP)],
            Layout::Vertical => lines,
        };
        if texts.is_empty() {
            return;
        }

        let family = if self
            .draft
            .font_family
            .to_ascii_lowercase()
            .contains("semibold")
        {
            FontFamily::Name(DISPLAY_FAMILY.into())
        } else {
            FontFamily::Proportional
        };
        let font = FontId::new(self.draft.font_size as f32, family);
        let rgb = parse_color(&self.draft.text_color);
        let text_color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);

        let galleys: Vec<_> = texts
            .iter()
            .map(|text| painter.layout_no_wrap(text.clone(), font.clone(), text_color))
            .collect();
        let widest = galleys
            .iter()
            .map(|galley| galley.size().x)
            .fold(0.0_f32, f32::max);
        let line_height = font.size * 1.32;
        let block_height = line_height * texts.len() as f32;
        let right = rect.right() - 34.0;

        if self.draft.background == Background::Solid {
            let bg = parse_color(&self.draft.background_color);
            let fill =
                Color32::from_rgba_unmultiplied(bg[0], bg[1], bg[2], self.draft.background_opacity);
            let pill = Rect::from_min_size(
                egui::pos2(right - widest - 12.0, centre.y - block_height / 2.0 - 7.0),
                Vec2::new(widest + 24.0, block_height + 14.0),
            );
            painter.rect_filled(pill, CornerRadius::same(9), fill);
        }

        for (index, text) in texts.iter().enumerate() {
            let y = centre.y - block_height / 2.0 + line_height * (index as f32 + 0.5);
            painter.text(
                egui::pos2(right, y),
                Align2::RIGHT_CENTER,
                text,
                font.clone(),
                text_color,
            );
        }
    }

    fn display_section(&mut self, ui: &mut egui::Ui, p: &Palette) {
        row(ui, "Meter", p, |ui| {
            ui.checkbox(&mut self.draft.visible, "Show in the taskbar");
        });
        row(ui, "Readouts", p, |ui| {
            ui.checkbox(&mut self.draft.show_download, "Download");
            ui.checkbox(&mut self.draft.show_upload, "Upload");
        });
        row(ui, "Layout", p, |ui| {
            ui.radio_value(&mut self.draft.layout, Layout::Horizontal, "One line");
            ui.radio_value(&mut self.draft.layout, Layout::Vertical, "Two lines");
        });
        row(ui, "Units", p, |ui| {
            egui::ComboBox::from_id_salt("units")
                .selected_text(match self.draft.unit {
                    Unit::Bits => "Bits (Kbps / Mbps)",
                    Unit::Bytes => "Bytes (KB/s / MB/s)",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.unit, Unit::Bits, "Bits (Kbps / Mbps)");
                    ui.selectable_value(&mut self.draft.unit, Unit::Bytes, "Bytes (KB/s / MB/s)");
                });
        });
        row(ui, "Refresh", p, |ui| {
            ui.add(
                egui::Slider::new(&mut self.draft.update_interval_ms, 250..=5000)
                    .step_by(250.0)
                    .suffix(" ms"),
            );
        });
    }

    fn appearance_section(&mut self, ui: &mut egui::Ui, p: &Palette) {
        row(ui, "Theme", p, |ui| {
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
        });
        row(ui, "Font", p, |ui| {
            ui.add_sized(
                [200.0, 24.0],
                egui::TextEdit::singleline(&mut self.draft.font_family),
            );
        });
        row(ui, "Size", p, |ui| {
            ui.add(
                egui::Slider::new(&mut self.draft.font_size, 8..=24)
                    .suffix(" px")
                    .fixed_decimals(0),
            );
        });
        row(ui, "Text colour", p, |ui| {
            let mut rgb = parse_color(&self.draft.text_color);
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                self.draft.text_color = format_color(rgb);
            }
            ui.label(
                RichText::new(self.draft.text_color.to_uppercase())
                    .size(11.5)
                    .color(p.muted),
            );
        });
        row(ui, "Background", p, |ui| {
            let mut solid = self.draft.background == Background::Solid;
            if ui.checkbox(&mut solid, "Rounded pill").changed() {
                self.draft.background = if solid {
                    Background::Solid
                } else {
                    Background::Transparent
                };
            }
            if solid {
                let mut rgb = parse_color(&self.draft.background_color);
                if ui.color_edit_button_srgb(&mut rgb).changed() {
                    self.draft.background_color = format_color(rgb);
                }
                ui.add(
                    egui::Slider::new(&mut self.draft.background_opacity, 0..=255).text("opacity"),
                );
            }
        });
        row(ui, "Position", p, |ui| {
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
            ui.add(egui::Slider::new(&mut self.draft.offset, 0..=40).text("gap"));
        });
    }

    fn network_section(&mut self, ui: &mut egui::Ui, p: &Palette) {
        row(ui, "Adapter", p, |ui| {
            egui::ComboBox::from_id_salt("adapter")
                .width(300.0)
                .selected_text(self.adapter_label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.adapter, None, "All active adapters");
                    for entry in &self.adapters {
                        let label = format!(
                            "{} — {} ({})",
                            entry.name,
                            entry.kind,
                            if entry.up {
                                "connected"
                            } else {
                                "disconnected"
                            }
                        );
                        ui.selectable_value(
                            &mut self.draft.adapter,
                            Some(entry.guid.clone()),
                            label,
                        );
                    }
                });
            if ui.button("Scan").clicked() {
                self.adapters = adapter::list();
            }
        });

        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{} physical interface{} detected. Filter drivers, VPN tunnels and Hyper-V switches are ignored.",
                self.adapters.len(),
                if self.adapters.len() == 1 { "" } else { "s" }
            ))
            .size(11.5)
            .color(p.muted),
        );
    }

    fn system_section(&mut self, ui: &mut egui::Ui, p: &Palette) {
        row(ui, "Startup", p, |ui| {
            let mut start = self.draft.start_with_windows;
            if ui.checkbox(&mut start, "Start with Windows").changed() {
                if let Err(error) = startup::set_enabled(start) {
                    self.status = format!("Startup entry: {error}");
                    self.status_ok = false;
                    start = startup::enabled();
                }
                self.draft.start_with_windows = start;
            }
        });
        row(ui, "Taskbar", p, |ui| {
            ui.radio_value(
                &mut self.draft.dock_mode,
                DockMode::Overlay,
                "Overlay (recommended)",
            );
            ui.radio_value(
                &mut self.draft.dock_mode,
                DockMode::Taskbar,
                "Dock into Explorer",
            );
        });
        ui.label(
            RichText::new(
                "Windows has no supported taskbar-toolbar API. Overlay draws a click-through window in the \
                 taskbar band; docking reparents a child window into Explorer and can break after updates.",
            )
            .size(11.5)
            .color(p.muted),
        );

        ui.add_space(18.0);
        ui.horizontal(|ui| {
            if ui.button("Reset to defaults").clicked() {
                self.draft = Config::default();
                let _ = startup::set_enabled(self.draft.start_with_windows);
                self.last_save = None;
            }
            if ui.button("Open settings folder").clicked() {
                if let (Some(explorer), Some(folder)) = (explorer_path(), config_path().parent()) {
                    let _ = std::process::Command::new(explorer).arg(folder).spawn();
                }
            }
        });

        ui.add_space(16.0);
        ui.label(
            RichText::new(format!(
                "NetPulse {} — changes apply immediately.",
                env!("CARGO_PKG_VERSION")
            ))
            .size(11.5)
            .color(p.muted),
        );
        ui.label(
            RichText::new(config_path().display().to_string())
                .size(11.0)
                .color(p.muted),
        );
    }
}

fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool, p: &Palette) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 32.0), Sense::click());
    let painter = ui.painter().clone();
    if selected {
        painter.rect_filled(rect, CornerRadius::same(8), p.surface);
        let bar = Rect::from_min_size(
            rect.left_top() + Vec2::new(0.0, 8.0),
            Vec2::new(2.5, rect.height() - 16.0),
        );
        painter.rect_filled(bar, CornerRadius::same(2), p.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::same(8), p.surface.gamma_multiply(0.55));
    }
    painter.text(
        rect.left_center() + Vec2::new(14.0, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::new(13.5, FontFamily::Proportional),
        if selected { p.ink } else { p.muted },
    );
    response
}

fn row<R>(ui: &mut egui::Ui, label: &str, p: &Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let result = ui
        .horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(132.0, 22.0),
                EguiLayout::left_to_right(Align::Center),
                |ui| {
                    ui.label(RichText::new(label).size(12.5).color(p.muted));
                },
            );
            add(ui)
        })
        .inner;
    ui.add_space(6.0);
    result
}

impl eframe::App for SettingsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        self.ensure_style(&context);
        let p = palette(self.dark());

        egui::Panel::left("netpulse-rail")
            .exact_size(RAIL_WIDTH)
            .resizable(false)
            .frame(
                Frame::default()
                    .fill(p.rail)
                    .inner_margin(Margin::symmetric(14, 18)),
            )
            .show(ui, |ui| self.rail(ui, &p));

        egui::CentralPanel::default()
            .frame(
                Frame::default()
                    .fill(p.canvas)
                    .inner_margin(Margin::symmetric(24, 18)),
            )
            .show(ui, |ui| self.content(ui, &p));

        self.flush(&context);
    }

    fn logic(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.reload_if_changed();
        context.request_repaint_after(Duration::from_secs(1));
    }
}
