//! Host to page: calls into a wallpaper page, its properties, and pause and play.

use super::*;

/// Call a page function if it exists. serde_json output is a valid JS literal, so page data cannot break out.
pub(super) fn call(win: &WebviewWindow, func: &str, args: &[Value]) {
    let a = args
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let _ = win.eval(format!(
        "try{{typeof {func}==='function'&&{func}({a})}}catch(e){{console.error(e)}}"
    ));
}

pub(super) fn push_props(win: &WebviewWindow, w: &Wallpaper, key: &str) {
    for (name, ctl) in library::props(w, &saved_props_path(&w.id, key)) {
        let ty = ctl.get("type").and_then(Value::as_str).unwrap_or("");
        let v = ctl.get("value").cloned().unwrap_or(Value::Null);
        let v = match ty {
            "button" | "label" => continue,
            "folderDropdown" => {
                let rel = ctl
                    .get("folder")
                    .and_then(Value::as_str)
                    .zip(v.as_str())
                    .map(|(f, v)| format!("{f}/{v}"));
                match rel {
                    Some(r) if w.dir.join(&r).is_file() => Value::String(r),
                    _ => Value::Null,
                }
            }
            _ => v,
        };
        let mut args = vec![Value::String(name), v];
        args.extend(crate::library::wallpaper_engine::page_value(&ctl, &args[1]));
        call(win, "sarabPropertyChanged", &args);
    }
}

pub(super) fn apply_state(app: &AppHandle, core: &mut Core, i: usize, st: State, force: bool) {
    let prev = core.displays[i].state;
    if prev == st && !force {
        return;
    }
    core.displays[i].state = st;
    if st == State::Play {
        core.sync_due = true;
    }
    let Some(win) = window(app, &core.displays[i]) else {
        return;
    };
    // Every pause freezes in place. Hiding the window would show the plain Windows wallpaper
    // whenever the covering app minimizes, alt-tabs, or is see-through, and frozen already costs ~0 CPU.
    let paused = st != State::Play;
    let _ = win.eval(if paused {
        "window.__sarab&&__sarab.freeze()"
    } else {
        "window.__sarab&&__sarab.unfreeze()"
    });
    call(&win, "sarabPlaybackChanged", &[json!({ "paused": paused })]);
    log(format!("display {i} {:?} -> {st:?}", prev));
}

/// Page finished loading: send everything the page needs, then its current pause state.
pub fn on_loaded(app: &AppHandle, core: &mut Core, lbl: &str) {
    let Some(i) = core
        .displays
        .iter()
        .position(|d| d.label.as_deref() == Some(lbl))
    else {
        return;
    };
    core.displays[i].loaded = true;
    core.sync_due = true;
    let (Some(win), Some(id)) = (
        window(app, &core.displays[i]),
        core.displays[i].wallpaper.clone(),
    ) else {
        return;
    };
    let _ = win.eval(format!(
        "window.__sarab&&(__sarab.setFps({}),__sarab.volume({}))",
        core.settings.fps, core.volume_now
    ));
    if let Some(w) = core.find(&id).cloned() {
        push_props(&win, &w, &core.displays[i].mon.key);
    }
    let st = core.displays[i].state;
    apply_state(app, core, i, st, true);
    write_status(core);
    // A web or URL wallpaper without a thumbnail gets one from its first seconds on screen.
    let lbl = lbl.to_string();
    let a = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        later(&a, move |app, core| capture_thumbnail(app, core, &lbl));
    });
}
