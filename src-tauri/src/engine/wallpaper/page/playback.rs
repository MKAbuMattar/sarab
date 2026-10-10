use super::*;

pub fn set_volume(app: &AppHandle, core: &mut Core, v: u8) {
    core.settings.volume = v;
    core.volume_now = v;
    let _ = settings::save(&cfg("settings.json"), &core.settings);
    push_volume(app, core);
}

pub(in crate::engine::wallpaper) fn page_volume(core: &Core, d: &Display) -> u8 {
    if core.muted {
        return 0;
    }
    d.wallpaper
        .as_deref()
        .and_then(|id| core.settings.wallpaper_volume.get(id).copied())
        .unwrap_or(core.settings.volume)
        .min(100)
}

pub(in crate::engine::wallpaper) fn page_rate(core: &Core, d: &Display) -> f64 {
    d.wallpaper
        .as_deref()
        .and_then(|id| core.settings.wallpaper_speed.get(id).copied())
        .filter(|r| r.is_finite())
        .map_or(1.0, |r| r.clamp(0.25, 4.0))
}

pub fn push_playback(app: &AppHandle, core: &Core) {
    for d in &core.displays {
        if let Some(win) = window(app, d) {
            let _ = win.eval(format!(
                "window.__sarab&&(__sarab.volume({}),__sarab.rate({}))",
                page_volume(core, d),
                page_rate(core, d)
            ));
        }
    }
}

pub fn set_fps(app: &AppHandle, core: &Core) {
    for i in 0..core.displays.len() {
        if let Some(win) = window(app, &core.displays[i]) {
            let _ = win.eval(format!(
                "window.__sarab&&__sarab.setFps({})",
                core.settings.fps
            ));
        }
    }
}

pub(in crate::engine::wallpaper) fn audio_muted(
    s: &Settings,
    desktop_focused: bool,
    others_playing: bool,
) -> bool {
    (s.audio_desktop_only && !desktop_focused) || (s.audio_mute_others && others_playing)
}

pub(in crate::engine::wallpaper) fn effective_volume(
    s: &Settings,
    desktop_focused: bool,
    others_playing: bool,
) -> u8 {
    if audio_muted(s, desktop_focused, others_playing) {
        0
    } else {
        s.volume
    }
}

pub(in crate::engine::wallpaper) fn push_volume(app: &AppHandle, core: &Core) {
    push_playback(app, core);
}
