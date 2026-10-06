//! Display commands: close, pause and play, and Customize values.

use super::*;

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
