#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cli;
mod library;
mod os;
mod pause;
mod presets;
mod settings;
mod update;
mod wallpaper;

use cli::Command;
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, DragDropEvent, Emitter, Manager, RunEvent, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use wallpaper::{log, on_main, Core, Shared};

fn with_core<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle, &mut Core) -> T + Send + 'static,
) -> T {
    on_main(app, move |app| {
        let st = app.state::<Shared>();
        let mut core = st.lock().unwrap();
        f(app, &mut core)
    })
}

fn changed(app: &AppHandle) {
    let _ = app.emit_to("main", "changed", ());
}

pub(crate) fn rescan(core: &mut Core) {
    core.lib = wallpaper::scan_all(&core.settings);
}

/// Resolve a CLI/UI target: a library id, or a path/URL that gets added to the library first.
fn resolve(core: &mut Core, target: &str) -> Result<String, String> {
    if core.find(target).is_some() {
        return Ok(target.to_string());
    }
    let w = library::add(&wallpaper::library_dir(&core.settings), &core.lib, target)?;
    rescan(core);
    Ok(w.id)
}

fn run_command(app: &AppHandle, core: &mut Core, cmd: Command) -> Result<(), String> {
    log(format!("command {cmd:?}"));
    match cmd {
        Command::Set { target, display } => {
            let id = resolve(core, &target)?;
            for i in wallpaper::targets(core, display)? {
                wallpaper::apply(app, core, i, &id)?;
            }
            // A wallpaper the user picked gets a full interval before cycling moves on.
            core.changed_at = std::time::Instant::now();
        }
        Command::Close { display } => {
            for i in wallpaper::targets(core, display)? {
                wallpaper::close(app, core, i);
            }
        }
        Command::Pause => core.manual = Some(true),
        Command::Play => core.manual = Some(false),
        Command::Resume => core.manual = None,
        Command::Toggle => {
            core.manual = if core.manual == Some(true) {
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
            for i in wallpaper::targets(core, display)? {
                wallpaper::set_prop(app, core, i, &key, &Value::String(value.clone()))?;
            }
        }
        Command::Volume(v) => wallpaper::set_volume(app, core, v),
        Command::Next => wallpaper::next(app, core)?,
        Command::Import(zip) => {
            library::import_zip(
                &wallpaper::library_dir(&core.settings),
                std::path::Path::new(&zip),
                library::MAX_UNPACKED,
            )?;
            rescan(core);
        }
        Command::Ui => open_ui(app, &core.settings.theme, &core.settings.backdrop),
        Command::Quit => app.exit(0),
        Command::Status => wallpaper::probe_pages(app, core),
        // Network work runs on the async runtime; the main thread only starts it.
        Command::Preset(id) => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { presets::download(&a, &id).await });
        }
        Command::CheckUpdate => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { update::check(&a).await });
        }
        Command::InstallUpdate => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { update::install(&a).await });
        }
    }
    wallpaper::tick(app, core);
    wallpaper::write_status(core);
    changed(app);
    Ok(())
}

fn handle_args(app: &AppHandle, args: &[String]) {
    match cli::parse(args) {
        Ok(Some(cmd)) => {
            let r = with_core(app, move |app, core| run_command(app, core, cmd));
            if let Err(e) = r {
                log(format!("error: {e}"));
            }
        }
        // A plain launch (desktop or Start menu shortcut, or clicking Sarab while it runs) opens the
        // window. The login launch passes --autostart and stays in the tray.
        Ok(None) if !args.iter().any(|a| a == AUTOSTART_FLAG) => {
            let _ = with_core(app, |app, core| run_command(app, core, Command::Ui));
        }
        Ok(None) => {}
        Err(e) => log(format!("error: {e}")),
    }
}

const AUTOSTART_FLAG: &str = "--autostart";

fn theme_of(name: &str) -> Option<tauri::Theme> {
    match name {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    }
}

/// Acrylic needs Windows 10 1903+, Mica Windows 11. Anything else gets the solid base color.
fn effects_for(name: &str) -> Option<tauri::utils::config::WindowEffectsConfig> {
    use tauri::window::{Effect, EffectsBuilder};
    let build = windows_version::OsVersion::current().build;
    let effect = match name {
        "acrylic" if build >= 18362 => Effect::Acrylic,
        "mica" if build >= 22000 => Effect::Mica,
        _ => return None,
    };
    Some(EffectsBuilder::new().effect(effect).build())
}

/// Effective theme for the page. WebView2 keeps the color scheme per profile, which every Sarab
/// window shares, so the page is told explicitly instead of trusting prefers-color-scheme.
fn page_theme(app: &AppHandle, setting: &str) -> &'static str {
    match setting {
        "light" => "light",
        "dark" => "dark",
        _ => match app.get_webview_window("main").and_then(|w| w.theme().ok()) {
            Some(tauri::Theme::Dark) => "dark",
            _ => "light",
        },
    }
}

/// Callers pass the settings because they may already hold the core lock.
fn open_ui(app: &AppHandle, theme: &str, backdrop: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let mut b = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Sarab")
        .inner_size(1060.0, 720.0)
        .min_inner_size(760.0, 500.0)
        .theme(theme_of(theme))
        // Always transparent so the backdrop can change without reopening; the page paints a solid base when it is "solid".
        .transparent(true);
    if let Some(fx) = effects_for(backdrop) {
        b = b.effects(fx);
    }
    // Built on demand and destroyed on close, so it costs nothing while nobody looks at it.
    if let Err(e) = b.build() {
        log(format!("open ui: {e}"));
    }
}

// ---- commands for the settings window (capability grants them to "main" only) ----

#[tauri::command]
async fn state(app: AppHandle) -> Value {
    let auto = app.autolaunch().is_enabled().unwrap_or(false);
    with_core(&app, move |app, core| {
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "webview": tauri::webview_version().unwrap_or_default(),
            "theme": page_theme(app, &core.settings.theme),
            "translucent": effects_for(&core.settings.backdrop).is_some(),
            "settings": core.settings,
            "library": core.lib,
            "categories": library::CATEGORIES,
            "manual": core.manual,
            "autostart": auto,
            "update": update::to_json(app),
            "presets": presets::to_json(app, &core.lib),
            "displays": core.displays.iter().map(|d| json!({
                "key": d.mon.key, "wallpaper": d.wallpaper, "state": d.state, "reason": d.reason, "error": d.error,
                "x": d.mon.rect.left, "y": d.mon.rect.top,
                "width": d.mon.rect.right - d.mon.rect.left, "height": d.mon.rect.bottom - d.mon.rect.top,
            })).collect::<Vec<_>>(),
        })
    })
}

#[tauri::command]
async fn set_wallpaper(
    app: AppHandle,
    target: String,
    display: Option<usize>,
) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Set { target, display })
    })
}

#[tauri::command]
async fn add(app: AppHandle, target: String) -> Result<Value, String> {
    with_core(&app, move |app, core| {
        let id = resolve(core, target.trim())?;
        changed(app);
        Ok(json!(id))
    })
}

#[tauri::command]
async fn import(app: AppHandle, path: String) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Import(path))
    })
}

#[tauri::command]
async fn remove(app: AppHandle, id: String) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let r = wallpaper::remove(app, core, &id);
        changed(app);
        r
    })
}

#[tauri::command]
async fn close(app: AppHandle, display: Option<usize>) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Close { display })
    })
}

#[tauri::command]
async fn toggle_pause(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Toggle)
    })
}

pub const WEBSITE: &str = "https://github.com/MKAbuMattar/sarab";
pub const ISSUES: &str = "https://github.com/MKAbuMattar/sarab/issues";

/// Opens one of Sarab's folders in Explorer, or its website or issue tracker in the browser.
/// Takes a fixed name, never a path or URL, so the page cannot open anything else.
#[tauri::command]
async fn open(app: AppHandle, which: String) -> Result<(), String> {
    let target = match which.as_str() {
        "config" | "library" => {
            let lib = with_core(&app, |_, core| wallpaper::library_dir(&core.settings));
            let dir = if which == "config" {
                settings::config_dir()
            } else {
                lib
            };
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            dir.into_os_string()
        }
        "website" => WEBSITE.into(),
        "issues" => ISSUES.into(),
        _ => return Err(format!("unknown target {which}")),
    };
    // explorer.exe opens folders itself and hands URLs to the default browser.
    std::process::Command::new("explorer.exe")
        .arg(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn play_anyway(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Play)
    })
}

#[tauri::command]
async fn resume_auto(app: AppHandle) -> Result<(), String> {
    with_core(&app.clone(), move |app, core| {
        run_command(app, core, Command::Resume)
    })
}

#[tauri::command]
async fn props(app: AppHandle, display: usize) -> Value {
    with_core(&app, move |_, core| {
        Value::Object(wallpaper::props_for(core, display))
    })
}

#[tauri::command]
async fn set_prop(
    app: AppHandle,
    display: usize,
    key: String,
    value: Value,
) -> Result<Value, String> {
    with_core(&app, move |app, core| {
        if display >= core.displays.len() {
            return Err("no such display".into());
        }
        wallpaper::set_prop(app, core, display, &key, &value)
    })
}

#[tauri::command]
async fn save_settings(app: AppHandle, new: settings::Settings) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let lib_changed = new.library_dir != core.settings.library_dir;
        let fit_changed = new.scaling != core.settings.scaling;
        if let Some(w) = app.get_webview_window("main") {
            if new.theme != core.settings.theme {
                let _ = w.set_theme(theme_of(&new.theme));
            }
            if new.backdrop != core.settings.backdrop {
                let _ = w.set_effects(effects_for(&new.backdrop));
            }
        }
        let vol = new.volume;
        core.settings = new;
        settings::save(&wallpaper::cfg("settings.json"), &core.settings)
            .map_err(|e| e.to_string())?;
        if lib_changed {
            rescan(core);
        }
        // The fit is part of the player URL, so running videos and GIFs load again with it.
        if fit_changed {
            for i in 0..core.displays.len() {
                let id = core.displays[i].wallpaper.clone();
                let kind = id
                    .as_deref()
                    .and_then(|id| core.find(id))
                    .map(|w| w.info.r#type);
                if let (Some(id), Some(library::Kind::Video | library::Kind::Gif)) = (id, kind) {
                    let _ = wallpaper::apply(app, core, i, &id);
                }
            }
        }
        wallpaper::set_volume(app, core, vol);
        wallpaper::set_fps(app, core);
        wallpaper::tick(app, core);
        changed(app);
        Ok(())
    })
}

fn editable(core: &Core, id: &str) -> Result<library::Wallpaper, String> {
    let w = core.find(id).cloned().ok_or("not found")?;
    if w.preset {
        return Err("built-in wallpapers cannot be edited".into());
    }
    Ok(w)
}

#[tauri::command]
async fn edit_info(app: AppHandle, id: String, edit: library::Edit) -> Result<(), String> {
    with_core(&app, move |app, core| {
        let w = editable(core, &id)?;
        library::edit_info(&w.dir, edit)?;
        rescan(core);
        changed(app);
        Ok(())
    })
}

#[tauri::command]
async fn details(app: AppHandle, id: String) -> Result<library::Details, String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    Ok(library::details(&w))
}

/// Opens the wallpaper's own folder in Explorer. Takes an id, never a path.
#[tauri::command]
async fn reveal(app: AppHandle, id: String) -> Result<(), String> {
    let w = with_core(&app, move |_, core| core.find(&id).cloned()).ok_or("not found")?;
    std::process::Command::new("explorer.exe")
        .arg(&w.dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_preset(app: AppHandle, id: String) -> Result<(), String> {
    presets::download(&app, &id).await
}

#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    update::check(&app).await
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    update::install(&app).await
}

#[tauri::command]
async fn autostart(app: AppHandle, enable: bool) -> Result<(), String> {
    let al = app.autolaunch();
    if enable { al.enable() } else { al.disable() }.map_err(|e| e.to_string())
}

/// A UI string for text Rust shows itself (tray menu, notifications), from the same files as the window.
pub(crate) fn ui_text(lang: &str, key: &str) -> String {
    let file = if lang == "ar" {
        include_str!("../../ui/i18n/ar.json")
    } else {
        include_str!("../../ui/i18n/en.json")
    };
    serde_json::from_str::<Value>(file)
        .ok()
        .and_then(|v| v.get(key).and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| key.to_string())
}

fn tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let t = |k: &str| ui_text(lang, k);
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "toggle", t("tray.toggle"), true, None::<&str>)?,
            &MenuItem::with_id(app, "next", t("tray.next"), true, None::<&str>)?,
            &MenuItem::with_id(app, "ui", t("tray.open"), true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", t("tray.quit"), true, None::<&str>)?,
        ],
    )?;
    let mut b = TrayIconBuilder::with_id("sarab")
        .tooltip("Sarab")
        .menu(&menu)
        .show_menu_on_left_click(false);
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.on_menu_event(|app, ev| {
        let cmd = match ev.id().as_ref() {
            "toggle" => "toggle",
            "next" => "next",
            "ui" => "ui",
            _ => "quit",
        };
        handle_args(app, &[cmd.to_string()]);
    })
    .on_tray_icon_event(|tray, ev| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = ev
        {
            handle_args(tray.app_handle(), &["ui".to_string()]);
        }
    })
    .build(app)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let app = tauri::Builder::default()
        // Must be the first plugin: a second `sarab ...` process hands its args to us and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let rest: Vec<String> = argv.into_iter().skip(1).collect();
            let app = app.clone();
            // Not inline: this callback runs inside a window message, where building windows can re-enter.
            std::thread::spawn(move || handle_args(&app, &rest));
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage::<update::Updates>(Default::default())
        .manage::<presets::Downloads>(Default::default())
        .manage::<Shared>(Mutex::new(Core::load()))
        .invoke_handler(tauri::generate_handler![
            state,
            set_wallpaper,
            add,
            import,
            remove,
            close,
            toggle_pause,
            open,
            play_anyway,
            resume_auto,
            props,
            set_prop,
            save_settings,
            autostart,
            check_update,
            install_update,
            get_preset,
            edit_info,
            details,
            reveal
        ])
        .on_window_event(|win, ev| {
            if win.label() != "main" {
                return;
            }
            // "Use system setting" follows Windows switching between light and dark while open.
            if let WindowEvent::ThemeChanged(_) = ev {
                changed(win.app_handle());
            }
            if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = ev {
                let paths: Vec<String> = paths
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                let app = win.app_handle().clone();
                std::thread::spawn(move || {
                    with_core(&app, move |app, core| {
                        for p in paths {
                            let r = if p.to_lowercase().ends_with(".zip") {
                                library::import_zip(
                                    &wallpaper::library_dir(&core.settings),
                                    std::path::Path::new(&p),
                                    library::MAX_UNPACKED,
                                )
                                .map(|_| ())
                            } else {
                                resolve(core, &p).map(|_| ())
                            };
                            if let Err(e) = r {
                                log(format!("drop {p}: {e}"));
                            }
                        }
                        rescan(core);
                        changed(app);
                    })
                });
            }
        })
        .setup(move |app| {
            // reqwest is built without a default TLS provider (the updater's choice); install it once for the whole app.
            let _ = rustls::crypto::ring::default_provider().install_default();
            let h = app.handle().clone();
            if let Ok(res) = app.path().resource_dir() {
                let _ = wallpaper::PRESET_DIR.set(res.join("presets"));
            }
            let lang = {
                let st = h.state::<Shared>();
                let mut core = st.lock().unwrap();
                rescan(&mut core);
                core.desktop = os::windows::find_desktop();
                if core.desktop.is_none() {
                    log("desktop layer (WorkerW) not found; will retry");
                }
                os::windows::refresh_desktop(core.desktop.as_ref());
                wallpaper::sync_displays(&h, &mut core);
                core.settings.language.clone()
            };
            tray(&h, &lang)?;
            update::spawn(h.clone());
            let tick_app = h.clone();
            std::thread::spawn(move || loop {
                // ponytail: 1 s poll for every probe; move lock/power/session to OS notifications if wakeups show up in profiles.
                std::thread::sleep(std::time::Duration::from_millis(1000));
                let a = tick_app.clone();
                if tick_app
                    .run_on_main_thread(move || {
                        let st = a.state::<Shared>();
                        let mut core = st.lock().unwrap();
                        wallpaper::tick(&a, &mut core);
                    })
                    .is_err()
                {
                    break;
                }
            });
            // Start with Windows is on by default: turned on once, at the first normal launch.
            // After that the user's choice stands; a login launch never changes it.
            if !args.iter().any(|a| a == AUTOSTART_FLAG) {
                let st = h.state::<Shared>();
                let mut core = st.lock().unwrap();
                if !core.settings.autostart_set {
                    match h.autolaunch().enable() {
                        Ok(()) => log("start with Windows turned on (first launch)"),
                        Err(e) => log(format!("start with Windows: {e}")),
                    }
                    core.settings.autostart_set = true;
                    let _ = settings::save(&wallpaper::cfg("settings.json"), &core.settings);
                }
            }
            let first = args.clone();
            std::thread::spawn(move || handle_args(&h, &first));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Sarab");

    app.run(|app, ev| match ev {
        // Wallpaper windows come and go; the app lives in the tray until Quit.
        RunEvent::ExitRequested {
            api, code: None, ..
        } => api.prevent_exit(),
        RunEvent::Exit => {
            let st = app.state::<Shared>();
            let core = st.lock().unwrap();
            for w in app.webview_windows().values() {
                let _ = w.destroy();
            }
            os::windows::refresh_desktop(core.desktop.as_ref());
            log("exit");
        }
        _ => {}
    });
}
