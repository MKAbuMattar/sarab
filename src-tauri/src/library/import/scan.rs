use super::*;

pub fn scan(lib: &Path) -> Vec<Wallpaper> {
    let mut out: Vec<Wallpaper> = fs::read_dir(lib)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| read(&e.path()))
        .collect();
    out.sort_by(|a, b| a.info.title.cmp(&b.info.title));
    out
}

pub fn read(dir: &Path) -> Option<Wallpaper> {
    let info: Manifest = serde_json::from_slice(&fs::read(dir.join(INFO)).ok()?).ok()?;
    let mut w = Wallpaper {
        id: dir.file_name()?.to_string_lossy().into_owned(),
        dir: dir.to_path_buf(),
        kind: info.r#type.name(),
        has_props: dir.join(PROPS).is_file(),
        preset: false,
        added: secs(fs::metadata(dir).ok().and_then(|m| m.created().ok())),
        too_new: info
            .app_version
            .as_deref()
            .is_some_and(|v| newer(v, env!("CARGO_PKG_VERSION"))),
        thumb: None,
        auto_thumb: info
            .thumbnail
            .as_deref()
            .map(|t| dir.join(t))
            .filter(|p| p.is_file() && p.starts_with(dir)),
        custom_thumb: crate::library::custom_thumbnail(dir),
        info,
    };
    w.thumb = match w.info.thumbnail_choice.as_deref() {
        Some("image") => w.custom_thumb.clone().or_else(|| w.auto_thumb.clone()),
        _ => w.auto_thumb.clone(),
    };
    Some(w)
}

pub fn newer(a: &str, b: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parts(a) > parts(b)
}

pub(in crate::library) fn secs(t: Option<std::time::SystemTime>) -> u64 {
    t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}
