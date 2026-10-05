use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub library_dir: Option<PathBuf>,
    pub pause_fullscreen: bool,
    pub pause_focus: bool,
    pub pause_battery: bool,
    pub pause_power_saver: bool,
    pub pause_remote: bool,
    pub per_display: bool,
    pub app_pause: Vec<String>,
    pub app_play: Vec<String>,
    pub fps: u32,
    pub volume: u8,
    /// Mute the wallpaper whenever an app has the focus instead of the desktop.
    pub audio_desktop_only: bool,
    /// Mute the wallpaper while another app plays sound.
    pub audio_mute_others: bool,
    /// How videos and GIFs fill the display: "cover", "contain", "fill" or "none".
    pub scaling: String,
    /// Move to the next wallpaper in the library every this many minutes; 0 is never.
    pub cycle_minutes: u32,
    /// Cycling and "next" go through the library "order"ly or at "random".
    pub cycle_order: String,
    /// Only wallpapers in this category take part in cycling and "next"; "all" for every one.
    pub cycle_category: String,
    /// Unload a wallpaper nobody can see (covered, locked, remote) after this many minutes, to
    /// free its memory; it loads again when it would play. 0 is never.
    pub unload_minutes: u32,
    pub language: String,
    /// "system", "light" or "dark" for the settings window.
    pub theme: String,
    /// Settings window backdrop: "acrylic" (see-through), "mica", or "solid".
    pub backdrop: String,
    /// Look for a newer release once a day. Nothing downloads without the user's consent.
    pub check_updates: bool,
    /// Start with Windows has been switched on once (at first launch); never forced again.
    pub autostart_set: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            library_dir: None,
            pause_fullscreen: true,
            pause_focus: false,
            pause_battery: true,
            pause_power_saver: true,
            pause_remote: true,
            per_display: true,
            app_pause: vec![],
            app_play: vec![],
            fps: 30,
            volume: 0,
            audio_desktop_only: false,
            audio_mute_others: true,
            scaling: "cover".into(),
            cycle_minutes: 0,
            cycle_order: "order".into(),
            cycle_category: "all".into(),
            // On by default where memory is tight (SPEC F17).
            unload_minutes: if crate::os::windows::total_ram() < 8 << 30 {
                5
            } else {
                0
            },
            language: "en".into(),
            theme: "system".into(),
            backdrop: "acrylic".into(),
            check_updates: true,
            autostart_set: false,
        }
    }
}

/// display key (e.g. `\\.\DISPLAY1`) -> wallpaper id
pub type Layout = BTreeMap<String, String>;

/// Named after the app identifier, as Tauri does: the installer's "delete app data" option removes
/// these, and they never collide with the per-user install folder (%LOCALAPPDATA%\Sarab).
pub const APP_ID: &str = "com.mkabumattar.sarab";

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into());
    base.join(APP_ID)
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into());
    base.join(APP_ID)
}

pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Write to a temp file, then rename, so a crash mid-write never leaves a half file.
pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)
}
