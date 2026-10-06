#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod cli;
mod common;
mod core;
mod engine;
mod library;
mod os;

use crate::core::{settings, update};
use crate::engine::wallpaper;
use crate::library::presets;
use std::sync::Mutex;
use tauri::{DragDropEvent, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use wallpaper::{log, Core, Shared};

use app::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Started as sarab.com, the console twin: answer in the terminal and never start the app.
    let exe = std::env::current_exe().unwrap_or_default();
    if exe
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("com"))
    {
        std::process::exit(cli::terminal::run(&args));
    }
    // The installer's calls: done here, before the window or the hand-off to a running Sarab.
    if let Some(add) = match args.first().map(String::as_str) {
        Some("--add-to-path") => Some(true),
        Some("--remove-from-path") => Some(false),
        _ => None,
    } {
        if let Err(e) = os::windows::set_on_path(add) {
            log(format!("PATH: {e}"));
        }
        return;
    }
    // A mistyped command is answered in the terminal, instead of only in the log of the running copy.
    if let Err(e) = cli::parse(&args) {
        os::windows::tell_terminal(&format!("sarab: {e}"));
        std::process::exit(2);
    }
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
            reset_props,
            save_settings,
            reset_settings,
            export_logs,
            autostart,
            check_update,
            install_update,
            get_preset,
            edit_info,
            details,
            reveal,
            export_wallpaper,
            move_library
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
            let _ = wallpaper::APP.set(h.clone());
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
