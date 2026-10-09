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
            "library": core.lib.iter().map(|w| {
                let mut v = json!(w);
                let url = |p: &std::path::Path| {
                    let at = std::fs::metadata(p)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map_or(0, |d| d.as_secs());
                    app.asset_protocol_scope()
                        .allow_file(p)
                        .ok()
                        .map(|()| format!("{}?v={at}", wallpaper::asset_url(p)))
                };
                if let Some(u) = w.thumb.as_deref().and_then(url) {
                    v["thumb_url"] = json!(u);
                }
                if let Some(u) = w.auto_thumb.as_deref().and_then(url) {
                    v["auto_thumb_url"] = json!(u);
                }
                if let Some(u) = w.custom_thumb.as_deref().and_then(url) {
                    v["custom_thumb_url"] = json!(u);
                }
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
    std::process::Command::new("explorer.exe")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub const WEBSITE: &str = "https://sarab.mkabumattar.com";

pub const ISSUES: &str = "https://github.com/MKAbuMattar/sarab/issues";

#[tauri::command]
pub(crate) async fn clipboard_text(app: AppHandle) -> Result<String, String> {
    on_main(&app, |_| os::platform::clipboard_text())
}

#[tauri::command]
pub(crate) async fn system_menu(app: AppHandle, action: String) {
    on_main(&app, move |app| {
        if let Some(h) = app
            .get_webview_window("main")
            .and_then(|w| os::platform::handle(&w))
        {
            os::platform::system_command(h, &action);
        }
    })
}
