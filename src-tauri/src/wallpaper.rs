//! Core state: which wallpaper runs on which display, and in which pause state.
//! Every function here runs on the main thread (see `on_main`), so the lock is never contended.

use crate::library::{self, Kind, Target, Wallpaper};
use crate::os::windows as os;
use crate::pause::{self, Reason, Signals, State};
use crate::settings::{self, Layout, Settings};
use serde_json::{json, Map, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const INJECT: &str = include_str!("inject.js");

#[cfg(windows)]
const APP_ORIGIN: &str = "http://tauri.localhost";

pub struct Display {
    pub mon: os::Monitor,
    pub wallpaper: Option<String>,
    pub state: State,
    pub reason: Reason,
    pub loaded: bool,
    pub error: Option<String>,
    /// Filled by `sarab status`: what the page itself reports.
    pub page: Value,
    /// Label of this display's wallpaper window. Unique per window: `destroy()` frees a label
    /// asynchronously, so reusing one for the replacement window fails.
    pub label: Option<String>,
    /// Since when nobody can see this display's wallpaper (covered, locked, remote).
    pub unseen_since: Option<std::time::Instant>,
    /// Its webview was closed to free memory; it loads again when it would play.
    pub unloaded: bool,
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
    /// When the wallpaper last changed, by the user or by cycling.
    pub changed_at: std::time::Instant,
}

pub type Shared = Mutex<Core>;

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

/// Run `f` on the main thread and wait for its result. Safe to call from the main thread too.
pub fn on_main<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    let a = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&a));
    })
    .expect("event loop gone");
    rx.recv().expect("main thread task dropped")
}

/// Queue `f` behind whatever the main thread is doing now. WebView2 callbacks can fire inside
/// nested message loops while the core lock is held; going through the event loop avoids re-entry.
pub fn later(app: &AppHandle, f: impl FnOnce(&AppHandle, &mut Core) + Send + 'static) {
    let a = app.clone();
    std::thread::spawn(move || {
        let a2 = a.clone();
        let _ = a.run_on_main_thread(move || {
            let st = a2.state::<Shared>();
            let mut core = st.lock().unwrap();
            f(&a2, &mut core);
        });
    });
}

pub fn library_dir(s: &Settings) -> PathBuf {
    s.library_dir
        .clone()
        .unwrap_or_else(|| settings::data_dir().join("Library"))
}

/// Folder of the web scenes that ship with Sarab, set once at startup from the resource dir.
pub static PRESET_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The user's library followed by the bundled presets.
pub fn scan_all(settings: &Settings) -> Vec<Wallpaper> {
    let mut lib = library::scan(&library_dir(settings));
    if let Some(dir) = PRESET_DIR.get() {
        lib.extend(library::scan(dir).into_iter().map(|mut w| {
            w.preset = true;
            w
        }));
    }
    lib
}

impl Core {
    pub fn load() -> Core {
        let settings: Settings = settings::load(&cfg("settings.json"));
        let lib = scan_all(&settings);
        Core {
            layout: settings::load(&cfg("layout.json")),
            restore: settings::load(&cfg("restore.json")),
            settings,
            lib,
            displays: vec![],
            manual: None,
            desktop: None,
            signals: Signals::default(),
            sync_due: false,
            ticks: 0,
            changed_at: std::time::Instant::now(),
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

fn window(app: &AppHandle, d: &Display) -> Option<WebviewWindow> {
    app.get_webview_window(d.label.as_deref()?)
}

fn key_file(key: &str) -> String {
    key.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

pub fn saved_props_path(id: &str, key: &str) -> PathBuf {
    cfg("props")
        .join(id)
        .join(format!("{}.json", key_file(key)))
}

/// `http://asset.localhost/C:/dir/index.html`: forward slashes keep relative links in web wallpapers working.
fn asset_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut out = String::from("http://asset.localhost/");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn player_url(src: &str, kind: &str, fit: &str, clip: Option<[f64; 2]>) -> String {
    let q: String = src
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    let range = clip.map_or(String::new(), |[a, b]| format!("&start={a}&end={b}"));
    format!("{APP_ORIGIN}/player.html?kind={kind}&fit={fit}{range}&src={q}")
}

/// The video and playlist ids in a YouTube link: watch, youtu.be, shorts, live, embed and
/// playlist forms, on www, m and music. None for anything else.
fn youtube_ids(u: &str) -> Option<(Option<String>, Option<String>)> {
    let rest = u.split_once("://").map_or(u, |(_, r)| r);
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host
        .trim_start_matches("www.")
        .trim_start_matches("m.")
        .trim_start_matches("music.");
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let id_chars = |s: &str| -> Option<String> {
        let id: String = s
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        (!id.is_empty()).then_some(id)
    };
    let param = |name: &str| {
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('='))
            .and_then(id_chars)
    };
    let video = match host {
        "youtu.be" => id_chars(path),
        "youtube.com" | "youtube-nocookie.com" => match path.split_once('/') {
            Some(("shorts" | "live" | "embed" | "v", id)) => id_chars(id),
            _ if path == "watch" => param("v"),
            _ => None,
        },
        _ => return None,
    };
    let list = param("list");
    (video.is_some() || list.is_some()).then_some((video, list))
}

/// Shadertoy pages are heavy; the embed form shows only the shader. YouTube refuses to play
/// when loaded directly (Error 153), so its links open Sarab's own page, which frames the player.
fn rewrite_url(u: &str) -> String {
    if let Some(id) = u.split("shadertoy.com/view/").nth(1) {
        return format!(
            "https://www.shadertoy.com/embed/{}?gui=false&paused=false&muted=true",
            id.trim_end_matches('/')
        );
    }
    if let Some((video, list)) = youtube_ids(u) {
        let mut q = vec![];
        q.extend(video.map(|v| format!("v={v}")));
        q.extend(list.map(|l| format!("list={l}")));
        return format!("{APP_ORIGIN}/youtube.html?{}", q.join("&"));
    }
    u.to_string()
}

/// Build the URL for a wallpaper and open the asset scope for exactly the files it needs.
fn url_for(app: &AppHandle, w: &Wallpaper, fit: &str) -> Result<tauri::Url, String> {
    let scope = app.asset_protocol_scope();
    let s = match (
        w.target().ok_or("wallpaper has no FileName")?,
        w.info.r#type,
    ) {
        (Target::Url(u), _) => rewrite_url(&u),
        (Target::File(f), k @ (Kind::Video | Kind::Gif)) => {
            if !f.is_file() {
                return Err(format!("missing file {}", f.display()));
            }
            scope.allow_file(&f).map_err(|e| e.to_string())?;
            player_url(
                &asset_url(&f),
                if k == Kind::Gif { "gif" } else { "video" },
                fit,
                w.info.clip,
            )
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
            scope
                .allow_directory(&root, true)
                .map_err(|e| e.to_string())?;
            asset_url(&f)
        }
        (_, k) => return Err(format!("{} wallpapers are not supported yet", k.name())),
    };
    tauri::Url::parse(&s).map_err(|e| e.to_string())
}

// ---- host -> page calls ----

/// Call a page function if it exists. serde_json output is a valid JS literal, so page data cannot break out.
fn call(win: &WebviewWindow, func: &str, args: &[Value]) {
    let a = args
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let _ = win.eval(format!(
        "try{{typeof {func}==='function'&&{func}({a})}}catch(e){{console.error(e)}}"
    ));
}

fn push_props(win: &WebviewWindow, w: &Wallpaper, key: &str) {
    for (name, ctl) in library::props(w, &saved_props_path(&w.id, key)) {
        let ty = ctl.get("type").and_then(Value::as_str).unwrap_or("");
        let v = ctl.get("value").cloned().unwrap_or(Value::Null);
        let v = match ty {
            "button" | "label" => continue,
            "folderDropdown" => {
                let rel = ctl
                    .get("folder")
                    .and_then(Value::as_str)
                    .zip(v.as_str())
                    .map(|(f, v)| format!("{f}/{v}"));
                match rel {
                    Some(r) if w.dir.join(&r).is_file() => Value::String(r),
                    _ => Value::Null,
                }
            }
            _ => v,
        };
        call(win, "sarabPropertyChanged", &[Value::String(name), v]);
    }
}

fn apply_state(app: &AppHandle, core: &mut Core, i: usize, st: State, force: bool) {
    let prev = core.displays[i].state;
    if prev == st && !force {
        return;
    }
    core.displays[i].state = st;
    if st == State::Play {
        core.sync_due = true;
    }
    let Some(win) = window(app, &core.displays[i]) else {
        return;
    };
    // Every pause freezes in place. Hiding the window would show the plain Windows wallpaper
    // whenever the covering app minimizes, alt-tabs, or is see-through, and frozen already costs ~0 CPU.
    let paused = st != State::Play;
    let _ = win.eval(if paused {
        "window.__sarab&&__sarab.freeze()"
    } else {
        "window.__sarab&&__sarab.unfreeze()"
    });
    call(&win, "sarabPlaybackChanged", &[json!({ "paused": paused })]);
    log(format!("display {i} {:?} -> {st:?}", prev));
}

/// Page finished loading: send everything the page needs, then its current pause state.
pub fn on_loaded(app: &AppHandle, core: &mut Core, lbl: &str) {
    let Some(i) = core
        .displays
        .iter()
        .position(|d| d.label.as_deref() == Some(lbl))
    else {
        return;
    };
    core.displays[i].loaded = true;
    core.sync_due = true;
    let (Some(win), Some(id)) = (
        window(app, &core.displays[i]),
        core.displays[i].wallpaper.clone(),
    ) else {
        return;
    };
    let _ = win.eval(format!(
        "window.__sarab&&(__sarab.setFps({}),__sarab.volume({}))",
        core.settings.fps, core.settings.volume
    ));
    if let Some(w) = core.find(&id).cloned() {
        push_props(&win, &w, &core.displays[i].mon.key);
    }
    let st = core.displays[i].state;
    apply_state(app, core, i, st, true);
    write_status(core);
}

// ---- apply / close ----

fn close_window(app: &AppHandle, core: &mut Core, i: usize) {
    if let Some(win) = window(app, &core.displays[i]) {
        let _ = win.destroy();
    }
    let d = &mut core.displays[i];
    d.loaded = false;
    d.page = Value::Null;
    d.label = None;
}

fn restore_picture(core: &mut Core, i: usize) {
    let key = core.displays[i].mon.key.clone();
    if let Some(orig) = core.restore.remove(&key) {
        if let Err(e) = os::set_picture(&core.displays[i].mon, &orig) {
            log(format!("restore picture {key}: {e}"));
        }
        core.save_restore();
    }
}

pub fn apply(app: &AppHandle, core: &mut Core, i: usize, id: &str) -> Result<(), String> {
    let w = core
        .find(id)
        .cloned()
        .ok_or_else(|| format!("no wallpaper {id}"))?;
    close_window(app, core, i);
    let key = core.displays[i].mon.key.clone();
    core.displays[i].error = None;
    let result = if w.info.r#type == Kind::Picture {
        let Some(Target::File(f)) = w.target() else {
            return Err("picture has no file".into());
        };
        if !core.restore.contains_key(&key) {
            if let Some(orig) = os::get_picture(&core.displays[i].mon) {
                core.restore.insert(key.clone(), orig);
                core.save_restore();
            }
        }
        os::set_picture(&core.displays[i].mon, &f.to_string_lossy()).map_err(|e| e.to_string())
    } else {
        create_window(app, core, i, &w)
    };
    match &result {
        Ok(()) => {
            core.displays[i].wallpaper = Some(w.id.clone());
            core.layout.insert(key, w.id.clone());
            core.save_layout();
        }
        Err(e) => {
            core.displays[i].error = Some(e.clone());
            log(format!("apply {id} on display {i}: {e}"));
        }
    }
    write_status(core);
    result
}

fn create_window(app: &AppHandle, core: &mut Core, i: usize, w: &Wallpaper) -> Result<(), String> {
    let desk = core.desktop.ok_or("desktop layer not found")?;
    let url = url_for(app, w, &core.settings.scaling)?;
    let origin = url.origin();
    let any_origin = w.info.r#type == Kind::Url;
    let lbl = new_label(i);
    let win = WebviewWindowBuilder::new(app, &lbl, WebviewUrl::External(url))
        .title(format!("sarab-wp-{i}"))
        .decorations(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .shadow(false)
        .resizable(false)
        .initialization_script(INJECT)
        // Local wallpapers stay on their own origin. URL wallpapers may redirect (embeds, logins).
        .on_navigation(move |u| any_origin || u.origin() == origin)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .on_page_load(|win, payload| {
            if payload.event() == tauri::webview::PageLoadEvent::Finished {
                let lbl = win.label().to_string();
                later(win.app_handle(), move |app, core| {
                    on_loaded(app, core, &lbl)
                });
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    core.displays[i].label = Some(lbl);
    let attached = win.hwnd().map_err(|e| e.to_string()).and_then(|hwnd| {
        os::attach(&desk, hwnd, core.displays[i].mon.rect).map_err(|e| e.to_string())?;
        os::show(hwnd, true);
        Ok(())
    });
    // A window that failed to embed would float over the desktop; drop it.
    if attached.is_err() {
        close_window(app, core, i);
    }
    attached
}

pub fn close(app: &AppHandle, core: &mut Core, i: usize) {
    close_window(app, core, i);
    restore_picture(core, i);
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

pub fn set_prop(
    app: &AppHandle,
    core: &mut Core,
    i: usize,
    key: &str,
    raw: &Value,
) -> Result<Value, String> {
    let id = core.displays[i]
        .wallpaper
        .clone()
        .ok_or("no wallpaper on that display")?;
    let w = core.find(&id).cloned().ok_or("wallpaper missing")?;
    let dkey = core.displays[i].mon.key.clone();
    let saved = saved_props_path(&id, &dkey);
    let ctls = library::props(&w, &saved);
    let ctl = ctls.get(key).ok_or_else(|| format!("no property {key}"))?;
    let v = library::coerce(ctl, raw)?;
    library::save_prop(&saved, ctl, key, &v).map_err(|e| e.to_string())?;
    if let Some(win) = window(app, &core.displays[i]) {
        let send = if ctl.get("type").and_then(Value::as_str) == Some("folderDropdown") {
            ctl.get("folder")
                .and_then(Value::as_str)
                .zip(v.as_str())
                .map(|(f, v)| Value::String(format!("{f}/{v}")))
                .unwrap_or(Value::Null)
        } else {
            v.clone()
        };
        call(
            &win,
            "sarabPropertyChanged",
            &[Value::String(key.into()), send],
        );
    }
    Ok(v)
}

/// Back to the wallpaper's own defaults on display `i`, and tell the page.
pub fn reset_props(app: &AppHandle, core: &mut Core, i: usize) -> Result<(), String> {
    let id = core.displays[i]
        .wallpaper
        .clone()
        .ok_or("no wallpaper on that display")?;
    let w = core.find(&id).cloned().ok_or("wallpaper missing")?;
    library::reset_props(&saved_props_path(&id, &core.displays[i].mon.key))
        .map_err(|e| e.to_string())?;
    if let Some(win) = window(app, &core.displays[i]) {
        push_props(&win, &w, &core.displays[i].mon.key);
    }
    Ok(())
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

pub fn set_volume(app: &AppHandle, core: &mut Core, v: u8) {
    core.settings.volume = v;
    let _ = settings::save(&cfg("settings.json"), &core.settings);
    for i in 0..core.displays.len() {
        if let Some(win) = window(app, &core.displays[i]) {
            let _ = win.eval(format!("window.__sarab&&__sarab.volume({v})"));
        }
    }
}

pub fn set_fps(app: &AppHandle, core: &Core) {
    for i in 0..core.displays.len() {
        if let Some(win) = window(app, &core.displays[i]) {
            let _ = win.eval(format!(
                "window.__sarab&&__sarab.setFps({})",
                core.settings.fps
            ));
        }
    }
}

// ---- displays, pause tick, status ----

/// Reconcile displays with the OS: new monitors, removed monitors, Explorer restarts.
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

/// The same video on several displays plays in step: the leftmost playing display leads, the
/// others follow. Displays are independent webviews, so without this they start at different
/// moments and drift further apart every time one is paused and resumed.
/// Displays to keep in step: those playing the same video or YouTube link, two or more to a
/// group, leftmost first. Each entry is a display's wallpaper id and whether it can be synced now.
fn sync_groups(displays: impl Iterator<Item = (Option<String>, bool)>) -> Vec<Vec<usize>> {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (id, syncable)) in displays.enumerate() {
        if let (Some(id), true) = (id, syncable) {
            groups.entry(id).or_default().push(i);
        }
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

fn sync_video(app: &AppHandle, core: &Core) {
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

/// Reasons that mean nobody can see the wallpaper. Every other pause leaves it visible.
fn unseen(why: Reason) -> bool {
    matches!(why, Reason::Covered | Reason::Locked | Reason::Remote)
}

/// Unload when the setting is on and the wallpaper has been out of sight that long. Only then:
/// unloading a visible one would show the plain Windows wallpaper.
fn should_unload(why: Reason, unseen_for: std::time::Duration, minutes: u32) -> bool {
    minutes > 0 && unseen(why) && unseen_for.as_secs() >= u64::from(minutes) * 60
}

/// Free or bring back each display's webview as its visibility changes.
fn unload_or_reload(app: &AppHandle, core: &mut Core) {
    for i in 0..core.displays.len() {
        let d = &mut core.displays[i];
        let why = d.reason;
        if unseen(why) {
            d.unseen_since.get_or_insert_with(std::time::Instant::now);
        } else {
            d.unseen_since = None;
        }
        let unseen_for = d.unseen_since.map(|t| t.elapsed()).unwrap_or_default();
        if !d.unloaded
            && d.label.is_some()
            && should_unload(why, unseen_for, core.settings.unload_minutes)
        {
            log(format!(
                "display {i}: unloading its wallpaper, unseen for {}s",
                unseen_for.as_secs()
            ));
            close_window(app, core, i);
            core.displays[i].unloaded = true;
        } else if d.unloaded && !unseen(why) {
            core.displays[i].unloaded = false;
            if let Some(id) = core.displays[i].wallpaper.clone() {
                log(format!("display {i}: loading its wallpaper again"));
                let _ = apply(app, core, i, &id);
            }
        }
    }
}

/// Which wallpaper comes after `current`: the next one in `ids`, wrapping, or with `random`
/// any other one, chosen by `seed`. None when `ids` is empty.
fn pick_next(ids: &[String], current: Option<&str>, random: bool, seed: u64) -> Option<String> {
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
fn cycle_due(minutes: u32, since: std::time::Duration, playing: bool, resting: bool) -> bool {
    minutes > 0 && playing && !resting && since.as_secs() >= u64::from(minutes) * 60
}

pub fn tick(app: &AppHandle, core: &mut Core) {
    sync_displays(app, core);
    if let Some(d) = core.desktop {
        let wins: Vec<_> = core
            .displays
            .iter()
            .filter_map(|x| window(app, x))
            .filter_map(|w| w.hwnd().ok())
            .collect();
        if os::ensure_order(&d, &wins) {
            log("desktop z-order repaired: a wallpaper had fallen below WorkerW or was hidden");
        }
    }
    let mons: Vec<os::Monitor> = core.displays.iter().map(|d| d.mon.clone()).collect();
    let mut s = os::signals(&mons);
    s.manual = core.manual;
    let decisions = pause::decide(&s, &core.settings);
    let changed = s != core.signals
        || decisions
            .iter()
            .zip(&core.displays)
            .any(|((st, why), d)| *st != d.state || *why != d.reason);
    core.signals = s;
    for (i, (st, why)) in decisions.into_iter().enumerate() {
        core.displays[i].reason = why;
        apply_state(app, core, i, st, false);
    }
    unload_or_reload(app, core);
    core.ticks += 1;
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
        // Wait a full interval before trying again, even after a failure.
        core.changed_at = std::time::Instant::now();
        let _ = app.emit_to("main", "changed", ());
    }
    // Every tick: a follower corrects its speed once a second, so it holds within a frame.
    core.sync_due = false;
    sync_video(app, core);
    if changed {
        write_status(core);
        let _ = app.emit_to("main", "changed", ());
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
    if let Err(e) = settings::save(&cfg("status.json"), &v) {
        log(format!("status: {e}"));
    }
}

/// Ask every page for its own report (frame count, video time); results land in status.json.
pub fn probe_pages(app: &AppHandle, core: &Core) {
    for i in 0..core.displays.len() {
        let Some(win) = window(app, &core.displays[i]) else {
            continue;
        };
        let a = app.clone();
        let _ = win.eval_with_callback("window.__sarab?__sarab.status():null", move |r| {
            let v: Value = serde_json::from_str(&r).unwrap_or(Value::Null);
            later(&a, move |_, core| {
                if let Some(d) = core.displays.get_mut(i) {
                    d.page = v;
                }
                write_status(core);
            });
        });
    }
}

pub fn remove(app: &AppHandle, core: &mut Core, id: &str) -> Result<(), String> {
    let w = core.find(id).cloned().ok_or("not found")?;
    let lib = library_dir(&core.settings);
    // Never delete outside the library folder, whatever the id says.
    let dir = fs::canonicalize(&w.dir).map_err(|e| e.to_string())?;
    let root = fs::canonicalize(&lib).map_err(|e| e.to_string())?;
    if !dir.starts_with(&root) || dir == root {
        return Err("refusing to delete outside the library".into());
    }
    for i in 0..core.displays.len() {
        if core.displays[i].wallpaper.as_deref() == Some(id) {
            close(app, core, i);
        }
    }
    // To the Recycle Bin, so a wrong click can be undone from Explorer.
    os::recycle(&dir)?;
    let _ = fs::remove_dir_all(cfg("props").join(id));
    core.lib = scan_all(&core.settings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_unload_only_when_unseen() {
        use std::time::Duration;
        let m = |n: u64| Duration::from_secs(n * 60);
        assert!(should_unload(Reason::Covered, m(5), 5));
        assert!(should_unload(Reason::Locked, m(9), 5));
        assert!(should_unload(Reason::Remote, m(5), 5));
        assert!(!should_unload(Reason::Covered, m(4), 5), "too soon");
        assert!(!should_unload(Reason::Covered, m(60), 0), "off");
        // Paused but still on screen: unloading would show the Windows wallpaper.
        for why in [
            Reason::Battery,
            Reason::PowerSaver,
            Reason::Focus,
            Reason::Manual,
            Reason::AppPause,
            Reason::OtherCovered,
            Reason::None,
        ] {
            assert!(!should_unload(why, m(60), 5), "{why:?}");
        }
    }

    #[test]
    fn pick_next_order_random_category() {
        let ids: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(pick_next(&ids, Some("a"), false, 0).as_deref(), Some("b"));
        assert_eq!(
            pick_next(&ids, Some("c"), false, 0).as_deref(),
            Some("a"),
            "wraps"
        );
        assert_eq!(pick_next(&ids, None, false, 0).as_deref(), Some("a"));
        assert_eq!(
            pick_next(&ids, Some("gone"), false, 0).as_deref(),
            Some("a")
        );
        assert_eq!(pick_next(&[], None, true, 5), None);
        // Random never repeats the current one and reaches every other one.
        for cur in ["a", "b", "c"] {
            let seen: std::collections::BTreeSet<String> = (0..20)
                .filter_map(|seed| pick_next(&ids, Some(cur), true, seed))
                .collect();
            assert!(!seen.contains(cur), "{cur} repeated");
            assert_eq!(seen.len(), 2);
        }
        let one = vec!["a".to_string()];
        assert_eq!(
            pick_next(&one, Some("a"), true, 7).as_deref(),
            Some("a"),
            "only one to show"
        );
        // The category filter happens before picking, in next(); a filtered list behaves the same.
        let nature: Vec<String> = vec!["b".into()];
        assert_eq!(
            pick_next(&nature, Some("a"), false, 0).as_deref(),
            Some("b")
        );
    }

    #[test]
    fn sync_groups_pick_shared_videos() {
        let d = |id: Option<&str>, ok: bool| (id.map(String::from), ok);
        // Displays 0 and 2 share a video; 1 plays something else alone; 3 shares but is resting.
        let groups = sync_groups(
            vec![
                d(Some("a"), true),
                d(Some("b"), true),
                d(Some("a"), true),
                d(Some("a"), false),
            ]
            .into_iter(),
        );
        assert_eq!(groups, vec![vec![0, 2]]);
        assert!(sync_groups(vec![d(Some("a"), true), d(None, true)].into_iter()).is_empty());
    }

    #[test]
    fn cycle_due_only_when_on_playing_and_seen() {
        use std::time::Duration;
        let m = |n: u64| Duration::from_secs(n * 60);
        assert!(cycle_due(15, m(15), true, false));
        assert!(cycle_due(15, m(40), true, false));
        assert!(!cycle_due(15, m(14), true, false), "too early");
        assert!(!cycle_due(0, m(999), true, false), "off");
        assert!(!cycle_due(15, m(15), false, false), "nothing playing");
        assert!(!cycle_due(15, m(15), true, true), "resting");
    }

    #[test]
    fn youtube_links() {
        let v = |id: &str| Some((Some(id.to_string()), None));
        assert_eq!(
            youtube_ids("https://www.youtube.com/watch?v=aqz-KE-bpKQ"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://youtube.com/watch?feature=share&v=aqz-KE-bpKQ&t=30"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://youtu.be/aqz-KE-bpKQ?si=x"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://m.youtube.com/watch?v=aqz-KE-bpKQ"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://www.youtube.com/shorts/aqz-KE-bpKQ"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://www.youtube.com/live/jfKfPfyJRdk?si=y"),
            v("jfKfPfyJRdk")
        );
        assert_eq!(
            youtube_ids("https://www.youtube.com/embed/aqz-KE-bpKQ"),
            v("aqz-KE-bpKQ")
        );
        assert_eq!(
            youtube_ids("https://www.youtube.com/playlist?list=PL123_ab-C"),
            Some((None, Some("PL123_ab-C".into())))
        );
        assert_eq!(
            youtube_ids("https://www.youtube.com/watch?v=aqz-KE-bpKQ&list=PL123"),
            Some((Some("aqz-KE-bpKQ".into()), Some("PL123".into())))
        );
        // Not YouTube, or YouTube without a video: left alone.
        assert_eq!(youtube_ids("https://www.youtube.com/@blender"), None);
        assert_eq!(
            youtube_ids("https://notyoutube.com/watch?v=aqz-KE-bpKQ"),
            None
        );
        assert_eq!(youtube_ids("https://example.com/youtu.be/x"), None);
        // An id cannot carry anything but id characters into the page URL.
        assert_eq!(youtube_ids("https://youtu.be/abc\"><script>"), v("abc"));
        assert_eq!(
            rewrite_url("https://youtu.be/aqz-KE-bpKQ"),
            "http://tauri.localhost/youtube.html?v=aqz-KE-bpKQ"
        );
        assert_eq!(rewrite_url("https://example.com/"), "https://example.com/");
    }
}
