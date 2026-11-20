use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    Bits,
    Bytes,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    Transparent,
    Solid,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    LeftOfTray,
    RightOfTray,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DockMode {
    /// Always-on-top, click-through window placed over the taskbar band.
    Overlay,
    /// Child window reparented into Explorer's taskbar (unsupported by Microsoft).
    Taskbar,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    /// `↓ 1.2 MB/s  ↑ 240 KB/s` on one line.
    Horizontal,
    /// Download on the first line, upload below it.
    Vertical,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Config {
    pub visible: bool,
    pub show_download: bool,
    pub show_upload: bool,
    pub unit: Unit,
    pub layout: Layout,
    pub font_family: String,
    pub font_size: u32,
    pub text_color: String,
    pub background: Background,
    pub background_color: String,
    pub background_opacity: u8,
    pub position: Position,
    pub offset: i32,
    pub update_interval_ms: u64,
    pub adapter: Option<String>,
    pub dock_mode: DockMode,
    pub theme: Theme,
    pub start_with_windows: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            visible: true,
            show_download: true,
            show_upload: true,
            unit: Unit::Bytes,
            layout: Layout::Horizontal,
            font_family: "Segoe UI Semibold".to_string(),
            font_size: 16,
            text_color: "#FFFFFF".to_string(),
            background: Background::Transparent,
            background_color: "#1F1F1F".to_string(),
            background_opacity: 210,
            position: Position::LeftOfTray,
            offset: 2,
            update_interval_ms: 1000,
            adapter: None,
            dock_mode: DockMode::Overlay,
            theme: Theme::System,
            start_with_windows: true,
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("NetPulse").join("config.json")
}

/// The taskbar follows the Windows (not app) theme on 10 and 11.
pub fn system_uses_light_theme() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize",
            KEY_READ,
        )
        .and_then(|key| key.get_value::<u32, _>("SystemUsesLightTheme"))
        .map(|value| value == 1)
        .unwrap_or(false)
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        // Refuse oversized config files before handing them to the JSON parser:
        // a tampered file must not be able to exhaust memory at startup.
        if fs::metadata(&path)
            .map(|meta| meta.len() > 64 * 1024)
            .unwrap_or(false)
        {
            return Self::recovered();
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            // No file yet: apply defaults, including first-run autostart.
            Err(_) => return Self::first_run(),
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(config) => config.sanitized(),
            // Corrupt file: keep the existing autostart state instead of
            // re-enabling it through the default value.
            Err(_) => Self::recovered(),
        }
    }

    /// Clamps values coming from the on-disk JSON into the ranges the UI and
    /// renderer actually support. The UI already constrains these ranges, but
    /// the config file is user-editable, so it must not be trusted blindly.
    fn sanitized(mut self) -> Self {
        self.font_size = self.font_size.clamp(8, 24);
        self.update_interval_ms = self.update_interval_ms.clamp(250, 5000);
        self.offset = self.offset.clamp(0, 40);
        self.font_family = self.font_family.chars().take(32).collect();
        self.text_color = self.text_color.chars().take(16).collect();
        self.background_color = self.background_color.chars().take(16).collect();
        if let Some(adapter) = &mut self.adapter {
            *adapter = adapter.chars().take(128).collect();
        }
        self
    }

    /// Defaults with a text colour that is readable on the current taskbar.
    fn first_run() -> Self {
        let mut config = Self::default();
        if system_uses_light_theme() {
            config.text_color = "#1A1A1A".to_string();
        }
        config
    }

    /// Used when the config file exists but cannot be parsed. Preserve the
    /// current autostart state so a corrupted file cannot silently re-add the
    /// HKCU Run entry the user may have deliberately removed.
    fn recovered() -> Self {
        let mut config = Self::first_run();
        config.start_with_windows = crate::startup::enabled();
        config
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, json)?;
        fs::rename(&tmp, &path)
    }
}
