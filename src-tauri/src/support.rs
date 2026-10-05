//! Help when something goes wrong: logs for a bug report, and settings back to their defaults.

use crate::settings::Settings;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Files in the config folder that explain a problem. Wallpapers and their settings stay out.
const LOG_FILES: [&str; 5] = [
    "sarab.log",
    "settings.json",
    "layout.json",
    "status.json",
    "restore.json",
];

/// Where exported files go: the user's Downloads folder.
// ponytail: %USERPROFILE%\Downloads; read the Downloads known folder if users move it and ask.
pub fn downloads() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Downloads"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

/// Zip the files that explain a problem into `dest`. Returns the zip's path.
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

/// Settings back to their defaults, except where the library lives and the record that Start
/// with Windows was set up once, so a reset neither loses wallpapers nor turns autostart back on.
pub fn reset(old: &Settings) -> Settings {
    Settings {
        library_dir: old.library_dir.clone(),
        autostart_set: old.autostart_set,
        ..Settings::default()
    }
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
}
