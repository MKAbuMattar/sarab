use super::*;

pub fn set_prop(
    app: &AppHandle,
    core: &mut Core,
    i: usize,
    key: &str,
    raw: &Value,
) -> Result<Value, String> {
    let id = core.displays[i]
        .wallpaper
        .clone()
        .ok_or("no wallpaper on that display")?;
    let w = core.find(&id).cloned().ok_or("wallpaper missing")?;
    let dkey = core.displays[i].mon.key.clone();
    let saved = saved_props_path(&id, &dkey);
    let ctls = library::props(&w, &saved);
    let ctl = ctls.get(key).ok_or_else(|| format!("no property {key}"))?;
    let v = library::coerce(ctl, raw)?;
    library::save_prop(&saved, ctl, key, &v).map_err(|e| e.to_string())?;
    if let Some(win) = window(app, &core.displays[i]) {
        let send = if ctl.get("type").and_then(Value::as_str) == Some("folderDropdown") {
            ctl.get("folder")
                .and_then(Value::as_str)
                .zip(v.as_str())
                .map(|(f, v)| Value::String(format!("{f}/{v}")))
                .unwrap_or(Value::Null)
        } else {
            v.clone()
        };
        let mut args = vec![Value::String(key.into()), send];
        args.extend(crate::library::import::wallpaper_engine::page_value(
            ctl, &args[1],
        ));
        call(&win, "sarabPropertyChanged", &args);
    }
    Ok(v)
}

pub fn reset_props(app: &AppHandle, core: &mut Core, i: usize) -> Result<(), String> {
    let id = core.displays[i]
        .wallpaper
        .clone()
        .ok_or("no wallpaper on that display")?;
    let w = core.find(&id).cloned().ok_or("wallpaper missing")?;
    library::reset_props(&saved_props_path(&id, &core.displays[i].mon.key))
        .map_err(|e| e.to_string())?;
    if let Some(win) = window(app, &core.displays[i]) {
        push_props(&win, &w, &core.displays[i].mon.key);
    }
    Ok(())
}

pub fn props_for(core: &Core, i: usize) -> Map<String, Value> {
    let Some(id) = core.displays.get(i).and_then(|d| d.wallpaper.clone()) else {
        return Map::new();
    };
    let Some(w) = core.find(&id) else {
        return Map::new();
    };
    library::props(w, &saved_props_path(&id, &core.displays[i].mon.key))
}
