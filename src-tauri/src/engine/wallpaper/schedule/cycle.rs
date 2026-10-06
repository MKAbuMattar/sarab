//! Change wallpaper every few minutes.

use super::*;

/// Which wallpaper comes after `current`: the next one in `ids`, wrapping, or with `random`
/// any other one, chosen by `seed`. None when `ids` is empty.
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
        // Never the same one twice in a row: pick among the others.
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

/// The next wallpaper on every display, from the category and in the order Settings asks for.
pub fn next(app: &AppHandle, core: &mut Core) -> Result<(), String> {
    let cur = core.displays.first().and_then(|d| d.wallpaper.clone());
    let s = &core.settings;
    let ids: Vec<String> = core
        .lib
        .iter()
        .filter(|w| {
            s.cycle_category == "all" || w.info.category.as_deref() == Some(&s.cycle_category)
        })
        // A program runs only when the user picks it, never because a timer did.
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

/// Time to move on: cycling is on, its interval has passed, something is playing, and nothing
/// rests. A frozen or covered wallpaper is not seen, so changing it would only cost a page load.
pub(in crate::engine::wallpaper) fn cycle_due(
    minutes: u32,
    since: std::time::Duration,
    playing: bool,
    resting: bool,
) -> bool {
    minutes > 0 && playing && !resting && since.as_secs() >= u64::from(minutes) * 60
}
