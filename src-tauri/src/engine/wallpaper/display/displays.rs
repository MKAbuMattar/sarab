use super::*;

pub fn sync_displays(app: &AppHandle, core: &mut Core) {
    let mons = os::monitors();
    let desk_ok = core.desktop.is_some_and(|d| os::desktop_alive(&d));
    let same = mons.len() == core.displays.len()
        && mons.iter().zip(&core.displays).all(|(m, d)| *m == d.mon);
    if same && desk_ok {
        return;
    }
    log(format!(
        "displays or desktop changed: {} monitors, desktop alive {desk_ok}",
        mons.len()
    ));
    for i in 0..core.displays.len() {
        close_window(app, core, i);
    }
    if !desk_ok {
        core.desktop = os::find_desktop();
    }
    core.displays = mons
        .into_iter()
        .map(|mon| Display {
            mon,
            wallpaper: None,
            state: State::Play,
            reason: Reason::None,
            loaded: false,
            error: None,
            page: Value::Null,
            label: None,
            unseen_since: None,
            unloaded: false,
            app: None,
        })
        .collect();
    for i in 0..core.displays.len() {
        let key = core.displays[i].mon.key.clone();
        if let Some(id) = core.layout.get(&key).cloned() {
            if core.find(&id).is_some() {
                let _ = apply(app, core, i, &id);
            } else {
                core.displays[i].error = Some(format!("wallpaper {id} is gone"));
            }
        }
    }
    write_status(core);
}
