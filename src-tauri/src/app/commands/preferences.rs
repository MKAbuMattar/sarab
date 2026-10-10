use super::*;

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
pub(crate) async fn export_settings(app: AppHandle) -> Result<String, String> {
    let file = with_core(&app, |_, core| {
        support::export_settings(&core.settings, &support::downloads())
    })
    .map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", file.display()))
        .spawn();
    Ok(file.display().to_string())
}

#[tauri::command]
pub(crate) async fn import_settings(app: AppHandle, json: String) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let new = support::import_settings(&json, &core.settings)?;
        log("settings imported from a file");
        apply_settings(app, core, new)
    })
}

#[tauri::command]
pub(crate) async fn autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enable { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

pub(crate) fn apply_settings(
    app: &AppHandle,
    core: &mut Core,
    new: settings::Settings,
) -> Result<(), String> {
    let lib_changed = new.library_dir != core.settings.library_dir;
    let fit_changed = new.scaling != core.settings.scaling;
    let span_changed = new.span != core.settings.span;
    if new.theme != core.settings.theme {
        os::platform::menu_theme(&new.theme);
    }
    if let Some(w) = app.get_webview_window("main") {
        if new.theme != core.settings.theme {
            let _ = w.set_theme(theme_of(&new.theme));
        }
        if new.backdrop != core.settings.backdrop {
            let _ = w.set_effects(effects_for(&new.backdrop));
        }
    }
    let vol = new.volume;
    core.settings = new;
    settings::save(&wallpaper::cfg("settings.json"), &core.settings).map_err(|e| e.to_string())?;
    if lib_changed {
        rescan(core);
    }
    if span_changed {
        let first = core.displays.first().and_then(|d| d.wallpaper.clone());
        for i in 0..core.displays.len() {
            let id = core.displays[i].wallpaper.clone().or_else(|| first.clone());
            if let Some(id) = id {
                let _ = wallpaper::apply(app, core, i, &id);
            }
        }
    }
    if fit_changed && !span_changed {
        for i in 0..core.displays.len() {
            let id = core.displays[i].wallpaper.clone();
            let kind = id
                .as_deref()
                .and_then(|id| core.find(id))
                .map(|w| w.info.r#type);
            if let (Some(id), Some(library::Kind::Video | library::Kind::Gif)) = (id, kind) {
                let _ = wallpaper::apply(app, core, i, &id);
            }
        }
    }
    wallpaper::set_volume(app, core, vol);
    wallpaper::set_fps(app, core);
    wallpaper::tick(app, core);
    changed(app);
    Ok(())
}
