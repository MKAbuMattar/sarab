//! status.json, read by `sarab status` and the checks.

use super::*;

pub fn write_status(core: &Core) {
    let displays: Vec<Value> = core
        .displays
        .iter()
        .map(|d| {
            json!({
                "key": d.mon.key,
                "rect": [d.mon.rect.left, d.mon.rect.top, d.mon.rect.right, d.mon.rect.bottom],
                "wallpaper": d.wallpaper,
                "title": d.wallpaper.as_deref().and_then(|id| core.find(id)).and_then(|w| w.info.title.clone()),
                "kind": d.wallpaper.as_deref().and_then(|id| core.find(id)).map(|w| w.kind),
                "state": d.state,
                "reason": d.reason,
                "loaded": d.loaded,
                "error": d.error,
                "page": d.page,
            })
        })
        .collect();
    let s = &core.signals;
    let v = json!({
        "pid": std::process::id(),
        "version": env!("CARGO_PKG_VERSION"),
        "manual": core.manual,
        "volume": core.settings.volume,
        "fps": core.settings.fps,
        "desktop": core.desktop.map(|d| json!({ "raised": d.raised, "progman": d.progman, "workerw": d.workerw, "defview": d.defview })),
        "signals": { "battery": s.on_battery, "power_saver": s.power_saver, "locked": s.locked, "remote": s.remote,
                     "foreground": s.foreground_app, "desktop_focused": s.desktop_focused, "covered": s.covered },
        "displays": displays,
    });
    if let Err(e) = settings::save(&cfg("status.json"), &v) {
        log(format!("status: {e}"));
    }
}

/// Ask every page for its own report (frame count, video time); results land in status.json.
pub fn probe_pages(app: &AppHandle, core: &Core) {
    for i in 0..core.displays.len() {
        let Some(win) = window(app, &core.displays[i]) else {
            continue;
        };
        let a = app.clone();
        let _ = win.eval_with_callback("window.__sarab?__sarab.status():null", move |r| {
            let v: Value = serde_json::from_str(&r).unwrap_or(Value::Null);
            later(&a, move |_, core| {
                if let Some(d) = core.displays.get_mut(i) {
                    d.page = v;
                }
                write_status(core);
            });
        });
    }
}
