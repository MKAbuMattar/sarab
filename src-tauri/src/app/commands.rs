//! Commands for the settings window. The capability grants them to the "main" window only.

use super::*;

#[tauri::command]
pub(crate) async fn state(app: AppHandle) -> Value {
    let auto = app.autolaunch().is_enabled().unwrap_or(false);
    with_core(&app, move |app, core| {
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "webview": tauri::webview_version().unwrap_or_default(),
            "theme": page_theme(app, &core.settings.theme),
            "translucent": effects_for(&core.settings.backdrop).is_some(),
            "settings": core.settings,
            // Thumbnails reach the window through the asset protocol, opened file by file.
            "library": core.lib.iter().map(|w| {
                let mut v = json!(w);
                if let Some(t) = &w.thumb {
                    if app.asset_protocol_scope().allow_file(t).is_ok() {
                        v["thumb_url"] = json!(wallpaper::asset_url(t));
                    }
                }
                // Videos and GIFs play on their tile while the pointer rests on it.
                if let (Some(library::Target::File(f)), library::Kind::Video | library::Kind::Gif) =
                    (w.target(), w.info.r#type)
                {
                    if f.is_file() && app.asset_protocol_scope().allow_file(&f).is_ok() {
                        v["preview_url"] = json!(wallpaper::asset_url(&f));
                    }
                }
                v
            }).collect::<Vec<_>>(),
            "categories": library::CATEGORIES,
            "library_dir": wallpaper::library_dir(&core.settings),
            "manual": core.manual,
            "autostart": auto,
            "update": update::to_json(app),
            "presets": presets::to_json(app, &core.lib),
            "displays": core.displays.iter().map(|d| json!({
                "key": d.mon.key, "wallpaper": d.wallpaper, "state": d.state, "reason": d.reason, "error": d.error,
                "x": d.mon.rect.left, "y": d.mon.rect.top,
                "width": d.mon.rect.right - d.mon.rect.left, "height": d.mon.rect.bottom - d.mon.rect.top,
            })).collect::<Vec<_>>(),
        })
    })
}

#[tauri::command]
pub(crate) async fn set_wallpaper(
    app: AppHandle,
    target: String,
    display: Option<usize>,
) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Set { target, display })
    })
}

#[tauri::command]
pub(crate) async fn add(app: AppHandle, target: String) -> Result<Value, String> {
    with_core(&app, move |app, core| {
        let id = resolve(core, target.trim())?;
        changed(app);
        Ok(json!(id))
    })
}

#[tauri::command]
pub(crate) async fn import(app: AppHandle, path: String) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Import(path))
    })
}

#[tauri::command]
pub(crate) async fn remove(app: AppHandle, id: String) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let r = wallpaper::remove(app, core, &id);
        changed(app);
        r
    })
}

#[tauri::command]
pub(crate) async fn close(app: AppHandle, display: Option<usize>) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Close { display })
    })
}

#[tauri::command]
pub(crate) async fn toggle_pause(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Toggle)
    })
}

/// Opens one of Sarab's folders in Explorer, or its website or issue tracker in the browser.
/// Takes a fixed name, never a path or URL, so the page cannot open anything else.
#[tauri::command]
pub(crate) async fn open(app: AppHandle, which: String) -> Result<(), String> {
    let target = match which.as_str() {
        "config" | "library" => {
            let lib = with_core(&app, |_, core| wallpaper::library_dir(&core.settings));
            let dir = if which == "config" {
                settings::config_dir()
            } else {
                lib
            };
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.into_os_string()
        }
        "website" => WEBSITE.into(),
        "issues" => ISSUES.into(),
        _ => return Err(format!("unknown target {which}")),
    };
    // explorer.exe opens folders itself and hands URLs to the default browser.
    std::process::Command::new("explorer.exe")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn play_anyway(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Play)
    })
}

#[tauri::command]
pub(crate) async fn resume_auto(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Resume)
    })
}

#[tauri::command]
pub(crate) async fn props(app: AppHandle, display: usize) -> Value {
    with_core(&app, move |_, core| {
        Value::Object(wallpaper::props_for(core, display))
    })
}

#[tauri::command]
pub(crate) async fn set_prop(
    app: AppHandle,
    display: usize,
    key: String,
    value: Value,
) -> Result<Value, String> {
    with_core(&app, move |app, core| {
        if display >= core.displays.len() {
            return Err("no such display".into());
        }
        wallpaper::set_prop(app, core, display, &key, &value)
    })
}

#[tauri::command]
pub(crate) async fn reset_props(app: AppHandle, display: usize) -> Result<(), String> {
    with_core(&app, move |app, core| {
        if display >= core.displays.len() {
            return Err("no such display".into());
        }
        wallpaper::reset_props(app, core, display)
    })
}

#[tauri::command]
pub(crate) async fn save_settings(app: AppHandle, new: settings::Settings) -> Result<(), String> {
    with_core(&app, move |app, core| apply_settings(app, core, new))
}

#[tauri::command]
pub(crate) async fn reset_settings(app: AppHandle) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let new = support::reset(&core.settings);
        log("settings reset to defaults");
        apply_settings(app, core, new)
    })
}

/// Zip the log and settings files into Downloads and show the zip in Explorer.
#[tauri::command]
pub(crate) async fn export_logs() -> Result<String, String> {
    let zip = support::export_logs(&settings::config_dir(), &support::downloads())
        .map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", zip.display()))
        .spawn();
    Ok(zip.display().to_string())
}

#[tauri::command]
pub(crate) async fn edit_info(
    app: AppHandle,
    id: String,
    edit: library::Edit,
) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let w = editable(core, &id)?;
        library::edit_info(&w.dir, edit)?;
        rescan(core);
        changed(app);
        Ok(())
    })
}

#[tauri::command]
pub(crate) async fn details(app: AppHandle, id: String) -> Result<library::Details, String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    Ok(library::details(&w))
}

/// Ask for a folder, then move the library there. Running wallpapers close during the move and
/// open again from the new place. Returns the new folder, or None when cancelled.
#[tauri::command]
pub(crate) async fn move_library(app: AppHandle) -> Result<Option<String>, String> {
    let (old, owner) = with_core(&app, |app, core| {
        let owner = app
            .get_webview_window("main")
            .and_then(|w| w.hwnd().ok())
            .map(|h| h.0 as isize);
        (wallpaper::library_dir(&core.settings), owner)
    });
    let owner = owner.map(|h| windows::Win32::Foundation::HWND(h as *mut _));
    let Some(new) = os::windows::pick_folder(owner, old.parent()) else {
        return Ok(None);
    };
    let new = if new.file_name().is_some_and(|n| n == "Library") {
        new
    } else {
        new.join("Library")
    };
    with_core(&app, move |app, core| {
        for i in 0..core.displays.len() {
            wallpaper::unload(app, core, i);
        }
        let moved = library::move_library(&old, &new);
        if moved.is_ok() {
            core.settings.library_dir = Some(new.clone());
            settings::save(&wallpaper::cfg("settings.json"), &core.settings)
                .map_err(|e| e.to_string())?;
            rescan(core);
            log(format!("library moved to {}", new.display()));
        }
        for i in 0..core.displays.len() {
            if let Some(id) = core.displays[i].wallpaper.clone() {
                let _ = wallpaper::apply(app, core, i, &id);
            }
        }
        changed(app);
        moved.map(|_| Some(new.display().to_string()))
    })
}

/// Saves the wallpaper as a package zip in Downloads and shows it in Explorer.
#[tauri::command]
pub(crate) async fn export_wallpaper(app: AppHandle, id: String) -> Result<String, String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    let zip = library::export_zip(&w, &support::downloads())?;
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", zip.display()))
        .spawn();
    Ok(zip.display().to_string())
}

/// Opens the wallpaper's own folder in Explorer. Takes an id, never a path.
#[tauri::command]
pub(crate) async fn reveal(app: AppHandle, id: String) -> Result<(), String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    std::process::Command::new("explorer.exe")
        .arg(&w.dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_preset(app: AppHandle, id: String) -> Result<(), String> {
    presets::download(&app, &id).await
}

#[tauri::command]
pub(crate) async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    update::check(&app).await
}

#[tauri::command]
pub(crate) async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}

#[tauri::command]
pub(crate) async fn autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enable { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}
