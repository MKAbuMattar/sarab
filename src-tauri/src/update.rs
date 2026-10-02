//! Update checks. A check runs a minute after start and then once a day, only while the user
//! allows it in Settings. Nothing is downloaded until the user chooses "Update now"; the updater
//! plugin then verifies the installer's signature against the public key in tauri.conf.json.

use crate::wallpaper::{log, Shared};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::UpdaterExt;

#[derive(Default)]
pub struct State {
    /// A newer release: version and its release notes.
    available: Option<(String, String)>,
    /// Download progress, 0 to 100, while installing.
    progress: Option<u8>,
    error: Option<String>,
    /// Unix seconds of the last finished check.
    checked: Option<u64>,
    /// The version the user was already notified about, so each release notifies once.
    notified: Option<String>,
}

pub type Updates = Mutex<State>;

const FIRST_CHECK: Duration = Duration::from_secs(60);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn changed(app: &AppHandle) {
    let _ = app.emit_to("main", "changed", ());
}

/// What the settings window shows.
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

/// Background checks for the life of the app.
pub fn spawn(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if allowed(&app) {
                let _ = tauri::async_runtime::block_on(check(&app));
            }
            // ponytail: plain sleep, so a PC that sleeps overnight checks a day after waking; a wall-clock schedule if that matters.
            std::thread::sleep(INTERVAL);
        }
    });
}

/// Ask the release feed whether a newer version exists. Returns the new version, if any.
pub async fn check(app: &AppHandle) -> Result<Option<String>, String> {
    let result = match app.updater() {
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
    }
    let shown = app
        .notification()
        .builder()
        .title("Sarab")
        .body(t("update.toast"))
        .show();
    if let Err(e) = shown {
        log(format!("update notification: {e}"));
    }
}

/// Download, verify and run the installer. On Windows the app exits once the installer starts;
/// the installer reopens Sarab when it finishes.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
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
