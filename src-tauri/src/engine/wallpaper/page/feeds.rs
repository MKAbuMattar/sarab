use super::*;

pub(in crate::engine::wallpaper) fn subscribers(core: &Core, feed: &str) -> Vec<usize> {
    (0..core.displays.len())
        .filter(|&i| {
            let d = &core.displays[i];
            d.loaded
                && d.state == State::Play
                && d.wallpaper
                    .as_deref()
                    .and_then(|id| core.find(id))
                    .is_some_and(|w| w.info.api.iter().any(|a| a == feed))
        })
        .collect()
}

pub(in crate::engine::wallpaper) fn push_sysinfo(app: &AppHandle, core: &mut Core) {
    let to = subscribers(core, "system");
    if to.is_empty() {
        return;
    }
    let info = core
        .sysinfo
        .get_or_insert_with(crate::engine::feeds::Sampler::new)
        .sample();
    let v = serde_json::to_value(&info).unwrap_or(Value::Null);
    for i in to {
        if let Some(win) = window(app, &core.displays[i]) {
            call(&win, "sarabSystemInfo", std::slice::from_ref(&v));
        }
    }
}

pub(in crate::engine::wallpaper) fn push_now_playing(app: &AppHandle, core: &mut Core) {
    use std::sync::atomic::Ordering;
    let to = subscribers(core, "nowplaying");
    if to.is_empty() {
        if let Some(n) = &mut core.now_playing {
            n.wanted.store(false, Ordering::Relaxed);
            n.sent = None;
        }
        return;
    }
    let n = core
        .now_playing
        .get_or_insert_with(crate::engine::feeds::NowPlaying::start);
    n.wanted.store(true, Ordering::Relaxed);
    let latest = n.latest.lock().unwrap().clone();
    let same = match (&n.sent, &latest) {
        (Some(Some(a)), Some(b)) => a.same_song(b),
        (Some(None), None) => true,
        _ => false,
    };
    if same && !core.ticks.is_multiple_of(10) {
        return;
    }
    n.sent = Some(latest.clone());
    let v = serde_json::to_value(&latest).unwrap_or(Value::Null);
    for i in to {
        if let Some(win) = window(app, &core.displays[i]) {
            call(&win, "__sarabMedia", std::slice::from_ref(&v));
        }
    }
}

pub(in crate::engine::wallpaper) fn run_audio_feed(app: &AppHandle, core: &mut Core) {
    let to: Vec<String> = subscribers(core, "audio")
        .into_iter()
        .filter_map(|i| core.displays[i].label.clone())
        .collect();
    let want = !to.is_empty();
    *core.audio_to.lock().unwrap() = to;
    match (&core.audio, want) {
        (Some(f), false) => {
            f.stop();
            core.audio = None;
        }
        (Some(f), true) if f.running() => {}
        (_, true) => {
            let (a, to) = (app.clone(), core.audio_to.clone());
            core.audio = Some(crate::engine::audio::Feed::start(move |s| {
                let levels: Vec<String> = s.iter().map(|v| format!("{v:.3}")).collect();
                let js = format!(
                    "try{{typeof sarabAudio==='function'&&sarabAudio([{}])}}catch(e){{}}",
                    levels.join(",")
                );
                for lbl in to.lock().unwrap().iter() {
                    if let Some(w) = a.get_webview_window(lbl) {
                        let _ = w.eval(&js);
                    }
                }
            }));
        }
        (None, false) => {}
    }
}
