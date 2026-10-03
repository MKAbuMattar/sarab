//! Update checks. A check runs a minute after start and then once a day, only while the user
//! allows it in Settings. Nothing is downloaded until the user chooses "Update now"; the
//! installer is kept only if its minisign signature matches the public key pinned below.

use crate::engine::wallpaper::log;
use crate::engine::{Engine, Handle, ui_text};
use base64::Engine as _;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Read;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const FIRST_CHECK: Duration = Duration::from_secs(60);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const PLATFORM: &str = "windows-x86_64";
/// The installed shortcut's AppUserModelID, so the toast carries Sarab's name and icon.
const APP_ID: &str = "com.mkabumattar.sarab";

#[derive(Deserialize, Clone)]
struct Asset {
    signature: String,
    url: String,
}

#[derive(Deserialize, Clone)]
struct Release {
    version: String,
    #[serde(default)]
    notes: String,
    platforms: BTreeMap<String, Asset>,
}

#[derive(Default)]
pub struct State {
    available: Option<Release>,
    /// Download progress, 0 to 100, while installing.
    progress: Option<u8>,
    error: Option<String>,
    /// Unix seconds of the last finished check.
    checked: Option<u64>,
    /// The version the user was already notified about, so each release notifies once.
    notified: Option<String>,
}

/// What the settings window shows.
#[derive(Clone, Default)]
pub struct View {
    /// Version and release notes.
    pub available: Option<(String, String)>,
    pub progress: Option<u8>,
    pub error: Option<String>,
    pub checked: Option<u64>,
}

impl State {
    pub fn view(&self) -> View {
        View {
            available: self
                .available
                .as_ref()
                .map(|r| (r.version.clone(), r.notes.clone())),
            progress: self.progress,
            error: self.error.clone(),
            checked: self.checked,
        }
    }
}

/// minisign public key (base64 of the .pub file) that signs every release installer.
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDdEN0JCQTcyRTY3RDE4OTgKUldTWUdIM21jcnA3ZldzTCtIUUQvbWc1ZUczbEpjQ2R1WTEvZCszU0plV2tQWDk1bE51YXB2eFEK";
const ENDPOINT: &str = "https://github.com/MKAbuMattar/sarab/releases/latest/download/latest.json";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `1.2.10` > `1.2.9`; a leading `v` is ignored.
fn newer(candidate: &str, current: &str) -> bool {
    let parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split(['.', '-', '+'])
            .take(3)
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parts(candidate) > parts(current)
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("sarab/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())
}

fn fetch_release() -> Result<Option<Release>, String> {
    let r: Release = client()?
        .get(ENDPOINT)
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| e.to_string())?;
    Ok(newer(&r.version, env!("CARGO_PKG_VERSION")).then_some(r))
}

/// Background checks for the life of the app.
pub fn spawn_daily(h: Handle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            let allowed = futures::executor::block_on(h.ask(|e| e.core.settings.check_updates));
            if allowed.unwrap_or(false) {
                check(&h);
            }
            // ponytail: plain sleep, so a PC that sleeps overnight checks a day after waking; a wall-clock schedule if that matters.
            std::thread::sleep(INTERVAL);
        }
    });
}

pub fn spawn_check(h: Handle) {
    std::thread::spawn(move || check(&h));
}

fn check(h: &Handle) {
    let result = fetch_release();
    h.run(move |e| {
        let s = &mut e.updates;
        s.checked = Some(now());
        match result {
            Ok(r) => {
                s.error = None;
                let fresh = r
                    .as_ref()
                    .filter(|r| s.notified.as_deref() != Some(r.version.as_str()));
                if let Some(r) = fresh {
                    s.notified = Some(r.version.clone());
                    notify(e, &r.version.clone());
                }
                e.updates.available = r;
            }
            Err(err) => {
                log(format!("update check: {err}"));
                s.error = Some(err);
            }
        }
        e.changed();
    });
}

fn notify(e: &Engine, version: &str) {
    let lang = &e.core.settings.language;
    let t = |k: &str| ui_text(lang, k).replace("{v}", version);
    log(format!("update available: {version}"));
    e.set_tooltip(&t("update.tooltip"));
    let shown = crate::platform::toast::show(APP_ID, "Sarab", &t("update.toast"));
    if let Err(err) = shown {
        log(format!("update notification: {err}"));
    }
}

/// Download, verify and run the installer, then quit. The installer reopens Sarab when it finishes.
pub fn spawn_install(h: Handle) {
    h.clone().run(move |e| {
        let Some(r) = e.updates.available.clone() else {
            e.updates.error = Some("no update is available".into());
            e.changed();
            return;
        };
        if e.updates.progress.is_some() {
            return;
        }
        e.updates.progress = Some(0);
        e.changed();
        std::thread::spawn(move || {
            let result = install(&h, &r);
            h.run(move |e| match result {
                Ok(path) => {
                    log(format!("installing update {}", r.version));
                    // NSIS switches: passive, update mode (keeps Start with Windows), restart when done.
                    match std::process::Command::new(&path)
                        .args(["/P", "/UPDATE", "/R"])
                        .spawn()
                    {
                        Ok(_) => e
                            .run_command(crate::model::cli::Command::Quit)
                            .unwrap_or_default(),
                        Err(err) => fail(e, err.to_string()),
                    }
                }
                Err(err) => fail(e, err),
            });
        });
    });
}

fn fail(e: &mut Engine, err: String) {
    log(format!("update install: {err}"));
    e.updates.progress = None;
    e.updates.error = Some(err);
    e.changed();
}

fn install(h: &Handle, r: &Release) -> Result<std::path::PathBuf, String> {
    let asset = r
        .platforms
        .get(PLATFORM)
        .ok_or("the release has no Windows installer")?;
    let mut resp = client()?
        .get(&asset.url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let total = resp.content_length().filter(|t| *t > 0);
    let mut bytes = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut buf = vec![0u8; 256 * 1024];
    let mut shown = 0u8;
    loop {
        let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        if let Some(t) = total {
            let pct = (bytes.len() as u64 * 100 / t).min(100) as u8;
            if pct != shown {
                shown = pct;
                h.run(move |e| {
                    e.updates.progress = Some(pct);
                    e.changed();
                });
            }
        }
    }
    verify(&bytes, &asset.signature, PUBKEY, &r.version)?;
    let path = std::env::temp_dir().join(format!("Sarab_{}_x64-setup.exe", r.version));
    std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

fn b64_text(s: &str) -> Result<String, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| e.to_string())?;
    String::from_utf8(raw).map_err(|e| e.to_string())
}

/// The release feed is not signed, so its version alone proves nothing; the signed trusted
/// comment must name the same version, or an old signed installer could pose as a new one.
fn verify(data: &[u8], signature: &str, pubkey: &str, version: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&b64_text(pubkey)?).map_err(|e| e.to_string())?;
    let sig =
        minisign_verify::Signature::decode(&b64_text(signature)?).map_err(|e| e.to_string())?;
    key.verify(data, &sig, true)
        .map_err(|e| format!("the installer signature is not valid: {e}"))?;
    let signed = sig
        .trusted_comment()
        .split('\t')
        .find_map(|f| f.strip_prefix("version:"));
    match signed {
        Some(v) if v.trim_start_matches('v') == version.trim_start_matches('v') => Ok(()),
        Some(v) => Err(format!(
            "the installer was signed for version {v}, not {version}"
        )),
        None => Err("the installer signature does not name a version".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_order() {
        assert!(newer("0.0.5", "0.0.4"));
        assert!(newer("v0.1.0", "0.0.9"));
        assert!(newer("1.2.10", "1.2.9"));
        assert!(!newer("0.0.4", "0.0.4"));
        assert!(!newer("0.0.3", "0.0.4"));
    }

    #[test]
    fn pinned_key_decodes() {
        assert!(minisign_verify::PublicKey::decode(&b64_text(PUBKEY).unwrap()).is_ok());
        assert!(ENDPOINT.starts_with("https://github.com/MKAbuMattar/sarab/"));
    }

    #[test]
    fn bad_signature_is_rejected() {
        assert!(verify(b"not the installer", "", PUBKEY, "0.0.4").is_err());
    }
}
