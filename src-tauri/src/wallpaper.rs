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
fn sync_video(app: &AppHandle, core: &Core) {
    let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, d) in core.displays.iter().enumerate() {
        let video = d
            .wallpaper
            .as_deref()
            .and_then(|id| core.find(id))
            .is_some_and(|w| w.info.r#type == Kind::Video);
        if video && d.loaded && d.state == State::Play {
            groups
                .entry(d.wallpaper.clone().unwrap_or_default())
                .or_default()
                .push(i);
        }
    }
    for idx in groups.into_values().filter(|g| g.len() > 1) {
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
    core.ticks += 1;
    if core.sync_due || core.ticks.is_multiple_of(5) {
        core.sync_due = false;
        sync_video(app, core);
    }
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
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(cfg("props").join(id));
    core.lib = scan_all(&core.settings);
    Ok(())
}
