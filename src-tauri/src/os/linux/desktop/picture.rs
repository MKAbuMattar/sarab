use super::*;
use gtk::gio::prelude::*;

pub fn set_picture(m: &Monitor, path: &str) -> Result<(), String> {
    let uri = gtk::gio::File::for_path(path).uri().to_string();
    if let Some(s) = gnome_settings("org.gnome.desktop.background", "picture-uri") {
        for key in ["picture-uri", "picture-uri-dark"] {
            if s.settings_schema().is_some_and(|k| k.has_key(key)) {
                s.set_string(key, &uri).map_err(|e| e.to_string())?;
            }
        }
        gtk::gio::Settings::sync();
        return Ok(());
    }
    if plasma_wallpaper(m, &uri) {
        return Ok(());
    }
    Err("this desktop has no wallpaper setting Sarab can change".into())
}

pub fn get_picture(_m: &Monitor) -> Option<String> {
    let s = gnome_settings("org.gnome.desktop.background", "picture-uri")?;
    let uri = s.string("picture-uri");
    gtk::gio::File::for_uri(&uri)
        .path()
        .map(|p| p.to_string_lossy().into_owned())
}

pub(in crate::os::linux) fn plasma_script(m: &Monitor, uri: &str) -> String {
    let at = serde_json::json!([m.rect.left, m.rect.top]);
    let uri = serde_json::json!(uri);
    format!(
        "const at = {at}; for (const d of desktops()) {{ const g = screenGeometry(d.screen); \
         if (g.x !== at[0] || g.y !== at[1]) continue; d.wallpaperPlugin = 'org.kde.image'; \
         d.currentConfigGroup = ['Wallpaper', 'org.kde.image', 'General']; \
         d.writeConfig('Image', {uri}); }}"
    )
}

fn plasma_wallpaper(m: &Monitor, uri: &str) -> bool {
    use gtk::gio;
    use gtk::glib::ToVariant;
    let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>) else {
        return false;
    };
    bus.call_sync(
        Some("org.kde.plasmashell"),
        "/PlasmaShell",
        "org.kde.PlasmaShell",
        "evaluateScript",
        Some(&(plasma_script(m, uri),).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        5000,
        None::<&gio::Cancellable>,
    )
    .is_ok()
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
