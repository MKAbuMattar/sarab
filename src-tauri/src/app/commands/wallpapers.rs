use super::*;

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

#[tauri::command]
pub(crate) async fn move_library(app: AppHandle) -> Result<Option<String>, String> {
    let (old, owner) = with_core(&app, |app, core| {
        let owner = app
            .get_webview_window("main")
            .and_then(|w| os::platform::handle(&w))
            .map(|h| h.0 as isize);
        (wallpaper::library_dir(&core.settings), owner)
    });
    let owner = owner.map(|h| os::platform::HWND(h as *mut _));
    let Some(new) = os::platform::pick_folder(owner, old.parent()) else {
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

#[tauri::command]
pub(crate) async fn export_wallpaper(app: AppHandle, id: String) -> Result<String, String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    let zip = library::export_zip(&w, &support::downloads())?;
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", zip.display()))
        .spawn();
    Ok(zip.display().to_string())
}

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

pub(crate) fn editable(core: &Core, id: &str) -> Result<library::Wallpaper, String> {
    let w = core.find(id).cloned().ok_or("not found")?;
    if w.preset {
        return Err("built-in wallpapers cannot be edited".into());
    }
    Ok(w)
}
