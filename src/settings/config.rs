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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            visible: true,
            show_download: true,
            show_upload: true,
            unit: Unit::Bits,
            font_family: "Segoe UI Semibold".to_string(),
            font_size: 12,
            text_color: "#FFFFFF".to_string(),
            background: Background::Transparent,
            background_color: "#1F1F1F".to_string(),
            background_opacity: 210,
            position: Position::LeftOfTray,
            offset: 6,
            update_interval_ms: 1000,
            adapter: None,
            dock_mode: DockMode::Overlay,
            theme: Theme::System,
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
fn taskbar_is_light() -> bool {
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
        let text = match fs::read_to_string(config_path()) {
            Ok(text) => text,
            Err(_) => return Self::first_run(),
        };
        serde_json::from_str(&text).unwrap_or_else(|_| Self::first_run())
    }

    /// Defaults with a text colour that is readable on the current taskbar.
    fn first_run() -> Self {
        let mut config = Self::default();
        if taskbar_is_light() {
            config.text_color = "#1A1A1A".to_string();
        }
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
