use super::*;

pub(in crate::engine::wallpaper) fn run_mouse_input(app: &AppHandle, core: &mut Core) {
    use std::sync::atomic::{AtomicBool, Ordering};
    let targets: Vec<(os::RECT, isize)> = if core.settings.mouse_input {
        core.displays
            .iter()
            .filter(|d| {
                d.wallpaper
                    .as_deref()
                    .and_then(|id| core.find(id))
                    .is_some_and(|w| matches!(w.info.r#type, Kind::Web | Kind::Url))
            })
            .filter_map(|d| os::handle(&window(app, d)?))
            .filter_map(|h| Some((os::window_rect(h)?, os::input_window(h)?)))
            .collect()
    } else {
        vec![]
    };
    let want = !targets.is_empty();
    *core.mouse_targets.lock().unwrap() = targets;
    match (&core.mouse_on, want) {
        (Some(on), false) => {
            on.store(false, Ordering::Relaxed);
            core.mouse_on = None;
        }
        (None, true) => {
            let on = std::sync::Arc::new(AtomicBool::new(true));
            os::forward_mouse(core.mouse_targets.clone(), on.clone());
            core.mouse_on = Some(on);
        }
        _ => {}
    }
}
