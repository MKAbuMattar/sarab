use super::*;

pub(in crate::engine::wallpaper) fn minutes(at: &str) -> Option<u32> {
    let (h, m) = at.split_once(':')?;
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

pub(in crate::engine::wallpaper) fn slot_now(slots: &[settings::Slot], now: u32) -> Option<usize> {
    let timed: Vec<(usize, u32)> = slots
        .iter()
        .enumerate()
        .filter(|(_, s)| !s.wallpaper.is_empty())
        .filter_map(|(i, s)| Some((i, minutes(&s.at)?)))
        .collect();
    let latest = |it: &mut dyn Iterator<Item = &(usize, u32)>| {
        it.max_by_key(|(i, m)| (*m, usize::MAX - i))
            .map(|(i, _)| *i)
    };
    latest(&mut timed.iter().filter(|(_, m)| *m <= now)).or_else(|| latest(&mut timed.iter()))
}

fn lower(p: &Path) -> String {
    let s = p.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

pub(in crate::engine::wallpaper) fn in_source(w: &Wallpaper, source: &str) -> bool {
    if w.info.r#type == Kind::App {
        return false;
    }
    if source.is_empty() || source == "all" {
        return true;
    }
    match source.split_once(':') {
        Some(("tag", tag)) => w.info.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)),
        Some(("folder", dir)) => match w.target() {
            Some(Target::File(f)) => lower(&f).starts_with(&format!("{}/", lower(Path::new(dir)))),
            _ => false,
        },
        _ => w.info.category.as_deref() == Some(source),
    }
}

pub(in crate::engine::wallpaper) fn source_for(core: &Core, i: usize) -> String {
    let i = if core.settings.span { 0 } else { i };
    core.displays
        .get(i)
        .and_then(|d| core.settings.display_playlists.get(&d.mon.key))
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| core.settings.cycle_category.clone())
}

pub(in crate::engine::wallpaper) fn add_folder(core: &mut Core, source: &str) {
    let Some(dir) = source.strip_prefix("folder:") else {
        return;
    };
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    let lib = library_dir(&core.settings);
    let mut added = 0;
    for e in rd.flatten().take(500) {
        let p = e.path();
        let media = matches!(
            library::kind_for(&p.to_string_lossy()),
            Some(Kind::Video | Kind::Gif | Kind::Picture)
        );
        if !p.is_file() || !media || library::needs_convert(&p.to_string_lossy()) {
            continue;
        }
        match library::add(&lib, &core.lib, &p.to_string_lossy()) {
            Ok(w) if core.find(&w.id).is_none() => {
                core.lib.push(w);
                added += 1;
            }
            Ok(_) => {}
            Err(e) => log(format!("playlist folder: {e}")),
        }
    }
    if added > 0 {
        log(format!(
            "playlist folder: added {added} new files from {dir}"
        ));
    }
}

pub(in crate::engine::wallpaper) fn run_schedule(app: &AppHandle, core: &mut Core) {
    let slots = core.settings.schedule.clone();
    let Some(i) = slot_now(&slots, os::local_minutes()) else {
        core.slot = None;
        return;
    };
    let due = (slots[i].at.clone(), slots[i].wallpaper.clone());
    if core.slot.as_ref() == Some(&due) {
        return;
    }
    core.slot = Some(due.clone());
    if core.find(&due.1).is_none() {
        log(format!("schedule {}: no wallpaper {}", due.0, due.1));
        return;
    }
    log(format!("schedule {}: changing to {}", due.0, due.1));
    for d in 0..core.displays.len() {
        if let Err(e) = apply(app, core, d, &due.1) {
            log(format!("schedule: {e}"));
        }
    }
    core.changed_at = std::time::Instant::now();
    let _ = app.emit_to("main", "changed", ());
}
