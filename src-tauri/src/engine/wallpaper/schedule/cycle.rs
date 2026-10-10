use super::*;

pub(in crate::engine::wallpaper) fn pick_next(
    ids: &[String],
    current: Option<&str>,
    random: bool,
    seed: u64,
) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let at = current.and_then(|c| ids.iter().position(|i| i == c));
    let i = if random && ids.len() > 1 {
        let k = (seed % (ids.len() as u64 - u64::from(at.is_some()))) as usize;
        match at {
            Some(a) if k >= a => k + 1,
            _ => k,
        }
    } else {
        at.map_or(0, |p| (p + 1) % ids.len())
    };
    Some(ids[i].clone())
}

pub fn next(app: &AppHandle, core: &mut Core) -> Result<(), String> {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let random = core.settings.cycle_order == "random";
    let count = if core.settings.span {
        core.displays.len().min(1)
    } else {
        core.displays.len()
    };
    let mut picks = vec![];
    for i in 0..count {
        let source = source_for(core, i);
        add_folder(core, &source);
        let ids: Vec<String> = core
            .lib
            .iter()
            .filter(|w| in_source(w, &source))
            .map(|w| w.id.clone())
            .collect();
        let cur = core.displays[i].wallpaper.clone();
        if let Some(id) = pick_next(&ids, cur.as_deref(), random, seed) {
            picks.push((i, id));
        }
    }
    if picks.is_empty() {
        return Err("no wallpaper to change to".into());
    }
    if core.settings.span {
        let id = picks[0].1.clone();
        picks = (0..core.displays.len()).map(|i| (i, id.clone())).collect();
    }
    for (i, id) in picks {
        apply(app, core, i, &id)?;
    }
    core.changed_at = std::time::Instant::now();
    Ok(())
}

pub(in crate::engine::wallpaper) fn cycle_due(
    minutes: u32,
    since: std::time::Duration,
    playing: bool,
    resting: bool,
) -> bool {
    minutes > 0 && playing && !resting && since.as_secs() >= u64::from(minutes) * 60
}
