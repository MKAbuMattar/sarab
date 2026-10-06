//! Add a file, folder or web address to the library.

use super::*;

pub fn kind_for(target: &str) -> Option<Kind> {
    if target.starts_with("http://") || target.starts_with("https://") {
        return Some(Kind::Url);
    }
    let ext = Path::new(target)
        .extension()?
        .to_str()?
        .to_ascii_lowercase();
    Some(match ext.as_str() {
        "mp4" | "webm" | "mkv" | "mov" | "m4v" | "avi" | "wmv" | "ogv" | "mpg" | "mpeg" => {
            Kind::Video
        }
        "gif" => Kind::Gif,
        "jpg" | "jpeg" | "png" | "bmp" | "webp" | "jfif" | "tif" | "tiff" => Kind::Picture,
        "html" | "htm" => Kind::Web,
        "exe" => Kind::App,
        _ => return None,
    })
}

pub(in crate::library) fn new_dir(lib: &Path, title: &str) -> io::Result<PathBuf> {
    let clean: String = title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(32)
        .collect();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = lib.join(format!(
        "{}-{:x}",
        if clean.is_empty() {
            "wallpaper"
        } else {
            &clean
        },
        nanos & 0xffff_ffff
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Add a file, a folder, or a URL. Files and URLs are referenced where they are; folders are copied in.
pub fn add(lib: &Path, existing: &[Wallpaper], target: &str) -> Result<Wallpaper, String> {
    use crate::library::import::wallpaper_engine::{self as we, PROJECT};
    // A Wallpaper Engine project.json stands for its folder.
    let parent;
    let target = match Path::new(target).file_name() {
        Some(n) if n.eq_ignore_ascii_case(PROJECT) => {
            parent = Path::new(target)
                .parent()
                .map(dunce_str)
                .unwrap_or_default();
            parent.as_str()
        }
        _ => target,
    };
    let as_path = Path::new(target);
    if as_path.is_dir() {
        // A Wallpaper Engine folder is converted on the way in. The original is never changed:
        // the new sarab.json goes into the library copy.
        let converted = if read(as_path).is_none() && as_path.join(PROJECT).is_file() {
            Some(we::convert(as_path)?)
        } else {
            None
        };
        if converted.is_none() && read(as_path).is_none() {
            return Err(format!("{target} has no {INFO} or {PROJECT}"));
        }
        if converted.is_none() && as_path.starts_with(lib) {
            return read(as_path).ok_or_else(|| "unreadable".into());
        }
        // The copy is named after the source path, so adding the same folder again reuses it.
        let src = fs::canonicalize(as_path).map_err(|e| e.to_string())?;
        let title: String = as_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();
        let dir = lib.join(format!(
            "{title}-{:08x}",
            fnv(dunce_str(&src).to_lowercase().as_bytes())
        ));
        if let Some(w) = read(&dir) {
            return Ok(w);
        }
        copy_dir(as_path, &dir).map_err(|e| e.to_string())?;
        if let Some(c) = converted {
            let written = crate::core::settings::save(&dir.join(INFO), &c.info).and_then(|()| {
                if c.props.is_empty() {
                    Ok(())
                } else {
                    crate::core::settings::save(&dir.join(PROPS), &c.props)
                }
            });
            if let Err(e) = written {
                let _ = fs::remove_dir_all(&dir);
                return Err(e.to_string());
            }
        }
        return read(&dir).ok_or_else(|| "copy failed".into());
    }
    let kind = kind_for(target).ok_or_else(|| format!("unsupported file type: {target}"))?;
    let file = if kind == Kind::Url {
        target.to_string()
    } else {
        let abs = fs::canonicalize(as_path).map_err(|e| format!("{target}: {e}"))?;
        dunce_str(&abs)
    };
    if let Some(w) = existing
        .iter()
        .find(|w| w.info.file.as_deref() == Some(&file))
    {
        return Ok(w.clone());
    }
    let title = if kind == Kind::Url {
        target
            .split("://")
            .nth(1)
            .unwrap_or(target)
            .trim_end_matches('/')
            .to_string()
    } else {
        as_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let info = Manifest {
        title: Some(title.clone()),
        r#type: kind,
        file: Some(file),
        external: kind != Kind::Url,
        ..Default::default()
    };
    let dir = new_dir(lib, &title).map_err(|e| e.to_string())?;
    crate::core::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())?;
    read(&dir).ok_or_else(|| "write failed".into())
}

/// `canonicalize` on Windows returns `\\?\C:\...`; strip it so paths stay readable and URL friendly.
pub(in crate::library) fn dunce_str(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// FNV-1a: a hash that stays the same across Rust releases, unlike DefaultHasher.
pub(in crate::library) fn fnv(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |h, b| {
        (h ^ *b as u32).wrapping_mul(0x0100_0193)
    })
}

pub(in crate::library) fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        let dest = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &dest)?
        } else {
            fs::copy(e.path(), dest).map(|_| ())?
        }
    }
    Ok(())
}
