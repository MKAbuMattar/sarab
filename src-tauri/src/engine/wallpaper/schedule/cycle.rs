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
    let cur = core.displays.first().and_then(|d| d.wallpaper.clone());
    let s = &core.settings;
    let ids: Vec<String> = core
        .lib
        .iter()
        .filter(|w| {
            s.cycle_category == "all" || w.info.category.as_deref() == Some(&s.cycle_category)
        })
        .filter(|w| w.info.r#type != Kind::App)
        .map(|w| w.id.clone())
        .collect();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    let Some(next) = pick_next(&ids, cur.as_deref(), s.cycle_order == "random", seed) else {
        return Err("no wallpaper to change to".into());
    };
    for i in 0..core.displays.len() {
        apply(app, core, i, &next)?;
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
