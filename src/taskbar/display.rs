use windows::Win32::Foundation::COLORREF;

use crate::settings::{Config, Unit};
use crate::state::Sample;

pub const DOWN_ARROW: &str = "\u{2193}";
pub const UP_ARROW: &str = "\u{2191}";
/// Space between the download and upload halves of a single-line meter.
pub const GAP: &str = "  ";

pub fn format_speed(bytes_per_second: f64, unit: Unit) -> String {
    let bytes = bytes_per_second.max(0.0);
    let (value, suffix) = match unit {
        Unit::Bits => {
            let bits = bytes * 8.0;
            if bits >= 1_000_000.0 {
                (bits / 1_000_000.0, "Mbps")
            } else {
                (bits / 1_000.0, "Kbps")
            }
        }
        Unit::Bytes => {
            if bytes >= 1_000_000.0 {
                (bytes / 1_000_000.0, "MB/s")
            } else {
                (bytes / 1_000.0, "KB/s")
            }
        }
    };
    format!("{} {suffix}", number(value))
}

fn number(value: f64) -> String {
    let text = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    // 12.50 -> 12.5, 8.00 -> 8
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn meter_lines(config: &Config, sample: &Sample) -> Vec<String> {
    if !sample.connected {
        return vec!["No network".to_string()];
    }
    let mut lines = Vec::new();
    if config.show_download {
        lines.push(format!(
            "{DOWN_ARROW} {}",
            format_speed(sample.down_bps, config.unit)
        ));
    }
    if config.show_upload {
        lines.push(format!(
            "{UP_ARROW} {}",
            format_speed(sample.up_bps, config.unit)
        ));
    }
    lines
}

pub fn tooltip(config: &Config, sample: &Sample) -> String {
    if !sample.connected {
        return "NetPulse\nNo network connection".to_string();
    }
    let mut lines = vec!["NetPulse".to_string()];
    if config.show_download {
        lines.push(format!(
            "{DOWN_ARROW} {}",
            format_speed(sample.down_bps, config.unit)
        ));
    }
    if config.show_upload {
        lines.push(format!(
            "{UP_ARROW} {}",
            format_speed(sample.up_bps, config.unit)
        ));
    }
    if !sample.adapter.is_empty() {
        lines.push(sample.adapter.clone());
    }
    lines.join("\n")
}

pub fn colorref(hex: &str) -> COLORREF {
    let digits = hex.trim().trim_start_matches('#');
    let value = u32::from_str_radix(digits, 16).unwrap_or(0x00FF_FFFF);
    let (r, g, b) = ((value >> 16) & 0xFF, (value >> 8) & 0xFF, value & 0xFF);
    COLORREF(r | (g << 8) | (b << 16))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speeds_scale_and_trim() {
        assert_eq!(format_speed(0.0, Unit::Bits), "0 Kbps");
        assert_eq!(format_speed(125_000.0, Unit::Bits), "1 Mbps");
        assert_eq!(format_speed(1_562_500.0, Unit::Bits), "12.5 Mbps");
        assert_eq!(format_speed(1_000_000.0, Unit::Bytes), "1 MB/s");
        assert_eq!(format_speed(125_000.0, Unit::Bytes), "125 KB/s");
        assert_eq!(format_speed(-5.0, Unit::Bits), "0 Kbps");
    }

    #[test]
    fn colour_round_trip() {
        assert_eq!(colorref("#FFD400").0, 0x0000_D4FF);
    }
}
