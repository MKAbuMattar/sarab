use crate::core::pause::{self, Reason, Signals, State};
use crate::core::settings::{self, Layout, Settings};
use crate::library::{self, Kind, Target, Wallpaper};
use crate::os::platform as os;
use crate::os::platform::RECT;
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

mod display;
pub use display::*;

mod page;
pub use page::*;

mod schedule;
pub use schedule::*;

mod output;
pub use output::*;

#[cfg(test)]
mod tests;

pub use crate::common::*;

pub const INJECT: &str = include_str!("../../scripts/inject.js");

#[cfg(windows)]
const APP_ORIGIN: &str = "http://tauri.localhost";
#[cfg(not(windows))]
const APP_ORIGIN: &str = "tauri://localhost";

pub struct Display {
    pub mon: os::Monitor,
    pub wallpaper: Option<String>,
    pub state: State,
    pub reason: Reason,
    pub loaded: bool,
    pub error: Option<String>,
    pub page: Value,
    pub label: Option<String>,
    pub unseen_since: Option<std::time::Instant>,
    pub unloaded: bool,
    pub app: Option<os::AppProcess>,
}

pub struct Core {
    pub settings: Settings,
    pub layout: Layout,
    pub lib: Vec<Wallpaper>,
    pub displays: Vec<Display>,
    pub manual: Option<bool>,
    pub desktop: Option<os::Desktop>,
    pub restore: BTreeMap<String, String>,
    pub signals: Signals,
    pub sync_due: bool,
    pub ticks: u64,
    pub sysinfo: Option<crate::engine::feeds::Sampler>,
    pub screensaver: Option<(Vec<String>, u32)>,
    pub mouse_targets: os::Targets,
    pub mouse_on: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    pub cpu_prev: Option<(u64, u64, u64)>,
    pub cpu_gate: pause::CpuGate,
    pub audio: Option<crate::engine::audio::Feed>,
    pub audio_to: std::sync::Arc<Mutex<Vec<String>>>,
    pub now_playing: Option<crate::engine::feeds::NowPlaying>,
    pub volume_now: u8,
    pub changed_at: std::time::Instant,
}

pub type Shared = Mutex<Core>;

pub static PRESET_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

impl Core {
    pub fn load() -> Core {
        let settings: Settings = settings::load(&cfg("settings.json"));
        let lib = scan_all(&settings);
        let volume_now = settings.volume;
        Core {
            layout: settings::load(&cfg("layout.json")),
            restore: settings::load(&cfg("restore.json")),
            settings,
            lib,
            displays: vec![],
            manual: None,
            desktop: None,
            signals: Signals::default(),
            sync_due: false,
            ticks: 0,
            volume_now,
            sysinfo: None,
            now_playing: None,
            audio: None,
            cpu_prev: None,
            cpu_gate: Default::default(),
            mouse_targets: Default::default(),
            mouse_on: None,
            screensaver: None,
            audio_to: Default::default(),
            changed_at: std::time::Instant::now(),
        }
    }
    pub fn find(&self, id: &str) -> Option<&Wallpaper> {
        self.lib.iter().find(|w| w.id == id)
    }
    fn save_layout(&self) {
        if let Err(e) = settings::save(&cfg("layout.json"), &self.layout) {
            log(format!("save layout: {e}"));
        }
    }
    fn save_restore(&self) {
        if let Err(e) = settings::save(&cfg("restore.json"), &self.restore) {
            log(format!("save restore: {e}"));
        }
    }
}
