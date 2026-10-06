//! The update notification, with buttons.

use super::*;

pub(in crate::os::windows) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The update toast: a reminder, so it stays on screen until answered, with two buttons. A click
/// on the toast itself sends "open"; the buttons send "install" and "later".
pub fn update_toast_xml(title: &str, body: &str, install: &str, later: &str) -> String {
    format!(
        concat!(
            r#"<toast scenario="reminder" launch="open" activationType="foreground">"#,
            r#"<visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>"#,
            r#"<actions><action content="{}" arguments="install" activationType="foreground"/>"#,
            r#"<action content="{}" arguments="later" activationType="foreground"/></actions></toast>"#
        ),
        xml_escape(title),
        xml_escape(body),
        xml_escape(install),
        xml_escape(later)
    )
}

/// Show `xml` as a toast from `app_id` and call `on_answer` with the arguments of whatever the
/// user clicked. The toast is kept alive here, or Windows drops its click events.
pub fn show_toast(
    app_id: &str,
    xml: &str,
    on_answer: impl Fn(String) + Send + Sync + 'static,
) -> windows::core::Result<()> {
    use windows::core::Interface;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::{
        ToastActivatedEventArgs, ToastNotification, ToastNotificationManager,
    };
    static SHOWN: std::sync::Mutex<Option<ToastNotification>> = std::sync::Mutex::new(None);
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    toast.Activated(&TypedEventHandler::new(
        move |_, args: windows::core::Ref<windows::core::IInspectable>| {
            let what = args
                .as_ref()
                .and_then(|a| a.cast::<ToastActivatedEventArgs>().ok())
                .and_then(|a| a.Arguments().ok())
                .map(|h| h.to_string())
                .unwrap_or_default();
            on_answer(what);
            Ok(())
        },
    ))?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)?;
    *SHOWN.lock().unwrap() = Some(toast);
    Ok(())
}
