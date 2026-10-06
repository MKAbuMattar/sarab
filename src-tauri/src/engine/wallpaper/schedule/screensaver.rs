use super::*;

pub(in crate::engine::wallpaper) fn screensaver_due(
    minutes: u32,
    idle_ms: u32,
    busy: bool,
) -> bool {
    minutes > 0 && !busy && u64::from(idle_ms) >= u64::from(minutes) * 60_000
}

pub(in crate::engine::wallpaper) fn start_screensaver(
    app: &AppHandle,
    core: &mut Core,
    stamp: u32,
) {
    let mut labels = vec![];
    for i in 0..core.displays.len() {
        let id = core
            .settings
            .screensaver_wallpaper
            .clone()
            .or_else(|| core.displays[i].wallpaper.clone());
        let Some(w) = id.as_deref().and_then(|id| core.find(id)).cloned() else {
            continue;
        };
        if w.info.r#type == Kind::Picture {
            continue;
        }
        let Ok(url) = url_for(app, &w, &core.settings.scaling) else {
            continue;
        };
        let lbl = format!("ss-{i}-{stamp}");
        let r = core.displays[i].mon.rect;
        let built = WebviewWindowBuilder::new(app, &lbl, WebviewUrl::External(url))
            .title("Sarab screensaver")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .shadow(false)
            .resizable(false)
            .initialization_script(INJECT)
            .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
            .build();
        let Ok(win) = built else { continue };
        let _ = win.set_position(tauri::PhysicalPosition::new(r.left, r.top));
        let _ = win.set_size(tauri::PhysicalSize::new(
            (r.right - r.left) as u32,
            (r.bottom - r.top) as u32,
        ));
        let _ = win.eval("document.documentElement.style.cursor='none'");
        let _ = win.show();
        labels.push(lbl);
    }
    if !labels.is_empty() {
        log(format!("screensaver on ({} displays)", labels.len()));
        core.screensaver = Some((labels, stamp));
    }
}

pub(in crate::engine::wallpaper) fn stop_screensaver(app: &AppHandle, core: &mut Core) {
    if let Some((labels, _)) = core.screensaver.take() {
        for l in labels {
            if let Some(w) = app.get_webview_window(&l) {
                let _ = w.destroy();
            }
        }
        log("screensaver off");
    }
}

pub(in crate::engine::wallpaper) fn run_screensaver(
    app: &AppHandle,
    core: &mut Core,
    others_playing: bool,
) {
    let (idle, stamp) = os::last_input();
    match &core.screensaver {
        Some((_, at)) if *at != stamp => stop_screensaver(app, core),
        Some(_) => {}
        None => {
            let busy =
                core.signals.covered.iter().any(|&c| c) || others_playing || core.signals.locked;
            if screensaver_due(core.settings.screensaver_minutes, idle, busy) {
                start_screensaver(app, core, stamp);
            }
        }
    }
}
