//! Edit a wallpaper's title, description, author, category, tags and thumbnail.

use super::*;

/// Record `name` (a file in the package) as its thumbnail in sarab.json.
pub fn set_thumbnail(dir: &Path, name: &str) -> Result<(), String> {
    let mut info: Manifest =
        serde_json::from_slice(&fs::read(dir.join(INFO)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    info.thumbnail = Some(name.to_string());
    crate::core::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())
}

/// What the edit dialog sends.
#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct Edit {
    pub title: String,
    pub description: String,
    pub author: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
}

pub(in crate::library) fn optional(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Check an edit and write it to the wallpaper's sarab.json. Limits follow the editors other
/// wallpaper apps ship: a title of 1 to 100 characters, up to 5 tags of up to 20 each.
pub fn edit_info(dir: &Path, e: Edit) -> Result<Manifest, String> {
    let mut info: Manifest =
        serde_json::from_slice(&fs::read(dir.join(INFO)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let title = e.title.trim();
    let len = |s: &str| s.chars().count();
    if title.is_empty() || len(title) > 100 {
        return Err("the title needs 1 to 100 characters".into());
    }
    if len(e.description.trim()) > 1000 || len(e.author.trim()) > 100 {
        return Err("the description is limited to 1000 characters and the author to 100".into());
    }
    let category = e.category.as_deref().and_then(optional);
    if let Some(c) = &category {
        if !CATEGORIES.contains(&c.as_str()) {
            return Err(format!("unknown category {c}"));
        }
    }
    let mut tags: Vec<String> = vec![];
    for t in e.tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        if len(t) > 20 {
            return Err(format!("the tag \"{t}\" is longer than 20 characters"));
        }
        if !tags.iter().any(|x| x.eq_ignore_ascii_case(t)) {
            tags.push(t.to_string());
        }
    }
    if tags.len() > 5 {
        return Err("use at most 5 tags".into());
    }
    info.title = Some(title.to_string());
    info.description = optional(&e.description);
    info.author = optional(&e.author);
    info.category = category;
    info.tags = tags;
    info.version += 1;
    crate::core::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())?;
    Ok(info)
}
