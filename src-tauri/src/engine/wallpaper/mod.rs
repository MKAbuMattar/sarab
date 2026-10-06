//! Core state: which wallpaper runs on which display, and in which pause state.
//! Every function here runs on the main thread (see `on_main`), so the lock is never contended.

use crate::core::pause::{self, Reason, Signals, State};
use crate::core::settings::{self, Layout, Settings};
use crate::library::{self, Kind, Target, Wallpaper};
use crate::os::windows as os;
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::Foundation::RECT;

mod app;
use app::*;
mod apply;
pub use apply::*;
mod capture;
pub use capture::*;
mod cpu;
use cpu::*;
mod cycle;
pub use cycle::*;
mod displays;
pub use displays::*;
mod mouse;
use mouse::*;
mod page;
pub use page::*;
mod page_feeds;
use page_feeds::*;
mod playback;
pub use playback::*;
mod props;
pub use props::*;
mod screensaver;
use screensaver::*;
mod span;
use span::*;
mod status;
pub use status::*;
mod sync;
use sync::*;
mod tick;
pub use tick::*;
mod unload;
use unload::*;
mod urls;
pub use urls::*;

#[cfg(test)]
mod tests;

mod labels;
use labels::*;
mod log;
pub use log::*;
mod paths;
pub use paths::*;
mod threads;
pub use threads::*;

pub const INJECT: &str = include_str!("../../scripts/inject.js");

#[cfg(windows)]
const APP_ORIGIN: &str = "http://tauri.localhost";

pub struct Display {
    pub mon: os::Monitor,
    pub wallpaper: Option<String>,
    pub state: State,
    pub reason: Reason,
    pub loaded: bool,
    pub error: Option<String>,
    /// Filled by `sarab status`: what the page itself reports.
    pub page: Value,
    /// Label of this display's wallpaper window. Unique per window: `destroy()` frees a label
    /// asynchronously, so reusing one for the replacement window fails.
    pub label: Option<String>,
    /// Since when nobody can see this display's wallpaper (covered, locked, remote).
    pub unseen_since: Option<std::time::Instant>,
    /// Its webview was closed to free memory; it loads again when it would play.
    pub unloaded: bool,
    /// The program an app wallpaper runs; dropping it ends the program.
    pub app: Option<os::AppProcess>,
}

pub struct Core {
    pub settings: Settings,
    pub layout: Layout,
    pub lib: Vec<Wallpaper>,
    pub displays: Vec<Display>,
    pub manual: Option<bool>,
    pub desktop: Option<os::Desktop>,
    /// Display key -> the OS wallpaper before Sarab changed it, so close can put it back.
    pub restore: BTreeMap<String, String>,
    pub signals: Signals,
    /// Run video sync on the next tick (a display just resumed or loaded).
    pub sync_due: bool,
    pub ticks: u64,
    /// Started the first time a playing wallpaper asks for system information.
    pub sysinfo: Option<crate::engine::feeds::Sampler>,
    /// The screensaver's window labels and the input stamp it started at, while it shows.
    pub screensaver: Option<(Vec<String>, u32)>,
    /// Where the mouse hook sends input, and whether it runs.
    pub mouse_targets: os::Targets,
    pub mouse_on: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// CPU counters at the last tick (idle, busy, Sarab's own) and the gate that steadies them.
    pub cpu_prev: Option<(u64, u64, u64)>,
    pub cpu_gate: pause::CpuGate,
    /// Runs while a playing wallpaper asks for the audio levels.
    pub audio: Option<crate::engine::audio::Feed>,
    /// Labels of the webviews the audio thread sends to.
    pub audio_to: std::sync::Arc<Mutex<Vec<String>>>,
    /// Started the first time a playing wallpaper asks for the current track.
    pub now_playing: Option<crate::engine::feeds::NowPlaying>,
    /// The volume the pages play at now, after the audio rules; the setting is the most it can be.
    pub volume_now: u8,
    /// When the wallpaper last changed, by the user or by cycling.
    pub changed_at: std::time::Instant,
}

pub type Shared = Mutex<Core>;

/// Folder of the web scenes that ship with Sarab, set once at startup from the resource dir.
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
