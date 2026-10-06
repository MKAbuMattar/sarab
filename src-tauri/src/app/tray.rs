use super::*;

pub(crate) fn tray_menu(
    app: &AppHandle,
    lang: &str,
    update: Option<&str>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let t = |k: &str| ui_text(lang, k);
    let menu = Menu::new(app)?;
    if let Some(v) = update {
        menu.append(&MenuItem::with_id(
            app,
            "install-update",
            t("update.trayItem").replace("{v}", v),
            true,
            None::<&str>,
        )?)?;
    }
    for (id, key) in [
        ("toggle", "tray.toggle"),
        ("next", "tray.next"),
        ("ui", "tray.open"),
        ("quit", "tray.quit"),
    ] {
        menu.append(&MenuItem::with_id(app, id, t(key), true, None::<&str>)?)?;
    }
    Ok(menu)
}

pub(crate) fn tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let menu = tray_menu(app, lang, None)?;
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
            "install-update" => "install-update",
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
