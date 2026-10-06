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
    pub pause_cpu: u8,
    pub app_pause: Vec<String>,
    pub app_play: Vec<String>,
    pub fps: u32,
    pub volume: u8,
    pub audio_desktop_only: bool,
    pub audio_mute_others: bool,
    pub scaling: String,
    pub cycle_minutes: u32,
    pub screensaver_minutes: u32,
    pub screensaver_wallpaper: Option<String>,
    pub span: bool,
    pub mouse_input: bool,
    pub keep_frame_on_quit: bool,
    pub cycle_order: String,
    pub cycle_category: String,
    pub unload_minutes: u32,
    pub language: String,
    pub theme: String,
    pub backdrop: String,
    pub check_updates: bool,
    pub update_channel: String,
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
            pause_cpu: 0,
            app_pause: vec![],
            app_play: vec![],
            fps: 30,
            volume: 0,
            audio_desktop_only: false,
            audio_mute_others: true,
            scaling: "cover".into(),
            cycle_minutes: 0,
            screensaver_minutes: 0,
            screensaver_wallpaper: None,
            span: false,
            mouse_input: false,
            keep_frame_on_quit: false,
            cycle_order: "order".into(),
            cycle_category: "all".into(),
            unload_minutes: if crate::os::windows::total_ram() < 8 << 30 {
                5
            } else {
                0
            },
            language: "en".into(),
            theme: "system".into(),
            backdrop: "acrylic".into(),
            check_updates: true,
            update_channel: "stable".into(),
            autostart_set: false,
        }
    }
}

pub type Layout = BTreeMap<String, String>;

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

pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(tmp, path)
}
