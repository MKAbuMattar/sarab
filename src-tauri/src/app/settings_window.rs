//! Helpers behind the settings window commands: saving settings, links, editable wallpapers.

use super::*;

pub const WEBSITE: &str = "https://github.com/MKAbuMattar/sarab";

pub const ISSUES: &str = "https://github.com/MKAbuMattar/sarab/issues";

/// Store new settings and apply what changed to the window and the wallpapers.
pub(crate) fn apply_settings(
    app: &AppHandle,
    core: &mut Core,
    new: settings::Settings,
) -> Result<(), String> {
    let lib_changed = new.library_dir != core.settings.library_dir;
    let fit_changed = new.scaling != core.settings.scaling;
    let span_changed = new.span != core.settings.span;
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
    // Span on or off: display 0's window changes size and the others gain or lose theirs.
    if span_changed {
        let first = core.displays.first().and_then(|d| d.wallpaper.clone());
        for i in 0..core.displays.len() {
            let id = core.displays[i].wallpaper.clone().or_else(|| first.clone());
            if let Some(id) = id {
                let _ = wallpaper::apply(app, core, i, &id);
            }
        }
    }
    // The fit is part of the player URL, so running videos and GIFs load again with it.
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

pub(crate) fn editable(core: &Core, id: &str) -> Result<library::Wallpaper, String> {
    let w = core.find(id).cloned().ok_or("not found")?;
    if w.preset {
        return Err("built-in wallpapers cannot be edited".into());
    }
    Ok(w)
}
