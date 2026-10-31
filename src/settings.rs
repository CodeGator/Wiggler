use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const INTERVAL_MIN: u32 = 30;
pub const INTERVAL_MAX: u32 = 300;
pub const INTERVAL_STEP: u32 = 15;
pub const DEFAULT_INTERVAL: u32 = 120;
pub const TIME_MIN: u32 = 0;
pub const TIME_MAX: u32 = 23 * 60 + 45;
pub const TIME_STEP: u32 = 15;
pub const DEFAULT_QUIET_AFTER_MINUTES: u32 = 18 * 60;

fn default_true() -> bool {
    true
}

fn default_quiet_after_minutes() -> u32 {
    DEFAULT_QUIET_AFTER_MINUTES
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub interval_seconds: u32,
    #[serde(default = "default_true")]
    pub idle_only: bool,
    #[serde(default = "default_true")]
    pub skip_when_locked: bool,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub quiet_after: bool,
    #[serde(default = "default_quiet_after_minutes")]
    pub quiet_after_minutes: u32,
    #[serde(default)]
    pub skip_weekends: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_seconds: DEFAULT_INTERVAL,
            idle_only: true,
            skip_when_locked: true,
            running: false,
            auto_start: false,
            quiet_after: false,
            quiet_after_minutes: DEFAULT_QUIET_AFTER_MINUTES,
            skip_weekends: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let Ok(text) = fs::read_to_string(load_path()) else {
            return Self::default();
        };
        let Ok(mut loaded) = serde_json::from_str::<Settings>(&text) else {
            return Self::default();
        };
        loaded.interval_seconds = snap_interval(loaded.interval_seconds);
        loaded.quiet_after_minutes = snap_time(loaded.quiet_after_minutes);
        loaded
    }

    pub fn save(&self) {
        let path = save_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let body = Settings {
            interval_seconds: snap_interval(self.interval_seconds),
            idle_only: self.idle_only,
            skip_when_locked: self.skip_when_locked,
            running: self.running,
            auto_start: self.auto_start,
            quiet_after: self.quiet_after,
            quiet_after_minutes: snap_time(self.quiet_after_minutes),
            skip_weekends: self.skip_weekends,
        };
        if let Ok(text) = serde_json::to_string_pretty(&body) {
            let _ = fs::write(path, text + "\n");
        }
    }
}

pub fn snap_interval(seconds: u32) -> u32 {
    snap_step(seconds, INTERVAL_MIN, INTERVAL_MAX, INTERVAL_STEP)
}

pub fn snap_time(minutes: u32) -> u32 {
    snap_step(minutes, TIME_MIN, TIME_MAX, TIME_STEP)
}

pub fn format_clock(minutes: u32) -> String {
    let minutes = snap_time(minutes);
    let hour24 = minutes / 60;
    let min = minutes % 60;
    let (hour12, suffix) = match hour24 {
        0 => (12, "AM"),
        1..=11 => (hour24, "AM"),
        12 => (12, "PM"),
        _ => (hour24 - 12, "PM"),
    };
    format!("{hour12}:{min:02} {suffix}")
}

fn snap_step(value: u32, min: u32, max: u32, step: u32) -> u32 {
    let clamped = value.clamp(min, max);
    min + ((clamped - min) / step) * step
}

fn appdata() -> PathBuf {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn save_path() -> PathBuf {
    appdata().join("Wiggler").join("settings.json")
}

fn load_path() -> PathBuf {
    let current = save_path();
    if current.exists() {
        return current;
    }
    let legacy = appdata().join("Wiggle").join("settings.json");
    if legacy.exists() {
        return legacy;
    }
    current
}
