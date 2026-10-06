//! Volume and frame rate for every playing wallpaper.

use super::*;

pub fn set_volume(app: &AppHandle, core: &mut Core, v: u8) {
    core.settings.volume = v;
    // The rules apply on the next tick; until then the new volume plays.
    core.volume_now = v;
    let _ = settings::save(&cfg("settings.json"), &core.settings);
    for i in 0..core.displays.len() {
        if let Some(win) = window(app, &core.displays[i]) {
            let _ = win.eval(format!("window.__sarab&&__sarab.volume({v})"));
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

/// The volume the audio rules allow: the set volume, or silence while an app has the focus
/// (desktop only) or while another app plays sound.
pub(in crate::engine::wallpaper) fn effective_volume(
    s: &Settings,
    desktop_focused: bool,
    others_playing: bool,
) -> u8 {
    if (s.audio_desktop_only && !desktop_focused) || (s.audio_mute_others && others_playing) {
        0
    } else {
        s.volume
    }
}

/// Send the volume to every page without touching the saved setting.
pub(in crate::engine::wallpaper) fn push_volume(app: &AppHandle, core: &Core, v: u8) {
    for d in &core.displays {
        if let Some(win) = window(app, d) {
            let _ = win.eval(format!("window.__sarab&&__sarab.volume({v})"));
        }
    }
}
