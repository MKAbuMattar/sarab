use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
    pub author: Option<String>,
    pub license: Option<String>,
    pub r#type: Kind,
    /// Relative to the package folder, or an absolute path when `external` is true, or a URL.
    pub file: Option<String>,
    /// The file lives outside the package folder (added from disk, not copied).
    pub external: bool,
    pub thumbnail: Option<String>,
    pub tags: Vec<String>,
    pub version: u32,
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
    Some(Wallpaper {
        id: dir.file_name()?.to_string_lossy().into_owned(),
        dir: dir.to_path_buf(),
        kind: info.r#type.name(),
        has_props: dir.join(PROPS).is_file(),
        preset: false,
        info,
    })
}

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
        _ => return None,
    })
}

fn new_dir(lib: &Path, title: &str) -> io::Result<PathBuf> {
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
    let as_path = Path::new(target);
    if as_path.is_dir() {
        if read(as_path).is_none() {
            return Err(format!("{target} has no {INFO}"));
        }
        if as_path.starts_with(lib) {
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
    crate::settings::save(&dir.join(INFO), &info).map_err(|e| e.to_string())?;
    read(&dir).ok_or_else(|| "write failed".into())
}

/// `canonicalize` on Windows returns `\\?\C:\...`; strip it so paths stay readable and URL friendly.
fn dunce_str(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

/// FNV-1a: a hash that stays the same across Rust releases, unlike DefaultHasher.
fn fnv(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c9dc5u32, |h, b| {
        (h ^ *b as u32).wrapping_mul(0x0100_0193)
    })
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
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

pub const MAX_UNPACKED: u64 = 2 << 30;

/// Import a Sarab package zip (sarab.json at its root). Rejects entries that escape the folder and archives over the size cap.
pub fn import_zip(lib: &Path, zip_path: &Path, max_bytes: u64) -> Result<Wallpaper, String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut total = 0u64;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.enclosed_name().is_none() {
            return Err(format!("unsafe path in zip: {}", f.name()));
        }
        total += f.size();
    }
    if total > max_bytes {
        return Err(format!(
            "zip unpacks to {total} bytes, limit is {max_bytes}"
        ));
    }
    if zip.by_name(INFO).is_err() {
        return Err(format!("zip has no {INFO} at its root"));
    }
    let title = zip_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = new_dir(lib, &title).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            let rel = f.enclosed_name().ok_or("unsafe path")?;
            let out = dir.join(rel);
            if f.is_dir() {
                fs::create_dir_all(&out).map_err(|e| e.to_string())?;
                continue;
            }
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            // take() bounds the write even if the header lied about the size.
            let mut w = fs::File::create(&out).map_err(|e| e.to_string())?;
            io::copy(&mut io::Read::take(&mut f, max_bytes), &mut w).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    read(&dir).ok_or_else(|| {
        let _ = fs::remove_dir_all(&dir);
        format!("{INFO} in zip is invalid")
    })
}

// ---- properties.json ----

/// Controls from the wallpaper's properties.json with this display's saved values laid over them.
pub fn props(w: &Wallpaper, saved_path: &Path) -> Map<String, Value> {
    let mut base: Map<String, Value> = fs::read(w.props_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let saved: Map<String, Value> = fs::read(saved_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    for (k, ctl) in base.iter_mut() {
        if let (Some(v), Some(obj)) = (saved.get(k), ctl.as_object_mut()) {
            obj.insert("value".into(), v.clone());
        }
    }
    base
}

/// Parse a raw value (from the CLI or UI) by the control's type. Returns the JS value to send.
pub fn coerce(ctl: &Value, raw: &Value) -> Result<Value, String> {
    let ty = ctl.get("type").and_then(Value::as_str).unwrap_or("");
    let s = match raw {
        Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    Ok(match ty {
        "slider" => {
            let n: f64 = s.parse().map_err(|_| format!("{s} is not a number"))?;
            let min = ctl.get("min").and_then(Value::as_f64).unwrap_or(f64::MIN);
            let max = ctl.get("max").and_then(Value::as_f64).unwrap_or(f64::MAX);
            serde_json::json!(n.clamp(min, max))
        }
        "checkbox" => Value::Bool(s.parse().map_err(|_| format!("{s} is not true/false"))?),
        "dropdown" | "scalerDropdown" => {
            let i: usize = s.parse().map_err(|_| format!("{s} is not an index"))?;
            let len = ctl
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            if i >= len {
                return Err(format!("index {i} out of range 0..{len}"));
            }
            serde_json::json!(i)
        }
        "button" => Value::Bool(true),
        "label" => return Err("labels have no value".into()),
        _ => Value::String(s),
    })
}

/// Save one value into the per-display file. Buttons are events, never saved.
pub fn save_prop(saved_path: &Path, ctl: &Value, key: &str, value: &Value) -> io::Result<()> {
    if ctl.get("type").and_then(Value::as_str) == Some("button") {
        return Ok(());
    }
    let mut saved: Map<String, Value> = fs::read(saved_path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    saved.insert(key.into(), value.clone());
    crate::settings::save(saved_path, &saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sarab-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let mut z = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, body) in entries {
            z.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            z.write_all(body.as_bytes()).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn zip_slip_rejected() {
        let d = tmp("slip");
        let lib = d.join("lib");
        let z = d.join("evil.zip");
        make_zip(
            &z,
            &[
                ("sarab.json", r#"{"type":"web","file":"index.html"}"#),
                ("../evil.txt", "x"),
            ],
        );
        let err = import_zip(&lib, &z, MAX_UNPACKED).unwrap_err();
        assert!(err.contains("unsafe path"), "{err}");
        assert!(!d.join("evil.txt").exists());
        assert!(scan(&lib).is_empty());
    }

    #[test]
    fn zip_size_cap() {
        let d = tmp("cap");
        let z = d.join("big.zip");
        make_zip(
            &z,
            &[
                ("sarab.json", r#"{"type":"web"}"#),
                ("a.bin", &"x".repeat(5000)),
            ],
        );
        assert!(import_zip(&d.join("lib"), &z, 1000)
            .unwrap_err()
            .contains("limit"));
    }

    #[test]
    fn package_zip_round_trip() {
        let d = tmp("round");
        let lib = d.join("lib");
        let z = d.join("Rain.zip");
        let info = r#"{"title":"Rain","description":"d","author":"a","type":"web","file":"index.html","tags":["x"],"version":3}"#;
        make_zip(
            &z,
            &[
                ("sarab.json", info),
                ("index.html", "<p>hi</p>"),
                ("js/app.js", "1"),
                (
                    "properties.json",
                    r#"{"speed":{"type":"slider","value":1,"min":0,"max":5,"step":0.1,"text":"Speed"}}"#,
                ),
            ],
        );
        let w = import_zip(&lib, &z, MAX_UNPACKED).unwrap();
        assert_eq!(w.info.title.as_deref(), Some("Rain"));
        assert_eq!(w.info.r#type, Kind::Web);
        assert_eq!(w.info.version, 3);
        assert!(w.has_props);
        assert!(w.dir.join("js/app.js").is_file());
        let back = serde_json::to_value(&w.info).unwrap();
        assert_eq!(back["type"], "web");
        let scanned = scan(&lib);
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].info, w.info);
        // Unknown types are rejected rather than guessed.
        assert!(serde_json::from_str::<Manifest>(r#"{"type":"unity"}"#).is_err());
        // A folder without sarab.json is not a package.
        let other = d.join("other.zip");
        make_zip(&other, &[("index.html", "x")]);
        assert!(import_zip(&lib, &other, MAX_UNPACKED)
            .unwrap_err()
            .contains("sarab.json"));
    }

    #[test]
    fn property_merge_and_coerce() {
        let d = tmp("props");
        fs::write(d.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
        fs::write(d.join(PROPS), r#"{"zeta":{"type":"slider","value":1,"min":0,"max":5,"step":1,"text":"Z"},"alpha":{"type":"checkbox","value":false,"text":"A"},"pick":{"type":"dropdown","value":0,"items":["a","b"],"text":"P"},"go":{"type":"button","value":"Go","text":"Go"}}"#).unwrap();
        let w = read(&d).unwrap();
        let saved = d.join("saved.json");
        let p = props(&w, &saved);
        // File order is kept (serde_json preserve_order), so the UI shows controls as authored.
        assert_eq!(
            p.keys().collect::<Vec<_>>(),
            vec!["zeta", "alpha", "pick", "go"]
        );
        let v = coerce(&p["zeta"], &Value::String("9".into())).unwrap();
        assert_eq!(v, serde_json::json!(5.0));
        save_prop(&saved, &p["zeta"], "zeta", &v).unwrap();
        save_prop(&saved, &p["go"], "go", &Value::Bool(true)).unwrap();
        let p2 = props(&w, &saved);
        assert_eq!(p2["zeta"]["value"], serde_json::json!(5.0));
        assert_eq!(p2["go"]["value"], "Go");
        assert!(coerce(&p["alpha"], &Value::String("yes".into())).is_err());
        assert!(coerce(&p["pick"], &Value::String("2".into())).is_err());
        assert_eq!(
            coerce(&p["pick"], &Value::String("1".into())).unwrap(),
            serde_json::json!(1)
        );
    }

    #[test]
    fn add_file_and_url() {
        let d = tmp("add");
        let lib = d.join("lib");
        let f = d.join("clip.mp4");
        fs::write(&f, "x").unwrap();
        let w = add(&lib, &[], f.to_str().unwrap()).unwrap();
        assert_eq!(w.info.r#type, Kind::Video);
        assert!(w.info.external);
        let again = add(&lib, &scan(&lib), f.to_str().unwrap()).unwrap();
        assert_eq!(again.id, w.id, "same file is not added twice");
        let u = add(&lib, &[], "https://www.shadertoy.com/view/abc").unwrap();
        assert_eq!(u.info.r#type, Kind::Url);
        assert!(matches!(u.target(), Some(Target::Url(_))));
        assert!(add(&lib, &[], "C:/x.docx").is_err());
        let src = d.join("Rain");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join(INFO), r#"{"type":"web","file":"index.html"}"#).unwrap();
        let a = add(&lib, &scan(&lib), src.to_str().unwrap()).unwrap();
        let b = add(&lib, &scan(&lib), src.to_str().unwrap()).unwrap();
        assert_eq!(a.id, b.id, "same folder is copied once");
        assert_eq!(
            scan(&lib)
                .iter()
                .filter(|w| w.id.starts_with("Rain-"))
                .count(),
            1
        );
    }
}
