//! The window's state, and opening folders and links.

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

pub const WEBSITE: &str = "https://github.com/MKAbuMattar/sarab";

pub const ISSUES: &str = "https://github.com/MKAbuMattar/sarab/issues";
