use super::*;

pub(in crate::engine::wallpaper) fn sync_groups(
    displays: impl Iterator<Item = (Option<String>, bool)>,
) -> Vec<Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (id, syncable)) in displays.enumerate() {
        if let (Some(id), true) = (id, syncable) {
            groups.entry(id).or_default().push(i);
        }
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

pub(in crate::engine::wallpaper) fn sync_video(app: &AppHandle, core: &Core) {
    let syncable = |d: &Display| {
        let w = d.wallpaper.as_deref().and_then(|id| core.find(id));
        let media = w.is_some_and(|w| match w.info.r#type {
            Kind::Video => true,
            Kind::Url => w
                .info
                .file
                .as_deref()
                .is_some_and(|u| youtube_ids(u).is_some()),
            _ => false,
        });
        media && d.loaded && d.state == State::Play
    };
    let groups = sync_groups(
        core.displays
            .iter()
            .map(|d| (d.wallpaper.clone(), syncable(d))),
    );
    for idx in groups {
        let Some(lead) = window(app, &core.displays[idx[0]]) else {
            continue;
        };
        let followers: Vec<String> = idx[1..]
            .iter()
            .filter_map(|&i| core.displays[i].label.clone())
            .collect();
        let a = app.clone();
        let _ = lead.eval_with_callback("window.__sarab?__sarab.time():null", move |r| {
            if r == "null" || r.is_empty() {
                return;
            }
            for lbl in &followers {
                if let Some(w) = a.get_webview_window(lbl) {
                    let _ = w.eval(format!("window.__sarab&&__sarab.follow({r})"));
                }
            }
        });
    }
}
