use super::*;

pub(crate) fn with_core<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle, &mut Core) -> T + Send + 'static,
) -> T {
    on_main(app, move |app| {
        let st = app.state::<Shared>();
        let mut core = st.lock().unwrap();
        f(app, &mut core)
    })
}

pub(crate) fn changed(app: &AppHandle) {
    let _ = app.emit_to("main", "changed", ());
}

pub(crate) fn rescan(core: &mut Core) {
    core.lib = wallpaper::scan_all(&core.settings);
    wallpaper::make_thumbnails(&core.lib);
}

pub(crate) fn resolve(core: &mut Core, target: &str) -> Result<String, String> {
    if core.find(target).is_some() {
        return Ok(target.to_string());
    }
    let w = library::add(&wallpaper::library_dir(&core.settings), &core.lib, target)?;
    rescan(core);
    Ok(w.id)
}

pub(crate) fn run_command(app: &AppHandle, core: &mut Core, cmd: Command) -> Result<(), String> {
    log(format!("command {cmd:?}"));
    match cmd {
        Command::Set { target, display } if library::needs_convert(&target) => {
            let lib = wallpaper::library_dir(&core.settings);
            let a = app.clone();
            log(format!("converting {target} to MP4"));
            std::thread::spawn(move || {
                match library::convert(&lib, std::path::Path::new(&target)) {
                    Ok(mp4) => wallpaper::later(&a, move |app, core| {
                        let target = mp4.to_string_lossy().into_owned();
                        if let Err(e) = run_command(app, core, Command::Set { target, display }) {
                            log(format!("set converted video: {e}"));
                        }
                        changed(app);
                    }),
                    Err(e) => log(format!("convert {target}: {e}")),
                }
            });
        }
        Command::Set { target, display } => {
            let id = resolve(core, &target)?;
            for i in wallpaper::targets(core, display)? {
                wallpaper::apply(app, core, i, &id)?;
            }
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
        Command::Ui => open_ui(app, &core.settings),
        Command::Quit if core.settings.keep_frame_on_quit => {
            wallpaper::keep_frames_then_exit(app, core)
        }
        Command::Quit => app.exit(0),
        Command::Status => wallpaper::probe_pages(app, core),
        Command::Preset(id) => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { presets::download(&a, &id).await });
        }
        Command::CheckUpdate => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { update::check(&a).await });
        }
        Command::Screenshot { path, display } => {
            let path = std::path::PathBuf::from(path);
            if !path.is_absolute() {
                return Err("give screenshot a full path, for example C:/shots/desktop.png".into());
            }
            wallpaper::screenshot(app, core, display.unwrap_or(0), path)?;
        }
        Command::SettingsExport(path) => {
            let path = std::path::PathBuf::from(path);
            if !path.is_absolute() {
                return Err(
                    "give settings export a full path, for example C:/backup/sarab-settings.json"
                        .into(),
                );
            }
            crate::core::settings::save(&path, &core.settings).map_err(|e| e.to_string())?;
        }
        Command::SettingsImport(path) => {
            let json = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
            let new = support::import_settings(&json, &core.settings)?;
            apply_settings(app, core, new)?;
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

pub(crate) fn handle_args(app: &AppHandle, args: &[String]) {
    match cli::parse(args) {
        Ok(Some(cmd)) => {
            let r = with_core(app, move |app, core| run_command(app, core, cmd));
            if let Err(e) = r {
                log(format!("error: {e}"));
            }
        }
        Ok(None) if !args.iter().any(|a| a == AUTOSTART_FLAG) => {
            let _ = with_core(app, |app, core| run_command(app, core, Command::Ui));
        }
        Ok(None) => {}
        Err(e) => log(format!("error: {e}")),
    }
}
