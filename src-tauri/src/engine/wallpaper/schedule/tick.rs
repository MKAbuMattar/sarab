use super::*;

pub fn tick(app: &AppHandle, core: &mut Core) {
    sync_displays(app, core);
    if let Some(d) = core.desktop {
        let wins: Vec<_> = core
            .displays
            .iter()
            .filter_map(|x| window(app, x))
            .filter_map(|w| os::handle(&w))
            .chain(
                core.displays
                    .iter()
                    .filter_map(|x| x.app.as_ref()?.hwnd)
                    .map(|h| os::HWND(h as _)),
            )
            .collect();
        if os::ensure_order(&d, &wins) {
            log("desktop z-order repaired: a wallpaper had fallen below WorkerW or was hidden");
        }
    }
    let mons: Vec<os::Monitor> = core.displays.iter().map(|d| d.mon.clone()).collect();
    let mut s = os::signals(&mons);
    s.manual = core.manual;
    s.cpu_busy = cpu_busy(core);
    other_load(core, &mut s);
    let decisions = pause::decide(&s, &core.settings);
    let changed = s != core.signals
        || decisions
            .iter()
            .zip(&core.displays)
            .any(|((st, why), d)| *st != d.state || *why != d.reason);
    core.signals = s;
    let decisions = if core.screensaver.is_some() {
        vec![(State::Covered, Reason::Covered); decisions.len()]
    } else {
        decisions
    };
    let decisions = if core.settings.span {
        vec![span_state(&decisions); decisions.len()]
    } else {
        decisions
    };
    for (i, (st, why)) in decisions.into_iter().enumerate() {
        core.displays[i].reason = why;
        apply_state(app, core, i, st, false);
    }
    unload_or_reload(app, core);
    push_sysinfo(app, core);
    push_now_playing(app, core);
    run_audio_feed(app, core);
    run_mouse_input(app, core);
    let ss_soon = core.settings.screensaver_minutes > 0
        && core.screensaver.is_none()
        && u64::from(os::last_input().0) >= u64::from(core.settings.screensaver_minutes) * 60_000;
    let others = ((core.settings.audio_mute_others && core.settings.volume > 0) || ss_soon)
        && os::other_audio_playing();
    run_screensaver(app, core, others);
    let v = effective_volume(&core.settings, core.signals.desktop_focused, others);
    if v != core.volume_now {
        log(format!("volume {} -> {v} (audio rules)", core.volume_now));
        core.volume_now = v;
        push_volume(app, core, v);
    }
    core.ticks += 1;
    run_schedule(app, core);
    let playing = core.displays.iter().any(|d| d.wallpaper.is_some());
    let resting = core.displays.iter().any(|d| d.state != State::Play);
    if cycle_due(
        core.settings.cycle_minutes,
        core.changed_at.elapsed(),
        playing,
        resting,
    ) {
        log("cycling to the next wallpaper");
        if let Err(e) = next(app, core) {
            log(format!("cycle: {e}"));
        }
        core.changed_at = std::time::Instant::now();
        let _ = app.emit_to("main", "changed", ());
    }
    core.sync_due = false;
    sync_video(app, core);
    if changed {
        write_status(core);
        let _ = app.emit_to("main", "changed", ());
    }
}
