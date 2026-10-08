use crate::engine::wallpaper::{log, Shared};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Updater, UpdaterExt};

pub fn feed(channel: &str) -> &'static str {
    match channel {
        "beta" => "https://github.com/MKAbuMattar/sarab/releases/download/beta/latest.json",
        _ => "https://github.com/MKAbuMattar/sarab/releases/latest/download/latest.json",
    }
}

fn updater(app: &AppHandle) -> Result<Updater, String> {
    let channel = app
        .state::<Shared>()
        .lock()
        .unwrap()
        .settings
        .update_channel
        .clone();
    let url = feed(&channel).parse().map_err(|e| format!("{e}"))?;
    app.updater_builder()
        .endpoints(vec![url])
        .and_then(|b| b.build())
        .map_err(|e| e.to_string())
}

#[derive(Default)]
pub struct State {
    available: Option<(String, String)>,
    progress: Option<u8>,
    error: Option<String>,
    checked: Option<u64>,
    notified: Option<String>,
}

pub type Updates = Mutex<State>;

const FIRST_CHECK: Duration = Duration::from_secs(60);
const INTERVAL: Duration = Duration::from_secs(3 * 60 * 60);
const FRESH: u64 = 60 * 60;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn changed(app: &AppHandle) {
    let _ = app.emit_to("main", "changed", ());
}

pub fn to_json(app: &AppHandle) -> Value {
    let st = app.state::<Updates>();
    let s = st.lock().unwrap();
    json!({
        "available": s.available.as_ref().map(|(v, n)| json!({ "version": v, "notes": n })),
        "progress": s.progress,
        "error": s.error,
        "checked": s.checked,
    })
}

fn allowed(app: &AppHandle) -> bool {
    app.state::<Shared>().lock().unwrap().settings.check_updates
}

pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if allowed(&app) {
                let _ = tauri::async_runtime::block_on(check(&app));
            }
            std::thread::sleep(INTERVAL);
        }
    });
}

pub async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let result = match updater(app) {
        Ok(u) => u.check().await.map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    let found = {
        let st = app.state::<Updates>();
        let mut s = st.lock().unwrap();
        s.checked = Some(now());
        match &result {
            Ok(Some(u)) => {
                s.available = Some((u.version.clone(), u.body.clone().unwrap_or_default()));
                s.error = None;
            }
            Ok(None) => {
                s.available = None;
                s.error = None;
            }
            Err(e) => s.error = Some(e.clone()),
        }
        match &result {
            Ok(Some(u)) if s.notified.as_deref() != Some(u.version.as_str()) => {
                s.notified = Some(u.version.clone());
                Some(u.version.clone())
            }
            _ => None,
        }
    };
    if let Some(v) = &found {
        notify(app, v);
    }
    if let Err(e) = &result {
        log(format!("update check: {e}"));
    }
    changed(app);
    result.map(|o| o.map(|u| u.version))
}

fn notify(app: &AppHandle, version: &str) {
    let lang = app
        .state::<Shared>()
        .lock()
        .unwrap()
        .settings
        .language
        .clone();
    let t = |k: &str| crate::ui_text(&lang, k).replace("{v}", version);
    log(format!("update available: {version}"));
    if let Some(tray) = app.tray_by_id("sarab") {
        let _ = tray.set_tooltip(Some(t("update.tooltip")));
        if let Ok(menu) = crate::tray_menu(app, &lang, Some(version)) {
            let _ = tray.set_menu(Some(menu));
        }
    }
    let xml = crate::os::platform::update_toast_xml(
        &t("update.toastTitle"),
        &t("update.toastBody"),
        &t("update.now"),
        &t("update.later"),
    );
    let a = app.clone();
    let shown =
        crate::os::platform::show_toast(crate::core::settings::APP_ID, &xml, move |answer| {
            log(format!("update toast: {answer:?}"));
            match answer.as_str() {
                "install" => crate::handle_args(&a, &["install-update".to_string()]),
                "later" => {}
                _ => crate::handle_args(&a, &["ui".to_string()]),
            }
        });
    if let Err(e) = shown {
        log(format!("update notification: {e}"));
    }
}

pub fn check_if_stale(app: &AppHandle, allowed: bool) {
    let last = app.state::<Updates>().lock().unwrap().checked;
    if !allowed || last.is_some_and(|t| now().saturating_sub(t) < FRESH) {
        return;
    }
    let a = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = check(&a).await;
    });
}

pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = updater(app)?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("no update is available")?;
    log(format!("installing update {}", update.version));
    let mut done: u64 = 0;
    let a = app.clone();
    let result = update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let pct = total
                    .filter(|t| *t > 0)
                    .map(|t| (done * 100 / t).min(100) as u8);
                let st = a.state::<Updates>();
                let mut s = st.lock().unwrap();
                if pct != s.progress {
                    s.progress = pct;
                    drop(s);
                    changed(&a);
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string());
    if let Err(e) = &result {
        let st = app.state::<Updates>();
        let mut s = st.lock().unwrap();
        s.progress = None;
        s.error = Some(e.clone());
        log(format!("update install: {e}"));
    }
    changed(app);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_feed() {
        assert!(feed("stable").ends_with("/releases/latest/download/latest.json"));
        assert!(feed("beta").ends_with("/releases/download/beta/latest.json"));
        assert_eq!(
            feed("anything else"),
            feed("stable"),
            "an unknown channel falls back to stable"
        );
        let wf = include_str!("../../../.github/workflows/release.yml");
        assert!(wf.contains("gh release upload beta latest.json"));
    }
}
