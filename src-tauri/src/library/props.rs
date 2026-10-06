//! properties.json: a wallpaper's controls and the values saved per display.

use super::*;

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
        // A number field is a slider without the slider: the same range rules.
        "slider" | "number" => {
            let n: f64 = s
                .trim()
                .parse()
                .map_err(|_| format!("{s} is not a number"))?;
            if !n.is_finite() {
                return Err(format!("{s} is not a number"));
            }
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
    crate::core::settings::save(saved_path, &saved)
}

/// Forget the values saved for one display, so the wallpaper's own defaults apply again.
pub fn reset_props(saved_path: &Path) -> io::Result<()> {
    match fs::remove_file(saved_path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r,
    }
}
