//! Close a wallpaper nobody sees for a while, and load it again when it would play.

use super::*;

/// Reasons that mean nobody can see the wallpaper. Every other pause leaves it visible.
pub(in crate::engine::wallpaper) fn unseen(why: Reason) -> bool {
    matches!(why, Reason::Covered | Reason::Locked | Reason::Remote)
}

/// Unload when the setting is on and the wallpaper has been out of sight that long. Only then:
/// unloading a visible one would show the plain Windows wallpaper.
pub(in crate::engine::wallpaper) fn should_unload(
    why: Reason,
    unseen_for: std::time::Duration,
    minutes: u32,
) -> bool {
    minutes > 0 && unseen(why) && unseen_for.as_secs() >= u64::from(minutes) * 60
}

/// Free or bring back each display's webview as its visibility changes.
pub(in crate::engine::wallpaper) fn unload_or_reload(app: &AppHandle, core: &mut Core) {
    for i in 0..core.displays.len() {
        let d = &mut core.displays[i];
        let why = d.reason;
        if unseen(why) {
            d.unseen_since.get_or_insert_with(std::time::Instant::now);
        } else {
            d.unseen_since = None;
        }
        let unseen_for = d.unseen_since.map(|t| t.elapsed()).unwrap_or_default();
        if !d.unloaded
            && d.label.is_some()
            && should_unload(why, unseen_for, core.settings.unload_minutes)
        {
            log(format!(
                "display {i}: unloading its wallpaper, unseen for {}s",
                unseen_for.as_secs()
            ));
            close_window(app, core, i);
            core.displays[i].unloaded = true;
        } else if d.unloaded && !unseen(why) {
            core.displays[i].unloaded = false;
            if let Some(id) = core.displays[i].wallpaper.clone() {
                log(format!("display {i}: loading its wallpaper again"));
                let _ = apply(app, core, i, &id);
            }
        }
    }
}
