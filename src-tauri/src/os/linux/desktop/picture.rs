use super::*;
use gtk::gio::prelude::*;

pub fn set_picture(_m: &Monitor, _path: &str) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn get_picture(_m: &Monitor) -> Option<String> {
    None
}

pub(in crate::os::linux) fn gnome_settings(schema: &str, key: &str) -> Option<gtk::gio::Settings> {
    let found = gtk::gio::SettingsSchemaSource::default()?.lookup(schema, true)?;
    found.has_key(key).then(|| gtk::gio::Settings::new(schema))
}

pub(in crate::os::linux) fn kde_config(
    file: &str,
    groups: &[&str],
    key: &str,
    value: &str,
) -> bool {
    let mut args = vec!["--file", file];
    for g in groups {
        args.extend(["--group", g]);
    }
    args.extend(["--key", key, value]);
    ["kwriteconfig6", "kwriteconfig5"].iter().any(|tool| {
        std::process::Command::new(tool)
            .args(&args)
            .status()
            .is_ok_and(|s| s.success())
    })
}

pub fn set_lock_screen(path: &Path) -> Result<(), String> {
    let uri = gtk::gio::File::for_path(path).uri().to_string();
    if let Some(s) = gnome_settings("org.gnome.desktop.screensaver", "picture-uri") {
        s.set_string("picture-uri", &uri)
            .map_err(|e| e.to_string())?;
        gtk::gio::Settings::sync();
        return Ok(());
    }
    let image = ["Greeter", "Wallpaper", "org.kde.image", "General"];
    if kde_config("kscreenlockerrc", &image, "Image", &uri) {
        return Ok(());
    }
    Err("this desktop has no lock screen picture Sarab can set".into())
}
