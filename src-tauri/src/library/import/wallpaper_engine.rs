use crate::library::{Kind, Manifest};
use serde_json::{json, Map, Value};
use std::{fs, path::Path};

pub const PROJECT: &str = "project.json";

#[derive(Debug)]
pub struct Converted {
    pub info: Manifest,
    pub props: Map<String, Value>,
}

pub fn convert(dir: &Path) -> Result<Converted, String> {
    let raw = fs::read(dir.join(PROJECT)).map_err(|e| format!("{PROJECT}: {e}"))?;
    let p: Value = serde_json::from_slice(&raw).map_err(|e| format!("{PROJECT}: {e}"))?;
    let s = |k: &str| {
        p.get(k)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let kind = match s("type").unwrap_or_default().to_ascii_lowercase().as_str() {
        "web" => Kind::Web,
        "video" => Kind::Video,
        "application" => Kind::App,
        "scene" => {
            return Err(
                "Wallpaper Engine scene wallpapers need Wallpaper Engine's own renderer; \
                        Sarab imports its web, video and application wallpapers"
                    .into(),
            )
        }
        other => {
            return Err(format!(
                "unknown Wallpaper Engine wallpaper type: {other:?}"
            ))
        }
    };
    let file = s("file")
        .ok_or("project.json names no file")?
        .replace('\\', "/");
    if !inside(dir, &file) {
        return Err(format!("project.json names a missing file: {file}"));
    }
    let tags = p
        .get("tags")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty() && t.chars().count() <= 20)
                .take(5)
                .collect()
        })
        .unwrap_or_default();
    let api = if kind == Kind::Web {
        uses_apis(dir)
    } else {
        vec![]
    };
    let info = Manifest {
        title: s("title").map(|t| strip_tags(&t)),
        description: s("description").map(|d| strip_tags(&d)),
        r#type: kind,
        file: Some(file),
        thumbnail: s("preview").filter(|f| inside(dir, f)),
        tags,
        api,
        version: 1,
        ..Default::default()
    };
    let props = p
        .pointer("/general/properties")
        .and_then(Value::as_object)
        .map(properties)
        .unwrap_or_default();
    Ok(Converted { info, props })
}

fn inside(dir: &Path, rel: &str) -> bool {
    !Path::new(rel).is_absolute()
        && !rel.split(['/', '\\']).any(|c| c == "..")
        && dir.join(rel).is_file()
}

const API_CALLS: [(&[u8], &str); 2] = [
    (b"wallpaperRegisterAudioListener", "audio"),
    (b"wallpaperRegisterMedia", "nowplaying"),
];

fn uses_apis(dir: &Path) -> Vec<String> {
    fn walk(d: &Path, depth: u8, left: &mut u32, found: &mut [bool]) {
        let Ok(rd) = fs::read_dir(d) else {
            return;
        };
        for e in rd.flatten() {
            if *left == 0 || found.iter().all(|f| *f) {
                return;
            }
            *left -= 1;
            let p = e.path();
            if p.is_dir() {
                if depth < 4 {
                    walk(&p, depth + 1, left, found);
                }
                continue;
            }
            let script = p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "js" | "html" | "htm"));
            let small = e.metadata().is_ok_and(|m| m.len() < 8 << 20);
            if !(script && small) {
                continue;
            }
            if let Ok(b) = fs::read(&p) {
                for (seen, (needle, _)) in found.iter_mut().zip(API_CALLS) {
                    *seen |= b.windows(needle.len()).any(|w| w == needle);
                }
            }
        }
    }
    let mut found = [false; API_CALLS.len()];
    walk(dir, 0, &mut 1000, &mut found);
    API_CALLS
        .iter()
        .zip(found)
        .filter(|(_, f)| *f)
        .map(|((_, api), _)| api.to_string())
        .collect()
}

fn properties(we: &Map<String, Value>) -> Map<String, Value> {
    let mut items: Vec<_> = we.iter().collect();
    items.sort_by_key(|(_, p)| p.get("order").and_then(Value::as_i64).unwrap_or(i64::MAX));
    items
        .into_iter()
        .filter_map(|(k, p)| Some((k.clone(), control(k, p)?)))
        .collect()
}

fn control(key: &str, p: &Value) -> Option<Value> {
    let text = p
        .get("text")
        .and_then(Value::as_str)
        .map(strip_tags)
        .filter(|t| !t.is_empty() && !t.starts_with("ui_"))
        .unwrap_or_else(|| key.to_string());
    let v = p.get("value");
    Some(
        match p.get("type")?.as_str()?.to_ascii_lowercase().as_str() {
            "slider" => {
                let precision = p
                    .get("precision")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(6) as i32;
                let fraction = p
                    .get("fraction")
                    .and_then(Value::as_bool)
                    .unwrap_or(precision > 0);
                json!({
                    "type": "slider", "text": text,
                    "value": v.and_then(Value::as_f64).unwrap_or(0.0),
                    "min": p.get("min").and_then(Value::as_f64).unwrap_or(0.0),
                    "max": p.get("max").and_then(Value::as_f64).unwrap_or(100.0),
                    "step": if fraction { 10f64.powi(-precision.max(1)) } else { 1.0 },
                })
            }
            "bool" => {
                json!({ "type": "checkbox", "text": text, "value": v.and_then(Value::as_bool).unwrap_or(false) })
            }
            "color" => {
                json!({ "type": "color", "text": text, "value": hex(v?.as_str()?)?, "we": "color" })
            }
            "combo" => {
                let opts = p.get("options")?.as_array()?;
                let labels: Vec<String> = opts
                    .iter()
                    .map(|o| {
                        o.get("label")
                            .and_then(Value::as_str)
                            .map(strip_tags)
                            .unwrap_or_default()
                    })
                    .collect();
                let values: Vec<Value> = opts
                    .iter()
                    .map(|o| o.get("value").cloned().unwrap_or(Value::Null))
                    .collect();
                let at = v
                    .and_then(|v| values.iter().position(|x| plain(x) == plain(v)))
                    .unwrap_or(0);
                json!({ "type": "dropdown", "text": text, "value": at, "items": labels, "we_values": values })
            }
            "textinput" => {
                json!({ "type": "textbox", "text": text, "value": v.and_then(Value::as_str).unwrap_or("") })
            }
            "text" => json!({ "type": "label", "text": text, "value": text }),
            _ => return None,
        },
    )
}

pub fn page_value(ctl: &Value, v: &Value) -> Option<Value> {
    if ctl.get("we").and_then(Value::as_str) == Some("color") {
        let h = v.as_str()?.trim_start_matches('#');
        let c = |i: usize| {
            u8::from_str_radix(h.get(i..i + 2)?, 16)
                .ok()
                .map(|n| f64::from(n) / 255.0)
        };
        return Some(Value::String(format!(
            "{:.4} {:.4} {:.4}",
            c(0)?,
            c(2)?,
            c(4)?
        )));
    }
    let values = ctl.get("we_values")?.as_array()?;
    values.get(v.as_u64()? as usize).cloned()
}

fn hex(we: &str) -> Option<String> {
    let c: Vec<f64> = we
        .split_whitespace()
        .filter_map(|x| x.parse().ok())
        .collect();
    let b = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    (c.len() >= 3).then(|| format!("#{:02x}{:02x}{:02x}", b(c[0]), b(c[1]), b(c[2])))
}

fn plain(v: &Value) -> String {
    v.as_str()
        .map(String::from)
        .unwrap_or_else(|| v.to_string())
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for ch in s.chars() {
        match ch {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(dir: &Path, json: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(PROJECT), json).unwrap();
    }

    #[test]
    fn wallpaper_engine_web_project() {
        let d = std::env::temp_dir().join(format!("sarab-we-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        project(
            &d,
            r#"{
          "title": "Neon <b>rain</b>", "description": "Rain", "type": "Web", "file": "index.html",
          "preview": "preview.gif", "tags": ["Abstract", "Relaxing"],
          "general": { "properties": {
            "schemecolor": { "order": 0, "text": "ui_browse_properties_scheme_color", "type": "color", "value": "1 0.5 0" },
            "speed": { "order": 2, "text": "Speed", "type": "slider", "value": 3, "min": 1, "max": 10 },
            "glow": { "order": 1, "text": "Glow<br>", "type": "slider", "value": 0.5, "min": 0, "max": 1, "fraction": true, "precision": 2 },
            "drops": { "order": 3, "text": "Drops", "type": "bool", "value": true },
            "mode": { "order": 4, "text": "Mode", "type": "combo", "value": "b", "options": [ { "label": "A", "value": "a" }, { "label": "B", "value": "b" } ] },
            "bg": { "order": 5, "text": "Background", "type": "file" }
          } } }"#,
        );
        fs::write(
            d.join("index.html"),
            "<script>wallpaperRegisterAudioListener(f => {}); wallpaperRegisterMediaPropertiesListener(p => {})</script>",
        )
        .unwrap();
        fs::write(d.join("preview.gif"), "GIF").unwrap();
        let c = convert(&d).unwrap();
        assert_eq!(c.info.title.as_deref(), Some("Neon rain"));
        assert_eq!(c.info.r#type, Kind::Web);
        assert_eq!(c.info.thumbnail.as_deref(), Some("preview.gif"));
        assert_eq!(c.info.tags, ["abstract", "relaxing"]);
        assert_eq!(
            c.info.api,
            ["audio", "nowplaying"],
            "the page registers audio and media listeners"
        );
        let keys: Vec<&str> = c.props.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["schemecolor", "glow", "speed", "drops", "mode"],
            "in order, file picker left out"
        );
        assert_eq!(c.props["schemecolor"]["value"], "#ff8000");
        assert_eq!(
            c.props["schemecolor"]["text"], "schemecolor",
            "a localization key shows the property name"
        );
        assert_eq!(c.props["glow"]["step"], 0.01);
        assert_eq!(c.props["glow"]["text"], "Glow");
        assert_eq!(c.props["mode"]["value"], 1);
        assert_eq!(
            page_value(&c.props["schemecolor"], &json!("#ff8000")),
            Some(json!("1.0000 0.5020 0.0000"))
        );
        assert_eq!(page_value(&c.props["mode"], &json!(0)), Some(json!("a")));
        assert_eq!(page_value(&c.props["speed"], &json!(4)), None);
        project(&d, r#"{"type":"scene","file":"scene.json"}"#);
        assert!(convert(&d).unwrap_err().contains("scene"));
        project(&d, r#"{"type":"web","file":"../outside.html"}"#);
        assert!(convert(&d).is_err());
        project(&d, r#"{"type":"video","file":"missing.mp4"}"#);
        assert!(convert(&d).is_err());
        let _ = fs::remove_dir_all(&d);
    }
}
