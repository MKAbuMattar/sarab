//! The engine thread: wallpapers, pausing, tray. It owns every WebView2 and runs its own message
//! loop, so the settings window (GPUI, main thread) never re-enters it and never waits on it.
//! Anything else talks to it by queueing a job through `Handle`.

pub mod presets;
pub mod update;
pub mod wallpaper;

use crate::model::cli::{self, Command};
use crate::model::library::{self, Wallpaper};
use crate::model::pause::{Reason, State};
use crate::model::settings::{self, Settings};
use crate::platform::desktop as os;
use crate::platform::{autostart, event_loop::Waker};
use futures::channel::{mpsc as amsc, oneshot};
use presets::Video;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use wallpaper::{Core, log};

pub const AUTOSTART_FLAG: &str = "--autostart";
pub const WEBSITE: &str = "https://github.com/MKAbuMattar/sarab";
pub const ISSUES: &str = "https://github.com/MKAbuMattar/sarab/issues";

type Job = Box<dyn FnOnce(&mut Engine) + Send>;

#[derive(Clone)]
pub struct Handle {
    tx: mpsc::Sender<Job>,
    waker: Waker,
}

impl Handle {
    pub fn run(&self, f: impl FnOnce(&mut Engine) + Send + 'static) {
        let _ = self.tx.send(Box::new(f));
        self.waker.wake();
    }
    /// Run on the engine and hand the result back to an async caller.
    pub fn ask<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Engine) -> T + Send + 'static,
    ) -> oneshot::Receiver<T> {
        let (tx, rx) = oneshot::channel();
        self.run(move |e| {
            let _ = tx.send(f(e));
        });
        rx
    }
}

/// What the engine tells the settings window.
pub enum ToUi {
    State(Arc<Snapshot>),
    Open,
    Quit,
}

#[derive(Clone)]
pub struct DisplayView {
    pub key: String,
    pub wallpaper: Option<String>,
    pub state: State,
    pub reason: Reason,
    pub error: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Clone)]
pub struct PresetView {
    pub video: Video,
    pub installed: bool,
    pub progress: Option<u8>,
}

/// Everything the settings window shows, copied out after each change.
#[derive(Clone)]
pub struct Snapshot {
    pub webview: String,
    pub settings: Settings,
    pub library: Vec<Wallpaper>,
    pub manual: Option<bool>,
    pub autostart: bool,
    pub update: update::View,
    pub presets: Vec<PresetView>,
    pub displays: Vec<DisplayView>,
}

pub struct View {
    pub host: windows::Win32::Foundation::HWND,
    pub webview: wry::WebView,
}

pub struct Engine {
    pub core: Core,
    /// Wallpaper webviews by label.
    pub views: HashMap<String, View>,
    pub web: wry::WebContext,
    pub handle: Handle,
    pub downloads: BTreeMap<String, u8>,
    pub updates: update::State,
    ui: amsc::UnboundedSender<ToUi>,
    tray: Option<tray_icon::TrayIcon>,
    quit: bool,
}

impl Engine {
    /// Push the current state to the settings window.
    pub fn changed(&self) {
        let c = &self.core;
        let snap = Snapshot {
            webview: wry::webview_version().unwrap_or_default(),
            settings: c.settings.clone(),
            library: c.lib.clone(),
            manual: c.manual,
            autostart: autostart::enabled(),
            update: self.updates.view(),
            presets: presets::catalog()
                .into_iter()
                .map(|v| PresetView {
                    installed: c.lib.iter().any(|w| w.id == v.id),
                    progress: self.downloads.get(&v.id).copied(),
                    video: v,
                })
                .collect(),
            displays: c
                .displays
                .iter()
                .map(|d| DisplayView {
                    key: d.mon.key.clone(),
                    wallpaper: d.wallpaper.clone(),
                    state: d.state,
                    reason: d.reason,
                    error: d.error.clone(),
                    x: d.mon.rect.left,
                    y: d.mon.rect.top,
                    width: d.mon.rect.right - d.mon.rect.left,
                    height: d.mon.rect.bottom - d.mon.rect.top,
                })
                .collect(),
        };
        let _ = self.ui.unbounded_send(ToUi::State(Arc::new(snap)));
    }

    pub fn open_ui(&self) {
        let _ = self.ui.unbounded_send(ToUi::Open);
    }

    pub fn set_tooltip(&self, text: &str) {
        if let Some(t) = &self.tray {
            let _ = t.set_tooltip(Some(text));
        }
    }

    pub fn rescan(&mut self) {
        self.core.lib = wallpaper::scan_all(&self.core.settings);
    }

    /// A library id, or a path/URL that gets added to the library first.
    pub fn resolve(&mut self, target: &str) -> Result<String, String> {
        if self.core.find(target).is_some() {
            return Ok(target.to_string());
        }
        let lib = wallpaper::library_dir(&self.core.settings);
        let w = library::add(&lib, &self.core.lib, target)?;
        self.rescan();
        Ok(w.id)
    }

    pub fn import(&mut self, zip: &str) -> Result<(), String> {
        library::import_zip(
            &wallpaper::library_dir(&self.core.settings),
            std::path::Path::new(zip),
            library::MAX_UNPACKED,
        )?;
        self.rescan();
        Ok(())
    }

    pub fn save_settings(&mut self, new: Settings) -> Result<(), String> {
        let lib_changed = new.library_dir != self.core.settings.library_dir;
        let vol = new.volume;
        self.core.settings = new;
        settings::save(&wallpaper::cfg("settings.json"), &self.core.settings)
            .map_err(|e| e.to_string())?;
        if lib_changed {
            self.rescan();
        }
        wallpaper::set_volume(self, vol);
        wallpaper::set_fps(self);
        wallpaper::tick(self);
        self.changed();
        Ok(())
    }

    pub fn run_command(&mut self, cmd: Command) -> Result<(), String> {
        log(format!("command {cmd:?}"));
        match cmd {
            Command::Set { target, display } => {
                let id = self.resolve(&target)?;
                for i in wallpaper::targets(&self.core, display)? {
                    wallpaper::apply(self, i, &id)?;
                }
            }
            Command::Close { display } => {
                for i in wallpaper::targets(&self.core, display)? {
                    wallpaper::close(self, i);
                }
            }
            Command::Pause => self.core.manual = Some(true),
            Command::Play => self.core.manual = Some(false),
            Command::Resume => self.core.manual = None,
            Command::Toggle => {
                self.core.manual = if self.core.manual == Some(true) {
                    None
                } else {
                    Some(true)
                }
            }
            Command::Prop {
                key,
                value,
                display,
            } => {
                for i in wallpaper::targets(&self.core, display)? {
                    wallpaper::set_prop(self, i, &key, &serde_json::Value::String(value.clone()))?;
                }
            }
            Command::Volume(v) => wallpaper::set_volume(self, v),
            Command::Next => {
                let cur = self.core.displays.first().and_then(|d| d.wallpaper.clone());
                let pos = cur
                    .and_then(|c| self.core.lib.iter().position(|w| w.id == c))
                    .map_or(0, |p| p + 1);
                let Some(next) = self
                    .core
                    .lib
                    .get(pos % self.core.lib.len().max(1))
                    .map(|w| w.id.clone())
                else {
                    return Err("library is empty".into());
                };
                for i in 0..self.core.displays.len() {
                    wallpaper::apply(self, i, &next)?;
                }
            }
            Command::Import(zip) => self.import(&zip)?,
            Command::Ui => self.open_ui(),
            Command::Quit => self.quit = true,
            Command::Status => wallpaper::probe_pages(self),
            // Network work runs on its own thread; the engine only starts it.
            Command::Preset(id) => presets::spawn(self.handle.clone(), id),
            Command::CheckUpdate => update::spawn_check(self.handle.clone()),
            Command::InstallUpdate => update::spawn_install(self.handle.clone()),
        }
        wallpaper::tick(self);
        wallpaper::write_status(&self.core);
        self.changed();
        Ok(())
    }

    pub fn handle_args(&mut self, args: &[String]) {
        match cli::parse(args) {
            Ok(Some(cmd)) => {
                if let Err(e) = self.run_command(cmd) {
                    log(format!("error: {e}"));
                }
            }
            // A plain launch (shortcut, or clicking Sarab while it runs) opens the window.
            // The login launch passes --autostart and stays in the tray.
            Ok(None) if !args.iter().any(|a| a == AUTOSTART_FLAG) => self.open_ui(),
            Ok(None) => {}
            Err(e) => log(format!("error: {e}")),
        }
    }
}

/// A UI string for text Rust shows itself (tray menu, notifications), from the window's files.
pub fn ui_text(lang: &str, key: &str) -> String {
    crate::ui::i18n::Strings::load(lang).get(key).to_string()
}

fn tray(lang: &str, handle: Handle) -> Option<tray_icon::TrayIcon> {
    use tray_icon::menu::{Menu, MenuEvent, MenuItem};
    use tray_icon::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let t = |k: &str| ui_text(lang, k);
    let menu = Menu::new();
    for (id, key) in [
        ("toggle", "tray.toggle"),
        ("next", "tray.next"),
        ("ui", "tray.open"),
        ("quit", "tray.quit"),
    ] {
        let _ = menu.append(&MenuItem::with_id(id, t(key), true, None));
    }
    let h = handle.clone();
    MenuEvent::set_event_handler(Some(move |ev: MenuEvent| {
        let cmd = ev.id().0.clone();
        h.run(move |e| e.handle_args(&[cmd]));
    }));
    TrayIconEvent::set_event_handler(Some(move |ev: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = ev
        {
            handle.run(|e| e.open_ui());
        }
    }));
    let icon = image::load_from_memory(include_bytes!("../../assets/tray.png"))
        .ok()
        .map(|i| i.into_rgba8())
        .and_then(|i| {
            let (w, h) = i.dimensions();
            tray_icon::Icon::from_rgba(i.into_raw(), w, h).ok()
        });
    let mut b = TrayIconBuilder::new()
        .with_id("sarab")
        .with_tooltip("Sarab")
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false);
    if let Some(icon) = icon {
        b = b.with_icon(icon);
    }
    b.build().map_err(|e| log(format!("tray: {e}"))).ok()
}

/// Start the engine thread. `first` are this process's own arguments.
pub fn start(first: Vec<String>, ui: amsc::UnboundedSender<ToUi>) -> Handle {
    let (tx, rx) = mpsc::channel::<Job>();
    let waker = Waker::new();
    let handle = Handle { tx, waker };
    let h = handle.clone();
    std::thread::Builder::new()
        .name("sarab-engine".into())
        .spawn(move || run(rx, waker, h, first, ui))
        .expect("spawn engine");
    handle
}

fn run(
    rx: mpsc::Receiver<Job>,
    waker: Waker,
    handle: Handle,
    first: Vec<String>,
    ui: amsc::UnboundedSender<ToUi>,
) {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
    }
    let mut core = Core::load();
    wallpaper::write_player();
    core.lib = wallpaper::scan_all(&core.settings);
    core.desktop = os::find_desktop();
    if core.desktop.is_none() {
        log("desktop layer (WorkerW) not found; will retry");
    }
    os::refresh_desktop(core.desktop.as_ref());
    let lang = core.settings.language.clone();
    let mut e = Engine {
        core,
        views: HashMap::new(),
        web: wry::WebContext::new(Some(settings::data_dir().join("EBWebView"))),
        handle: handle.clone(),
        downloads: BTreeMap::new(),
        updates: update::State::default(),
        ui,
        tray: None,
        quit: false,
    };
    e.tray = tray(&lang, handle.clone());
    wallpaper::sync_displays(&mut e);
    // Start with Windows is on by default: turned on once, at the first normal launch.
    if !first.iter().any(|a| a == AUTOSTART_FLAG) && !e.core.settings.autostart_set {
        match autostart::set(true, AUTOSTART_FLAG) {
            Ok(()) => log("start with Windows turned on (first launch)"),
            Err(err) => log(format!("start with Windows: {err}")),
        }
        e.core.settings.autostart_set = true;
        let _ = settings::save(&wallpaper::cfg("settings.json"), &e.core.settings);
    }
    update::spawn_daily(handle.clone());
    e.handle_args(&first);
    e.changed();

    // ponytail: 1 s poll for every probe; move lock/power/session to OS notifications if wakeups show up in profiles.
    const TICK: Duration = Duration::from_millis(1000);
    let mut next = Instant::now() + TICK;
    while !e.quit {
        let wait = next.saturating_duration_since(Instant::now()).as_millis() as u32;
        waker.wait(wait);
        while let Ok(job) = rx.try_recv() {
            job(&mut e);
            if e.quit {
                break;
            }
        }
        if Instant::now() >= next {
            wallpaper::tick(&mut e);
            next = Instant::now() + TICK;
        }
    }

    for i in 0..e.core.displays.len() {
        wallpaper::close_window(&mut e, i);
    }
    os::refresh_desktop(e.core.desktop.as_ref());
    e.tray = None;
    log("exit");
    let _ = e.ui.unbounded_send(ToUi::Quit);
}
