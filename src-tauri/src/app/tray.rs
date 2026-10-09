use super::*;
use std::sync::Mutex;
#[cfg(not(windows))]
use tauri::menu::{Menu, MenuItem};

/// The update a tray item offers, once one is found.
static UPDATE: Mutex<Option<String>> = Mutex::new(None);
/// Where the tray icon was right-clicked, in physical pixels.
static CLICKED_AT: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));
const MENU_WINDOW: &str = "tray-menu";

/// The tray items: (id, label, Segoe Fluent glyph).
fn tray_items(lang: &str) -> Vec<(&'static str, String, &'static str)> {
    let t = |k: &str| ui_text(lang, k);
    let mut items = Vec::new();
    if let Some(v) = UPDATE.lock().unwrap().clone() {
        items.push((
            "install-update",
            t("update.trayItem").replace("{v}", &v),
            "\u{E896}",
        ));
    }
    items.extend([
        ("toggle", t("tray.toggle"), "\u{E769}"),
        ("next", t("tray.next"), "\u{E893}"),
        ("ui", t("tray.open"), "\u{E8A7}"),
        ("quit", t("tray.quit"), "\u{E7E8}"),
    ]);
    items
}

fn run_item(app: &AppHandle, id: &str) {
    let cmd = match id {
        "toggle" | "next" | "ui" | "install-update" | "quit" => id,
        _ => return,
    };
    handle_args(app, &[cmd.to_string()]);
}

/// Offers the update in the tray, from the update check.
pub(crate) fn tray_update(app: &AppHandle, lang: &str, version: &str) {
    *UPDATE.lock().unwrap() = Some(version.to_string());
    // Linux keeps the native menu: AppIndicator needs one.
    #[cfg(not(windows))]
    if let (Some(tray), Ok(menu)) = (app.tray_by_id("sarab"), native_menu(app, lang)) {
        let _ = tray.set_menu(Some(menu));
    }
    #[cfg(windows)]
    let _ = (app, lang);
}

#[cfg(not(windows))]
fn native_menu(app: &AppHandle, lang: &str) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::new(app)?;
    for (id, label, _) in tray_items(lang) {
        menu.append(&MenuItem::with_id(app, id, label, true, None::<&str>)?)?;
    }
    Ok(menu)
}

pub(crate) fn tray(app: &AppHandle, lang: &str) -> tauri::Result<()> {
    let mut b = TrayIconBuilder::with_id("sarab")
        .tooltip("Sarab")
        .show_menu_on_left_click(false);
    #[cfg(not(windows))]
    {
        b = b.menu(&native_menu(app, lang)?);
    }
    #[cfg(windows)]
    let _ = lang;
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.on_menu_event(|app, ev| run_item(app, ev.id().as_ref()))
        .on_tray_icon_event(|tray, ev| {
            let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = ev
            else {
                return;
            };
            match button {
                MouseButton::Left => handle_args(tray.app_handle(), &["ui".to_string()]),
                MouseButton::Right if cfg!(windows) => {
                    open_tray_menu(tray.app_handle(), (position.x, position.y))
                }
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

/// Sarab's own tray menu: a small window at the cursor, closed when it loses focus.
fn open_tray_menu(app: &AppHandle, at: (f64, f64)) {
    *CLICKED_AT.lock().unwrap() = at;
    if let Some(w) = app.get_webview_window(MENU_WINDOW) {
        let _ = w.emit_to(MENU_WINDOW, "tray-menu-open", ());
        return;
    }
    let built =
        WebviewWindowBuilder::new(app, MENU_WINDOW, WebviewUrl::App("tray-menu.html".into()))
            .title("Sarab")
            .inner_size(240.0, 180.0)
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            // Not tao's shadow: it sizes the window as if it still had a title bar.
            .shadow(false)
            .visible(false)
            .build();
    match built {
        Ok(w) => {
            if let Some(h) = os::platform::handle(&w) {
                os::platform::round_corners(h);
            }
            let a = app.clone();
            w.on_window_event(move |e| {
                if let tauri::WindowEvent::Focused(false) = e {
                    hide_tray_menu(&a);
                }
            });
        }
        Err(e) => log(format!("tray menu: {e}")),
    }
}

fn hide_tray_menu(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MENU_WINDOW) {
        let _ = w.hide();
    }
}

#[tauri::command]
pub(crate) async fn tray_menu_items(app: AppHandle) -> Value {
    with_core(&app, |app, core| {
        let lang = core.settings.language.clone();
        json!({
            "theme": page_theme(app, &core.settings.theme),
            "dir": if lang == "ar" { "rtl" } else { "ltr" },
            "items": tray_items(&lang)
                .into_iter()
                .map(|(id, label, glyph)| json!({ "id": id, "label": label, "glyph": glyph }))
                .collect::<Vec<_>>(),
        })
    })
}

/// Sizes the menu to its content (CSS pixels) and shows it beside the clicked point,
/// inside the work area of that display.
#[tauri::command]
pub(crate) async fn tray_menu_show(app: AppHandle, width: f64, height: f64) {
    on_main(&app, move |app| {
        let Some(w) = app.get_webview_window(MENU_WINDOW) else {
            return;
        };
        let (x, y) = *CLICKED_AT.lock().unwrap();
        let Ok(Some(mon)) = app.monitor_from_point(x, y) else {
            return;
        };
        let s = mon.scale_factor();
        let (pw, ph) = ((width * s).ceil(), (height * s).ceil());
        let work = mon.work_area();
        let (left, top) = (work.position.x as f64, work.position.y as f64);
        let (right, bottom) = (left + work.size.width as f64, top + work.size.height as f64);
        let px = if x + pw > right { x - pw } else { x };
        let py = if y - ph >= top { y - ph } else { y };
        let px = px.clamp(left, (right - pw).max(left));
        let py = py.clamp(top, (bottom - ph).max(top));
        let _ = w.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32));
        let _ = w.set_position(tauri::PhysicalPosition::new(px as i32, py as i32));
        let _ = w.show();
        let _ = w.set_focus();
    })
}

#[tauri::command]
pub(crate) async fn tray_menu_pick(app: AppHandle, id: String) {
    on_main(&app, hide_tray_menu);
    run_item(&app, &id);
}
