//! Built-in wallpapers. Web scenes ship inside the installer (read-only resources). The 4K videos
//! are public-domain NASA footage, too large to ship, so Sarab downloads one only when the user
//! asks and keeps it only if its size and SHA-256 match the values pinned here.

use crate::engine::Handle;
use crate::engine::wallpaper::{library_dir, log};
use crate::model::library::{self, Kind, Manifest};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub title: String,
    pub description: String,
    pub credit: String,
    pub source: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(default)]
    pub clip: Option<[f64; 2]>,
}

pub fn catalog() -> Vec<Video> {
    serde_json::from_str(include_str!("presets.json")).expect("presets.json is valid")
}

/// Download a preset on its own thread. Progress and the result go back to the engine.
pub fn spawn(h: Handle, id: String) {
    let Some(v) = catalog().into_iter().find(|v| v.id == id) else {
        log(format!("no preset {id}"));
        return;
    };
    h.run(move |e| {
        let dir = library_dir(&e.core.settings).join(&v.id);
        if library::read(&dir).is_some() || e.downloads.contains_key(&v.id) {
            return;
        }
        e.downloads.insert(v.id.clone(), 0);
        e.changed();
        let h = e.handle.clone();
        std::thread::spawn(move || {
            let result = fetch(&h, &v, &dir);
            match &result {
                Ok(()) => log(format!("preset {} installed", v.id)),
                Err(err) => {
                    log(format!("preset {}: {err}", v.id));
                    let _ = std::fs::remove_dir_all(&dir);
                }
            }
            h.run(move |e| {
                e.downloads.remove(&v.id);
                e.rescan();
                e.changed();
            });
        });
    });
}

fn fetch(h: &Handle, v: &Video, dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let file = format!("{}.mp4", v.id);
    let part = dir.join(format!("{file}.part"));
    let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut resp = reqwest::blocking::get(&v.url).map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let mut hash = Sha256::new();
    let mut got: u64 = 0;
    let mut shown: u8 = 0;
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        got += n as u64;
        // The pinned size is the most a server may send; anything more is not the expected file.
        if got > v.size {
            return Err("the server sent more data than the pinned file size".into());
        }
        hash.update(chunk);
        out.write_all(chunk).map_err(|e| e.to_string())?;
        let pct = (got * 100 / v.size.max(1)) as u8;
        if pct != shown {
            shown = pct;
            let id = v.id.clone();
            h.run(move |e| {
                e.downloads.insert(id, pct);
                e.changed();
            });
        }
    }
    drop(out);
    let digest = hex(&hash.finalize());
    verify(v, got, &digest)?;
    std::fs::rename(&part, dir.join(&file)).map_err(|e| e.to_string())?;
    let info = Manifest {
        title: Some(v.title.clone()),
        description: Some(v.description.clone()),
        author: Some(v.credit.clone()),
        license: Some("Public domain (NASA)".into()),
        r#type: Kind::Video,
        file: Some(file),
        tags: vec!["preset".into(), "4K".into()],
        version: 1,
        clip: v.clip,
        ..Default::default()
    };
    crate::model::settings::save(&dir.join(library::INFO), &info).map_err(|e| e.to_string())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A download is kept only when both its size and its SHA-256 match the pinned values.
fn verify(v: &Video, got: u64, digest: &str) -> Result<(), String> {
    if got != v.size || !digest.eq_ignore_ascii_case(&v.sha256) {
        return Err(format!(
            "the file does not match its pinned checksum ({got} bytes, sha256 {digest})"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_answer() {
        // FIPS 180-2 test vector, so a sha2 upgrade that changes output fails here, not in a user's download.
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let v = catalog().remove(0);
        assert!(verify(&v, v.size, &v.sha256).is_ok());
        assert!(verify(&v, v.size - 1, &v.sha256).is_err(), "short file");
        let mut wrong = v.sha256.clone();
        wrong.replace_range(0..1, if wrong.starts_with('0') { "1" } else { "0" });
        assert!(verify(&v, v.size, &wrong).is_err(), "one changed hex digit");
    }

    #[test]
    fn catalog_is_pinned_4k() {
        let c = catalog();
        assert!(!c.is_empty());
        let mut ids = std::collections::BTreeSet::new();
        for v in &c {
            assert!(ids.insert(v.id.clone()), "duplicate id {}", v.id);
            assert!(v.url.starts_with("https://"), "{} is not https", v.id);
            assert_eq!(v.sha256.len(), 64, "{} sha256", v.id);
            assert!(
                v.sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
            );
            assert!(v.width >= 3840 && v.height >= 2160, "{} is not 4K", v.id);
            assert!(v.size > 0);
            if let Some([a, b]) = v.clip {
                assert!(0.0 <= a && a < b, "{} clip", v.id);
            }
        }
    }
}
