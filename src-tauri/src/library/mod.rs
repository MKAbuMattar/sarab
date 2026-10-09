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

#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
#[serde(default)]
pub struct Manifest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub titles: std::collections::BTreeMap<String, String>,
    pub descriptions: std::collections::BTreeMap<String, String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub r#type: Kind,
    pub file: Option<String>,
    pub external: bool,
    pub thumbnail: Option<String>,
    pub api: Vec<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub version: u32,
    pub app_version: Option<String>,
    pub clip: Option<[f64; 2]>,
    pub thumbnail_choice: Option<String>,
    pub thumbnail_time: Option<f64>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Wallpaper {
    pub id: String,
    pub dir: PathBuf,
    pub info: Manifest,
    pub kind: &'static str,
    pub has_props: bool,
    pub preset: bool,
    pub added: u64,
    pub thumb: Option<PathBuf>,
    pub auto_thumb: Option<PathBuf>,
    pub custom_thumb: Option<PathBuf>,
    pub frame_thumb: Option<PathBuf>,
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

pub const PROPS: &str = "properties.json";

pub const CATEGORIES: [&str; 10] = [
    "nature", "space", "abstract", "city", "animals", "anime", "games", "vehicles", "minimal",
    "other",
];
