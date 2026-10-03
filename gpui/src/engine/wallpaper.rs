//! Core state: which wallpaper runs on which display, and in which pause state.
//! Every function here runs on the engine thread.

use crate::engine::{Engine, View};
use crate::model::library::{self, Kind, Target, Wallpaper};
use crate::model::pause::{self, Reason, Signals, State};
use crate::model::settings::{self, Layout, Settings};
use crate::platform::desktop as os;
use crate::platform::host;
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

pub const INJECT: &str = include_str!("inject.js");

/// The player page, served from a folder Sarab writes at startup.
const APP_HOST: &str = "app.sarab";
/// The wallpaper's own files. Each webview maps this to its wallpaper's folder only.
const WP_HOST: &str = "wallpaper.sarab";

pub struct Display {
    pub mon: os::Monitor,
    pub wallpaper: Option<String>,
    pub state: State,
    pub reason: Reason,
    pub loaded: bool,
    pub error: Option<String>,
    /// Filled by `sarab status`: what the page itself reports.
    pub page: Value,
    /// Label of this display's wallpaper webview, unique per webview.
    pub label: Option<String>,
}

pub struct Core {
    pub settings: Settings,
    pub layout: Layout,
    pub lib: Vec<Wallpaper>,
    pub displays: Vec<Display>,
    pub manual: Option<bool>,
    pub desktop: Option<os::Desktop>,
    /// Display key -> the OS wallpaper before Sarab changed it, so close can put it back.
    pub restore: BTreeMap<String, String>,
    pub signals: Signals,
    /// Run video sync on the next tick (a display just resumed or loaded).
    pub sync_due: bool,
    pub ticks: u64,
}

pub fn cfg(name: &str) -> PathBuf {
    settings::config_dir().join(name)
}

pub fn log(msg: impl AsRef<str>) {
    use std::io::Write;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = fs::create_dir_all(settings::config_dir());
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(cfg("sarab.log"))
    {
        let _ = writeln!(f, "{secs} {}", msg.as_ref());
    }
}

pub fn library_dir(s: &Settings) -> PathBuf {
    s.library_dir
        .clone()
        .unwrap_or_else(|| settings::data_dir().join("Library"))
}

fn player_dir() -> PathBuf {
    settings::data_dir().join("player")
}

/// The player is part of the app; write it where WebView2 can serve it.
pub fn write_player() {
    let dir = player_dir();
    let _ = fs::create_dir_all(&dir);
    for (name, body) in [
        ("player.html", include_str!("../../assets/player.html")),
        ("player.js", include_str!("../../assets/player.js")),
    ] {
        if let Err(e) = fs::write(dir.join(name), body) {
            log(format!("write {name}: {e}"));
        }
    }
}

/// Folder of the web scenes that ship next to sarab.exe.
pub fn preset_dir() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join("presets"))
}

/// The user's library followed by the bundled presets.
pub fn scan_all(settings: &Settings) -> Vec<Wallpaper> {
    let mut lib = library::scan(&library_dir(settings));
    if let Some(dir) = preset_dir() {
        lib.extend(library::scan(&dir).into_iter().map(|mut w| {
            w.preset = true;
            w
        }));
    }
    lib
}

impl Core {
    pub fn load() -> Core {
        let settings: Settings = settings::load(&cfg("settings.json"));
        Core {
            layout: settings::load(&cfg("layout.json")),
            restore: settings::load(&cfg("restore.json")),
            settings,
            lib: vec![],
            displays: vec![],
            manual: None,
            desktop: None,
            signals: Signals::default(),
            sync_due: false,
            ticks: 0,
        }
    }
    pub fn find(&self, id: &str) -> Option<&Wallpaper> {
        self.lib.iter().find(|w| w.id == id)
    }
    fn save_layout(&self) {
        if let Err(e) = settings::save(&cfg("layout.json"), &self.layout) {
            log(format!("save layout: {e}"));
        }
    }
    fn save_restore(&self) {
        if let Err(e) = settings::save(&cfg("restore.json"), &self.restore) {
            log(format!("save restore: {e}"));
        }
    }
}

fn new_label(i: usize) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    format!(
        "wp-{i}-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

fn view(e: &Engine, i: usize) -> Option<&View> {
    e.views.get(e.core.displays.get(i)?.label.as_deref()?)
}

fn key_file(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

pub fn saved_props_path(id: &str, key: &str) -> PathBuf {
    cfg("props")
        .join(id)
        .join(format!("{}.json", key_file(key)))
}

fn encode(s: &str, keep_slash: bool) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            b'/' if keep_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `https://wallpaper.sarab/dir/index.html` for a file under `root`.
fn wp_url(root: &Path, file: &Path) -> Result<String, String> {
    let rel = file
        .strip_prefix(root)
        .map_err(|_| "file is outside its folder")?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    Ok(format!("https://{WP_HOST}/{}", encode(&rel, true)))
}

/// Shadertoy and YouTube pages are heavy; their embed forms show only the content.
fn rewrite_url(u: &str) -> String {
    if let Some(id) = u.split("shadertoy.com/view/").nth(1) {
        return format!(
            "https://www.shadertoy.com/embed/{}?gui=false&paused=false&muted=true",
            id.trim_end_matches('/')
        );
    }
    let yt = u
        .split("youtube.com/watch?v=")
        .nth(1)
        .or_else(|| u.split("youtu.be/").nth(1));
    if let Some(id) = yt {
        let id: String = id
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        return format!(
            "https://www.youtube.com/embed/{id}?autoplay=1&mute=1&loop=1&playlist={id}&controls=0"
        );
    }
    u.to_string()
}

/// The URL to load, and the folders the webview may read: exactly the ones the wallpaper needs.
/// The URL to load and the folders (host name, folder) it may read.
type Plan = (String, Vec<(&'static str, PathBuf)>);

fn url_for(w: &Wallpaper, fit: &str) -> Result<Plan, String> {
    match (
        w.target().ok_or("wallpaper has no FileName")?,
        w.info.r#type,
    ) {
        (Target::Url(u), _) => Ok((rewrite_url(&u), vec![])),
        (Target::File(f), k @ (Kind::Video | Kind::Gif)) => {
            if !f.is_file() {
                return Err(format!("missing file {}", f.display()));
            }
            let dir = f.parent().ok_or("file has no folder")?.to_path_buf();
            let src = wp_url(&dir, &f)?;
            let range = w
                .info
                .clip
                .map_or(String::new(), |[a, b]| format!("&start={a}&end={b}"));
            let kind = if k == Kind::Gif { "gif" } else { "video" };
            let url = format!(
                "https://{APP_HOST}/player.html?kind={kind}&fit={fit}{range}&src={}",
                encode(&src, false)
            );
            Ok((url, vec![(APP_HOST, player_dir()), (WP_HOST, dir)]))
        }
        (Target::File(f), Kind::Web) => {
            if !f.is_file() {
                return Err(format!("missing file {}", f.display()));
            }
            let root = if w.info.external {
                f.parent().unwrap_or(&f).to_path_buf()
            } else {
                w.dir.clone()
            };
            Ok((wp_url(&root, &f)?, vec![(WP_HOST, root)]))
        }
        (_, k) => Err(format!("{} wallpapers are not supported yet", k.name())),
    }
}

fn origin(u: &str) -> String {
    u.splitn(4, '/').take(3).collect::<Vec<_>>().join("/") + "/"
}

// ---- host -> page calls ----

fn eval(v: &View, js: &str) {
    let _ = v.webview.evaluate_script(js);
}

/// Call a page function if it exists. serde_json output is a valid JS literal, so page data cannot break out.
fn call(v: &View, func: &str, args: &[Value]) {
    let a = args
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(",");
    eval(
        v,
        &format!("try{{typeof {func}==='function'&&{func}({a})}}catch(e){{console.error(e)}}"),
    );
}

fn push_props(v: &View, w: &Wallpaper, key: &str) {
    for (name, ctl) in library::props(w, &saved_props_path(&w.id, key)) {
        let ty = ctl.get("type").and_then(Value::as_str).unwrap_or("");
        let val = ctl.get("value").cloned().unwrap_or(Value::Null);
        let val = match ty {
            "button" | "label" => continue,
            "folderDropdown" => {
                let rel = ctl
                    .get("folder")
                    .and_then(Value::as_str)
                    .zip(val.as_str())
                    .map(|(f, v)| format!("{f}/{v}"));
                match rel {
                    Some(r) if w.dir.join(&r).is_file() => Value::String(r),
                    _ => Value::Null,
                }
            }
            _ => val,
        };
        call(v, "sarabPropertyChanged", &[Value::String(name), val]);
    }
}

fn apply_state(e: &mut Engine, i: usize, st: State, force: bool) {
    let prev = e.core.displays[i].state;
    if prev == st && !force {
        return;
    }
    e.core.displays[i].state = st;
    if st == State::Play {
        e.core.sync_due = true;
    }
    let Some(v) = view(e, i) else { return };
    // Every pause freezes in place. Hiding the window would show the plain Windows wallpaper
    // whenever the covering app minimizes, alt-tabs, or is see-through, and frozen already costs ~0 CPU.
    let paused = st != State::Play;
    eval(
        v,
        if paused {
            "window.__sarab&&__sarab.freeze()"
        } else {
            "window.__sarab&&__sarab.unfreeze()"
        },
    );
    call(v, "sarabPlaybackChanged", &[json!({ "paused": paused })]);
    log(format!("display {i} {prev:?} -> {st:?}"));
}

/// Page finished loading: send everything the page needs, then its current pause state.
pub fn on_loaded(e: &mut Engine, lbl: &str) {
    let Some(i) = e
        .core
        .displays
        .iter()
        .position(|d| d.label.as_deref() == Some(lbl))
    else {
        return;
    };
    e.core.displays[i].loaded = true;
    e.core.sync_due = true;
    let (Some(v), Some(id)) = (view(e, i), e.core.displays[i].wallpaper.clone()) else {
        return;
    };
    eval(
        v,
        &format!(
            "window.__sarab&&(__sarab.setFps({}),__sarab.volume({}))",
            e.core.settings.fps, e.core.settings.volume
        ),
    );
    if let Some(w) = e.core.find(&id) {
        push_props(v, w, &e.core.displays[i].mon.key);
    }
    let st = e.core.displays[i].state;
    apply_state(e, i, st, true);
    write_status(&e.core);
}

// ---- apply / close ----

pub fn close_window(e: &mut Engine, i: usize) {
    if let Some(lbl) = e.core.displays[i].label.take()
        && let Some(v) = e.views.remove(&lbl)
    {
        // The webview goes first; it lives inside the host window.
        drop(v.webview);
        host::destroy(v.host);
    }
    let d = &mut e.core.displays[i];
    d.loaded = false;
    d.page = Value::Null;
}

fn restore_picture(core: &mut Core, i: usize) {
    let key = core.displays[i].mon.key.clone();
    if let Some(orig) = core.restore.remove(&key) {
        if let Err(err) = os::set_picture(&core.displays[i].mon, &orig) {
            log(format!("restore picture {key}: {err}"));
        }
        core.save_restore();
    }
}

pub fn apply(e: &mut Engine, i: usize, id: &str) -> Result<(), String> {
    let w = e
        .core
        .find(id)
        .cloned()
        .ok_or_else(|| format!("no wallpaper {id}"))?;
    close_window(e, i);
    let key = e.core.displays[i].mon.key.clone();
    e.core.displays[i].error = None;
    let result = if w.info.r#type == Kind::Picture {
        let Some(Target::File(f)) = w.target() else {
            return Err("picture has no file".into());
        };
        let core = &mut e.core;
        if !core.restore.contains_key(&key)
            && let Some(orig) = os::get_picture(&core.displays[i].mon)
        {
            core.restore.insert(key.clone(), orig);
            core.save_restore();
        }
        os::set_picture(&core.displays[i].mon, &f.to_string_lossy()).map_err(|e| e.to_string())
    } else {
        create_view(e, i, &w)
    };
    match &result {
        Ok(()) => {
            e.core.displays[i].wallpaper = Some(w.id.clone());
            e.core.layout.insert(key, w.id.clone());
            e.core.save_layout();
        }
        Err(err) => {
            e.core.displays[i].error = Some(err.clone());
            log(format!("apply {id} on display {i}: {err}"));
        }
    }
    write_status(&e.core);
    result
}

fn create_view(e: &mut Engine, i: usize, w: &Wallpaper) -> Result<(), String> {
    use wry::dpi::{PhysicalPosition, PhysicalSize};
    let desk = e.core.desktop.ok_or("desktop layer not found")?;
    let (url, folders) = url_for(w, &e.core.settings.scaling)?;
    let allowed = origin(&url);
    let any_origin = w.info.r#type == Kind::Url;
    let lbl = new_label(i);
    let r = e.core.displays[i].mon.rect;
    let host = host::create_host().map_err(|err| err.to_string())?;
    let h = e.handle.clone();
    let loaded = lbl.clone();
    let built = wry::WebViewBuilder::new_with_web_context(&mut e.web)
        .with_bounds(wry::Rect {
            position: PhysicalPosition::new(0, 0).into(),
            size: PhysicalSize::new((r.right - r.left) as u32, (r.bottom - r.top) as u32).into(),
        })
        .with_initialization_script(INJECT)
        .with_autoplay(true)
        .with_focused(false)
        // Local wallpapers stay on their own origin. URL wallpapers may redirect (embeds, logins).
        .with_navigation_handler(move |u| {
            any_origin || u == "about:blank" || u.starts_with(&allowed)
        })
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .with_on_page_load_handler(move |ev, u| {
            if matches!(ev, wry::PageLoadEvent::Finished) && !u.starts_with("about:") {
                let lbl = loaded.clone();
                h.run(move |e| on_loaded(e, &lbl));
            }
        })
        .build_as_child(&host::Host(host));
    let webview = match built {
        Ok(wv) => wv,
        Err(err) => {
            host::destroy(host);
            return Err(err.to_string());
        }
    };
    for (name, dir) in &folders {
        host::map_folder(&webview, name, dir).map_err(|err| err.to_string())?;
    }
    webview.load_url(&url).map_err(|err| err.to_string())?;
    e.views.insert(lbl.clone(), View { host, webview });
    e.core.displays[i].label = Some(lbl);
    let attached = os::attach(&desk, host, r)
        .map_err(|err| err.to_string())
        .map(|()| os::show(host, true));
    // A window that failed to embed would float over the desktop; drop it.
    if attached.is_err() {
        close_window(e, i);
    }
    attached
}

pub fn close(e: &mut Engine, i: usize) {
    close_window(e, i);
    restore_picture(&mut e.core, i);
    let core = &mut e.core;
    core.displays[i].wallpaper = None;
    core.displays[i].error = None;
    let key = core.displays[i].mon.key.clone();
    core.layout.remove(&key);
    core.save_layout();
    os::refresh_desktop(core.desktop.as_ref());
    write_status(core);
}

/// Display selection from the CLI or UI: one display, or all of them.
pub fn targets(core: &Core, display: Option<usize>) -> Result<Vec<usize>, String> {
    match display {
        Some(n) if n < core.displays.len() => Ok(vec![n]),
        Some(n) => Err(format!("no display {n}; there are {}", core.displays.len())),
        None => Ok((0..core.displays.len()).collect()),
    }
}

// ---- properties ----

pub fn set_prop(e: &mut Engine, i: usize, key: &str, raw: &Value) -> Result<Value, String> {
    let id = e.core.displays[i]
        .wallpaper
        .clone()
        .ok_or("no wallpaper on that display")?;
    let w = e.core.find(&id).cloned().ok_or("wallpaper missing")?;
    let saved = saved_props_path(&id, &e.core.displays[i].mon.key);
    let ctls = library::props(&w, &saved);
    let ctl = ctls.get(key).ok_or_else(|| format!("no property {key}"))?;
    let val = library::coerce(ctl, raw)?;
    library::save_prop(&saved, ctl, key, &val).map_err(|e| e.to_string())?;
    if let Some(v) = view(e, i) {
        let send = if ctl.get("type").and_then(Value::as_str) == Some("folderDropdown") {
            ctl.get("folder")
                .and_then(Value::as_str)
                .zip(val.as_str())
                .map(|(f, v)| Value::String(format!("{f}/{v}")))
                .unwrap_or(Value::Null)
        } else {
            val.clone()
        };
        call(
            v,
            "sarabPropertyChanged",
            &[Value::String(key.into()), send],
        );
    }
    Ok(val)
}

pub fn props_for(core: &Core, i: usize) -> Map<String, Value> {
    let Some(id) = core.displays.get(i).and_then(|d| d.wallpaper.clone()) else {
        return Map::new();
    };
    let Some(w) = core.find(&id) else {
        return Map::new();
    };
    library::props(w, &saved_props_path(&id, &core.displays[i].mon.key))
}

pub fn set_volume(e: &mut Engine, v: u8) {
    e.core.settings.volume = v;
    let _ = settings::save(&cfg("settings.json"), &e.core.settings);
    for view in e.views.values() {
        eval(view, &format!("window.__sarab&&__sarab.volume({v})"));
    }
}

pub fn set_fps(e: &Engine) {
    for view in e.views.values() {
        eval(
            view,
            &format!("window.__sarab&&__sarab.setFps({})", e.core.settings.fps),
        );
    }
}

// ---- displays, pause tick, status ----

/// Reconcile displays with the OS: new monitors, removed monitors, Explorer restarts.
pub fn sync_displays(e: &mut Engine) {
    let mons = os::monitors();
    let desk_ok = e.core.desktop.is_some_and(|d| os::desktop_alive(&d));
    let same = mons.len() == e.core.displays.len()
        && mons.iter().zip(&e.core.displays).all(|(m, d)| *m == d.mon);
    if same && desk_ok {
        return;
    }
    log(format!(
        "displays or desktop changed: {} monitors, desktop alive {desk_ok}",
        mons.len()
    ));
    for i in 0..e.core.displays.len() {
        close_window(e, i);
    }
    if !desk_ok {
        e.core.desktop = os::find_desktop();
    }
    e.core.displays = mons
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
        })
        .collect();
    for i in 0..e.core.displays.len() {
        let key = e.core.displays[i].mon.key.clone();
        if let Some(id) = e.core.layout.get(&key).cloned() {
            if e.core.find(&id).is_some() {
                let _ = apply(e, i, &id);
            } else {
                e.core.displays[i].error = Some(format!("wallpaper {id} is gone"));
            }
        }
    }
    write_status(&e.core);
    e.changed();
}

/// The same video on several displays plays in step: the leftmost playing display leads, the
/// others follow. Displays are independent webviews, so without this they start at different
/// moments and drift further apart every time one is paused and resumed.
fn sync_video(e: &Engine) {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, d) in e.core.displays.iter().enumerate() {
        let video = d
            .wallpaper
            .as_deref()
            .and_then(|id| e.core.find(id))
            .is_some_and(|w| w.info.r#type == Kind::Video);
        if video && d.loaded && d.state == State::Play {
            groups
                .entry(d.wallpaper.clone().unwrap_or_default())
                .or_default()
                .push(i);
        }
    }
    for idx in groups.into_values().filter(|g| g.len() > 1) {
        let Some(lead) = view(e, idx[0]) else {
            continue;
        };
        let followers: Vec<usize> = idx[1..].to_vec();
        let h = e.handle.clone();
        let _ = lead.webview.evaluate_script_with_callback(
            "window.__sarab?__sarab.time():null",
            move |r| {
                if r == "null" || r.is_empty() {
                    return;
                }
                let followers = followers.clone();
                h.run(move |e| {
                    for &i in &followers {
                        if let Some(v) = view(e, i) {
                            eval(v, &format!("window.__sarab&&__sarab.follow({r})"));
                        }
                    }
                });
            },
        );
    }
}

pub fn tick(e: &mut Engine) {
    sync_displays(e);
    if let Some(d) = e.core.desktop {
        let hosts: Vec<_> = e.views.values().map(|v| v.host).collect();
        if os::ensure_order(&d, &hosts) {
            log("desktop z-order repaired: a wallpaper had fallen below WorkerW or was hidden");
        }
    }
    let mons: Vec<os::Monitor> = e.core.displays.iter().map(|d| d.mon.clone()).collect();
    let mut s = os::signals(&mons);
    s.manual = e.core.manual;
    let decisions = pause::decide(&s, &e.core.settings);
    let changed = s != e.core.signals
        || decisions
            .iter()
            .zip(&e.core.displays)
            .any(|((st, why), d)| *st != d.state || *why != d.reason);
    e.core.signals = s;
    for (i, (st, why)) in decisions.into_iter().enumerate() {
        e.core.displays[i].reason = why;
        apply_state(e, i, st, false);
    }
    e.core.ticks += 1;
    if e.core.sync_due || e.core.ticks.is_multiple_of(5) {
        e.core.sync_due = false;
        sync_video(e);
    }
    if changed {
        write_status(&e.core);
        e.changed();
    }
}

pub fn write_status(core: &Core) {
    let displays: Vec<Value> = core
        .displays
        .iter()
        .map(|d| {
            json!({
                "key": d.mon.key,
                "rect": [d.mon.rect.left, d.mon.rect.top, d.mon.rect.right, d.mon.rect.bottom],
                "wallpaper": d.wallpaper,
                "kind": d.wallpaper.as_deref().and_then(|id| core.find(id)).map(|w| w.kind),
                "state": d.state,
                "reason": d.reason,
                "loaded": d.loaded,
                "error": d.error,
                "page": d.page,
            })
        })
        .collect();
    let s = &core.signals;
    let v = json!({
        "pid": std::process::id(),
        "manual": core.manual,
        "volume": core.settings.volume,
        "fps": core.settings.fps,
        "desktop": core.desktop.map(|d| json!({ "raised": d.raised, "progman": d.progman, "workerw": d.workerw, "defview": d.defview })),
        "signals": { "battery": s.on_battery, "power_saver": s.power_saver, "locked": s.locked, "remote": s.remote,
                     "foreground": s.foreground_app, "desktop_focused": s.desktop_focused, "covered": s.covered },
        "displays": displays,
    });
    if let Err(err) = settings::save(&cfg("status.json"), &v) {
        log(format!("status: {err}"));
    }
}

/// Ask every page for its own report (frame count, video time); results land in status.json.
pub fn probe_pages(e: &Engine) {
    for i in 0..e.core.displays.len() {
        let Some(v) = view(e, i) else { continue };
        let h = e.handle.clone();
        let _ = v.webview.evaluate_script_with_callback(
            "window.__sarab?__sarab.status():null",
            move |r| {
                let page: Value = serde_json::from_str(&r).unwrap_or(Value::Null);
                h.run(move |e| {
                    if let Some(d) = e.core.displays.get_mut(i) {
                        d.page = page;
                    }
                    write_status(&e.core);
                });
            },
        );
    }
}

pub fn remove(e: &mut Engine, id: &str) -> Result<(), String> {
    let w = e.core.find(id).cloned().ok_or("not found")?;
    let lib = library_dir(&e.core.settings);
    // Never delete outside the library folder, whatever the id says.
    let dir = fs::canonicalize(&w.dir).map_err(|e| e.to_string())?;
    let root = fs::canonicalize(&lib).map_err(|e| e.to_string())?;
    if !dir.starts_with(&root) || dir == root {
        return Err("refusing to delete outside the library".into());
    }
    for i in 0..e.core.displays.len() {
        if e.core.displays[i].wallpaper.as_deref() == Some(id) {
            close(e, i);
        }
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(cfg("props").join(id));
    e.core.lib = scan_all(&e.core.settings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_stay_on_their_host() {
        let root = Path::new(r"C:\lib\my scene");
        assert_eq!(
            wp_url(root, &root.join("index.html")).unwrap(),
            "https://wallpaper.sarab/index.html"
        );
        assert_eq!(
            wp_url(root, &root.join(r"a b\c#d.html")).unwrap(),
            "https://wallpaper.sarab/a%20b/c%23d.html"
        );
        assert!(wp_url(root, Path::new(r"C:\other\x.html")).is_err());
        assert_eq!(
            origin("https://wallpaper.sarab/a/b.html?x"),
            "https://wallpaper.sarab/"
        );
        // A look-alike host must not pass the prefix check.
        assert!(
            !"https://wallpaper.sarab.evil.com/".starts_with(&origin("https://wallpaper.sarab/x"))
        );
    }
}
