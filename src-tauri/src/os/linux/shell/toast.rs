use gtk::gio;
use gtk::glib::{self, ToVariant};

pub fn update_toast_xml(title: &str, body: &str, install: &str, later: &str) -> String {
    serde_json::json!([title, body, install, later]).to_string()
}

pub fn show_toast(
    _app_id: &str,
    xml: &str,
    on_answer: impl Fn(String) + Send + Sync + 'static,
) -> Result<(), String> {
    let [title, body, install, later]: [String; 4] =
        serde_json::from_str(xml).map_err(|e| e.to_string())?;
    let bus = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>)
        .map_err(|e| e.to_string())?;
    let actions = vec!["install".to_string(), install, "later".into(), later];
    let hints: std::collections::HashMap<String, glib::Variant> = Default::default();
    let args = ("Sarab", 0u32, "sarab", title, body, actions, hints, -1i32).to_variant();
    let reply = bus
        .call_sync(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
            Some(&args),
            None,
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
        )
        .map_err(|e| e.to_string())?;
    let Some((id,)) = reply.get::<(u32,)>() else {
        return Err("the notification service gave no id".into());
    };
    bus.signal_subscribe(
        Some("org.freedesktop.Notifications"),
        Some("org.freedesktop.Notifications"),
        Some("ActionInvoked"),
        Some("/org/freedesktop/Notifications"),
        None,
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, params| {
            if let Some((shown, action)) = params.get::<(u32, String)>() {
                if shown == id {
                    on_answer(action);
                }
            }
        },
    );
    Ok(())
}
