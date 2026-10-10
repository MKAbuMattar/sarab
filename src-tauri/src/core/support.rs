use crate::core::settings::Settings;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const LOG_FILES: [&str; 5] = [
    "sarab.log",
    "settings.json",
    "layout.json",
    "status.json",
    "restore.json",
];

pub fn downloads() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|p| PathBuf::from(p).join("Downloads"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

pub fn export_logs(config: &Path, dest: &Path) -> io::Result<PathBuf> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let path = dest.join(format!("sarab-logs-{secs}.zip"));
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path)?);
    let opts = zip::write::SimpleFileOptions::default();
    for name in LOG_FILES {
        if let Ok(bytes) = std::fs::read(config.join(name)) {
            zip.start_file(name, opts).map_err(io::Error::other)?;
            zip.write_all(&bytes)?;
        }
    }
    zip.finish().map_err(io::Error::other)?;
    Ok(path)
}

pub fn reset(old: &Settings) -> Settings {
    Settings {
        library_dir: old.library_dir.clone(),
        autostart_set: old.autostart_set,
        ..Settings::default()
    }
}

pub const SETTINGS_FILE: &str = "sarab-settings.json";

pub fn export_settings(s: &Settings, dest: &Path) -> io::Result<PathBuf> {
    let path = dest.join(SETTINGS_FILE);
    crate::core::settings::save(&path, s)?;
    Ok(path)
}

pub fn import_settings(json: &str, old: &Settings) -> Result<Settings, String> {
    let bad = |e: String| format!("not a Sarab settings file: {e}");
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| bad(e.to_string()))?;
    let known = serde_json::to_value(Settings::default()).map_err(|e| bad(e.to_string()))?;
    let ours = v
        .as_object()
        .is_some_and(|o| o.keys().any(|k| known.get(k).is_some()));
    if !ours {
        return Err(bad("no Sarab settings in it".into()));
    }
    let mut s: Settings = serde_json::from_value(v).map_err(|e| bad(e.to_string()))?;
    if s.library_dir.as_ref().is_some_and(|d| !d.is_dir()) {
        s.library_dir = old.library_dir.clone();
    }
    s.autostart_set = old.autostart_set;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_logs_zips_the_known_files() {
        let d = std::env::temp_dir().join(format!("sarab-test-logs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("sarab.log"), "1 started\n").unwrap();
        std::fs::write(d.join("settings.json"), "{}").unwrap();
        std::fs::write(d.join("private.txt"), "not for a bug report").unwrap();
        let zip_path = export_logs(&d, &d).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()).unwrap();
        let mut names: Vec<String> = (0..z.len())
            .map(|i| z.by_index(i).unwrap().name().to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["sarab.log", "settings.json"]);
        let mut log = String::new();
        io::Read::read_to_string(&mut z.by_name("sarab.log").unwrap(), &mut log).unwrap();
        assert_eq!(log, "1 started\n");
    }

    #[test]
    fn reset_keeps_library_and_autostart_record() {
        let old = Settings {
            library_dir: Some("D:/Wallpapers".into()),
            autostart_set: true,
            fps: 60,
            volume: 70,
            language: "ar".into(),
            ..Settings::default()
        };
        let new = reset(&old);
        assert_eq!(new.library_dir, old.library_dir);
        assert!(new.autostart_set);
        assert_eq!(new.fps, Settings::default().fps);
        assert_eq!(new.volume, 0);
        assert_eq!(new.language, "en");
    }

    #[test]
    fn settings_round_trip_through_a_file() {
        let d = std::env::temp_dir().join(format!("sarab-test-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let mine = Settings {
            fps: 60,
            pause_cpu: 80,
            app_pause: vec!["blender.exe".into()],
            language: "ar".into(),
            library_dir: Some(d.clone()),
            ..Settings::default()
        };
        let file = export_settings(&mine, &d).unwrap();
        assert_eq!(file.file_name().unwrap(), SETTINGS_FILE);
        let here = Settings {
            autostart_set: true,
            library_dir: Some("D:/Mine".into()),
            ..Settings::default()
        };
        let got = import_settings(&std::fs::read_to_string(&file).unwrap(), &here).unwrap();
        assert_eq!((got.fps, got.pause_cpu, got.language.as_str()), (60, 80, "ar"));
        assert_eq!(got.app_pause, ["blender.exe"]);
        assert_eq!(got.library_dir, Some(d.clone()), "an existing folder is kept");
        assert!(got.autostart_set, "this PC's autostart record is kept");
        let moved = import_settings(r#"{"fps":15,"library_dir":"Z:/nowhere/at/all"}"#, &here).unwrap();
        assert_eq!(moved.fps, 15);
        assert_eq!(moved.library_dir, here.library_dir, "a missing folder falls back");
        assert!(import_settings("not json", &here).is_err());
        assert!(import_settings("[1,2]", &here).is_err());
        assert!(import_settings(r#"{"name":"a package"}"#, &here).is_err());
        assert!(import_settings(r#"{"fps":"fast"}"#, &here).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
