//! Put a wallpaper on a display, take it off, and delete one from the library.

use super::*;

pub(in crate::engine::wallpaper) fn close_window(app: &AppHandle, core: &mut Core, i: usize) {
    if let Some(win) = window(app, &core.displays[i]) {
        let _ = win.destroy();
    }
    let d = &mut core.displays[i];
    d.loaded = false;
    d.page = Value::Null;
    d.label = None;
    d.app = None;
}

/// Close display `i`'s webview but keep its wallpaper and layout, to load it again later.
pub fn unload(app: &AppHandle, core: &mut Core, i: usize) {
    close_window(app, core, i);
}

pub(in crate::engine::wallpaper) fn restore_picture(core: &mut Core, i: usize) {
    let key = core.displays[i].mon.key.clone();
    if let Some(orig) = core.restore.remove(&key) {
        if let Err(e) = os::set_picture(&core.displays[i].mon, &orig) {
            log(format!("restore picture {key}: {e}"));
        }
        core.save_restore();
    }
}

pub fn apply(app: &AppHandle, core: &mut Core, i: usize, id: &str) -> Result<(), String> {
    let w = core
        .find(id)
        .cloned()
        .ok_or_else(|| format!("no wallpaper {id}"))?;
    close_window(app, core, i);
    let key = core.displays[i].mon.key.clone();
    core.displays[i].error = None;
    let result = if spanned(core, i) && w.info.r#type != Kind::Picture {
        // Drawn by display 0's window; this display only records what it shows.
        Ok(())
    } else if w.info.r#type == Kind::Picture {
        let Some(Target::File(f)) = w.target() else {
            return Err("picture has no file".into());
        };
        if !core.restore.contains_key(&key) {
            if let Some(orig) = os::get_picture(&core.displays[i].mon) {
                core.restore.insert(key.clone(), orig);
                core.save_restore();
            }
        }
        os::set_picture(&core.displays[i].mon, &f.to_string_lossy()).map_err(|e| e.to_string())
    } else {
        create_window(app, core, i, &w)
    };
    match &result {
        Ok(()) => {
            core.displays[i].wallpaper = Some(w.id.clone());
            core.layout.insert(key, w.id.clone());
            core.save_layout();
        }
        Err(e) => {
            core.displays[i].error = Some(e.clone());
            log(format!("apply {id} on display {i}: {e}"));
        }
    }
    write_status(core);
    result
}

/// Where display `i`'s wallpaper window goes: its display, or all of them when spanned.
pub(in crate::engine::wallpaper) fn wallpaper_rect(core: &Core, i: usize) -> RECT {
    if core.settings.span {
        span_rect(&core.displays.iter().map(|d| d.mon.rect).collect::<Vec<_>>())
    } else {
        core.displays[i].mon.rect
    }
}

pub(in crate::engine::wallpaper) fn create_window(
    app: &AppHandle,
    core: &mut Core,
    i: usize,
    w: &Wallpaper,
) -> Result<(), String> {
    if w.info.r#type == Kind::App {
        return start_app(app, core, i, w);
    }
    let desk = core.desktop.ok_or("desktop layer not found")?;
    let url = url_for(app, w, &core.settings.scaling)?;
    let origin = url.origin();
    let any_origin = w.info.r#type == Kind::Url;
    let lbl = new_label(i);
    let win = WebviewWindowBuilder::new(app, &lbl, WebviewUrl::External(url))
        .title(format!("sarab-wp-{i}"))
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .shadow(false)
        .resizable(false)
        .initialization_script(INJECT)
        // Local wallpapers stay on their own origin. URL wallpapers may redirect (embeds, logins).
        .on_navigation(move |u| any_origin || u.origin() == origin)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_page_load(|win, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                let lbl = win.label().to_string();
                later(win.app_handle(), move |app, core| {
                    on_loaded(app, core, &lbl)
                });
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    core.displays[i].label = Some(lbl);
    let attached = win.hwnd().map_err(|e| e.to_string()).and_then(|hwnd| {
        os::attach(&desk, hwnd, wallpaper_rect(core, i)).map_err(|e| e.to_string())?;
        os::show(hwnd, true);
        Ok(())
    });
    // A window that failed to embed would float over the desktop; drop it.
    if attached.is_err() {
        close_window(app, core, i);
    }
    attached
}

pub fn close(app: &AppHandle, core: &mut Core, i: usize) {
    close_window(app, core, i);
    restore_picture(core, i);
    core.displays[i].wallpaper = None;
    core.displays[i].error = None;
    let key = core.displays[i].mon.key.clone();
    core.layout.remove(&key);
    core.save_layout();
    os::refresh_desktop(core.desktop.as_ref());
    write_status(core);
}

/// Display selection from the CLI or UI: one display, or all of them.
pub fn targets(core: &Core, display: Option<usize>) -> Result<Vec<usize>, String> {
    match display {
        Some(n) if n < core.displays.len() => Ok(vec![n]),
        Some(n) => Err(format!("no display {n}; there are {}", core.displays.len())),
        None => Ok((0..core.displays.len()).collect()),
    }
}

pub fn remove(app: &AppHandle, core: &mut Core, id: &str) -> Result<(), String> {
    let w = core.find(id).cloned().ok_or("not found")?;
    let lib = library_dir(&core.settings);
    // Never delete outside the library folder, whatever the id says.
    let dir = fs::canonicalize(&w.dir).map_err(|e| e.to_string())?;
    let root = fs::canonicalize(&lib).map_err(|e| e.to_string())?;
    if !dir.starts_with(&root) || dir == root {
        return Err("refusing to delete outside the library".into());
    }
    for i in 0..core.displays.len() {
        if core.displays[i].wallpaper.as_deref() == Some(id) {
            close(app, core, i);
        }
    }
    // To the Recycle Bin, so a wrong click can be undone from Explorer.
    os::recycle(&dir)?;
    let _ = fs::remove_dir_all(cfg("props").join(id));
    core.lib = scan_all(&core.settings);
    Ok(())
}
