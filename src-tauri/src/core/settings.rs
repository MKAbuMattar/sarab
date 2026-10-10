use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Slot {
    pub at: String,
    pub wallpaper: String,
}

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
    pub pause_gpu: u8,
    pub pause_memory: u8,
    pub pause_network: u32,
    pub pause_vm: bool,
    pub lock_screen: bool,
    pub schedule: Vec<Slot>,
    pub display_playlists: BTreeMap<String, String>,
    pub display_fit: BTreeMap<String, String>,
    pub wallpaper_speed: BTreeMap<String, f64>,
    pub wallpaper_volume: BTreeMap<String, u8>,
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
            unload_minutes: if crate::os::platform::total_ram() < 8 << 30 {
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
            pause_gpu: 0,
            pause_memory: 0,
            pause_network: 0,
            pause_vm: false,
            lock_screen: false,
            schedule: vec![],
            display_playlists: BTreeMap::new(),
            display_fit: BTreeMap::new(),
            wallpaper_speed: BTreeMap::new(),
            wallpaper_volume: BTreeMap::new(),
        }
    }
}

pub type Layout = BTreeMap<String, String>;

pub const APP_ID: &str = "com.mkabumattar.sarab";

#[cfg(windows)]
fn base(windows: &str, _xdg: &str, _home: &str) -> PathBuf {
    std::env::var_os(windows)
        .map(PathBuf::from)
        .unwrap_or_else(|| ".".into())
}

#[cfg(not(windows))]
fn base(_windows: &str, xdg: &str, home: &str) -> PathBuf {
    xdg_dir(std::env::var_os(xdg), std::env::var_os("HOME"), home)
}

#[cfg(not(windows))]
fn xdg_dir(
    set: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    under_home: &str,
) -> PathBuf {
    set.filter(|v| !v.is_empty() && Path::new(v).is_absolute())
        .map(PathBuf::from)
        .or_else(|| home.map(|h| PathBuf::from(h).join(under_home)))
        .unwrap_or_else(|| ".".into())
}

pub fn config_dir() -> PathBuf {
    base("APPDATA", "XDG_CONFIG_HOME", ".config").join(APP_ID)
}

pub fn data_dir() -> PathBuf {
    base("LOCALAPPDATA", "XDG_DATA_HOME", ".local/share").join(APP_ID)
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

#[cfg(all(test, not(windows)))]
mod xdg_tests {
    use super::*;

    #[test]
    fn xdg_dirs() {
        let home = Some("/home/a".into());
        assert_eq!(
            xdg_dir(Some("/x/cfg".into()), home.clone(), ".config"),
            PathBuf::from("/x/cfg")
        );
        assert_eq!(
            xdg_dir(None, home.clone(), ".config"),
            PathBuf::from("/home/a/.config")
        );
        assert_eq!(
            xdg_dir(Some("".into()), home.clone(), ".local/share"),
            PathBuf::from("/home/a/.local/share")
        );
        assert_eq!(
            xdg_dir(Some("relative".into()), home, ".config"),
            PathBuf::from("/home/a/.config"),
            "XDG paths must be absolute"
        );
    }
}
