pub mod presets;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub mod import;
pub use import::*;

mod manage;
pub use manage::*;

#[cfg(test)]
mod tests;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    Web,
    Url,
    Video,
    Gif,
    Picture,
    App,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Web => "web",
            Kind::Url => "url",
            Kind::Video => "video",
            Kind::Gif => "gif",
            Kind::Picture => "picture",
            Kind::App => "app",
        }
    }
}

/// `sarab.json`: the package manifest at the root of every wallpaper folder.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Manifest {
    pub title: Option<String>,
    pub description: Option<String>,
    /// The title and description in other languages, by language code ("ar", "fr"...).
    /// `title` and `description` stay the fallback.
    pub titles: std::collections::BTreeMap<String, String>,
    pub descriptions: std::collections::BTreeMap<String, String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub r#type: Kind,
    /// Relative to the package folder, or an absolute path when `external` is true, or a URL.
    pub file: Option<String>,
    /// The file lives outside the package folder (added from disk, not copied).
    pub external: bool,
    pub thumbnail: Option<String>,
    /// Data the page asks Sarab for: "system" (sarabSystemInfo once a second) and
    /// "nowplaying" (sarabNowPlaying when the track changes), and "audio" (sarabAudio, 128
    /// levels about 30 times a second).
    pub api: Vec<String>,
    /// One of `CATEGORIES`, for the library filter.
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub version: u32,
    /// The Sarab version the package was made for. Older versions flag it and ask before playing.
    pub app_version: Option<String>,
    /// Play only this range of a video, in seconds, and loop inside it.
    pub clip: Option<[f64; 2]>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Wallpaper {
    pub id: String,
    pub dir: PathBuf,
    pub info: Manifest,
    pub kind: &'static str,
    pub has_props: bool,
    /// Ships with Sarab (read-only, cannot be deleted).
    pub preset: bool,
    /// When the folder was created, in Unix seconds, for "newest first".
    pub added: u64,
    /// The thumbnail image, when the package has one.
    pub thumb: Option<PathBuf>,
    /// Made for a newer Sarab than this one.
    pub too_new: bool,
}

pub enum Target {
    File(PathBuf),
    Url(String),
}

impl Wallpaper {
    pub fn target(&self) -> Option<Target> {
        let f = self.info.file.as_deref()?;
        Some(match self.info.r#type {
            Kind::Url => Target::Url(f.to_string()),
            _ if self.info.external => Target::File(f.into()),
            _ => Target::File(self.dir.join(f)),
        })
    }
    pub fn props_path(&self) -> PathBuf {
        self.dir.join(PROPS)
    }
}

pub const INFO: &str = "sarab.json";

/// Controls a wallpaper exposes to the user: slider, checkbox, dropdown, color, textbox, button, label, folderDropdown.
pub const PROPS: &str = "properties.json";

/// The categories a wallpaper can be filed under. Keys into `category.*` UI strings.
pub const CATEGORIES: [&str; 10] = [
    "nature", "space", "abstract", "city", "animals", "anime", "games", "vehicles", "minimal",
    "other",
];
