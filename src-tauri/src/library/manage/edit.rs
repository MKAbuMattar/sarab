use super::*;

pub fn set_thumbnail(dir: &Path, name: &str) -> Result<(), String> {
    let mut info: Manifest =
        serde_json::from_slice(&fs::read(dir.join(INFO)).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    info.thumbnail = Some(name.to_string());
    crate::core::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
pub struct Edit {
    pub title: String,
    pub description: String,
    pub author: String,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub clip: Option<Vec<f64>>,
    pub thumbnail: Option<String>,
    pub thumbnail_time: Option<f64>,
}

pub const MIN_CLIP: f64 = 0.5;

pub(in crate::library) fn optional(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

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
    let thumbnail_choice = match e.thumbnail.as_deref() {
        None => info.thumbnail_choice.clone(),
        Some("auto") => None,
        Some("image") if custom_thumbnail(dir).is_some() => Some("image".to_string()),
        Some("image") => return Err("choose an image for the thumbnail first".into()),
        Some("frame") if frame_thumbnail(dir).is_none() => {
            return Err("use a frame of the video for the thumbnail first".into())
        }
        Some("frame") => Some("frame".to_string()),
        Some(other) => return Err(format!("unknown thumbnail choice {other}")),
    };
    let thumbnail_time = match (thumbnail_choice.as_deref(), e.thumbnail_time) {
        (Some("frame"), Some(t)) if t.is_finite() && t >= 0.0 => Some(t),
        (Some("frame"), None) => info.thumbnail_time,
        (Some("frame"), Some(_)) => return Err("the frame time must be 0 or more".into()),
        _ => info.thumbnail_time,
    };
    let clip = match e.clip.as_deref() {
        None => info.clip,
        Some([]) => None,
        Some(&[a, b]) if info.r#type == Kind::Video && a.is_finite() && b.is_finite() && a >= 0.0 && b - a >= MIN_CLIP => {
            Some([a, b])
        }
        Some(_) => {
            return Err("the part that plays needs a start of 0 or more and an end at least half a second later".into())
        }
    };
    info.title = Some(title.to_string());
    info.description = optional(&e.description);
    info.author = optional(&e.author);
    info.category = category;
    info.tags = tags;
    info.clip = clip;
    info.thumbnail_choice = thumbnail_choice;
    info.thumbnail_time = thumbnail_time;
    info.version += 1;
    crate::core::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())?;
    Ok(info)
}
