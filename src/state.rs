use std::sync::atomic::{AtomicBool, AtomicIsize};
use std::sync::{Arc, RwLock};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

use crate::settings::Config;

pub const HOST_CLASS: &str = "NetPulseHost";
pub const METER_CLASS: &str = "NetPulseMeter";

pub const WM_SPEED_UPDATED: u32 = WM_APP + 1;
pub const WM_CONFIG_CHANGED: u32 = WM_APP + 2;
pub const WM_EXIT: u32 = WM_APP + 3;
pub const WM_TOGGLE_VISIBLE: u32 = WM_APP + 4;

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub down_bps: f64,
    pub up_bps: f64,
    pub connected: bool,
    pub adapter: String,
}

pub struct Shared {
    pub config: RwLock<Config>,
    pub sample: RwLock<Sample>,
    pub host: AtomicIsize,
    pub exit: AtomicBool,
}

impl Shared {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config: RwLock::new(config),
            sample: RwLock::new(Sample::default()),
            host: AtomicIsize::new(0),
            exit: AtomicBool::new(false),
        })
    }
}
