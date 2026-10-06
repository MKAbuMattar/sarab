//! Where Sarab keeps things: config files, the library, saved Customize values.

use crate::core::settings::{self, Settings};
use crate::engine::wallpaper::PRESET_DIR;
use crate::library::{self, Wallpaper};
use std::path::PathBuf;

pub fn cfg(name: &str) -> PathBuf {
    settings::config_dir().join(name)
}

pub fn library_dir(s: &Settings) -> PathBuf {
    s.library_dir
        .clone()
        .unwrap_or_else(|| settings::data_dir().join("Library"))
}

/// The user's library followed by the bundled presets.
pub fn scan_all(settings: &Settings) -> Vec<Wallpaper> {
    let mut lib = library::scan(&library_dir(settings));
    if let Some(dir) = PRESET_DIR.get() {
        lib.extend(library::scan(dir).into_iter().map(|mut w| {
            w.preset = true;
            w
        }));
    }
    lib
}

pub(crate) fn key_file(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

pub fn saved_props_path(id: &str, key: &str) -> PathBuf {
    cfg("props")
        .join(id)
        .join(format!("{}.json", key_file(key)))
}
