//! Window and tray text in the user's language.

use super::*;

/// A UI string for text Rust shows itself (tray menu, notifications), from the same files as the window.
pub(crate) fn ui_text(lang: &str, key: &str) -> String {
    let file = match lang {
        "ar" => include_str!("../../../ui/i18n/ar.json"),
        "de" => include_str!("../../../ui/i18n/de.json"),
        "es" => include_str!("../../../ui/i18n/es.json"),
        "fr" => include_str!("../../../ui/i18n/fr.json"),
        "ja" => include_str!("../../../ui/i18n/ja.json"),
        "pt" => include_str!("../../../ui/i18n/pt.json"),
        "ru" => include_str!("../../../ui/i18n/ru.json"),
        "tr" => include_str!("../../../ui/i18n/tr.json"),
        "zh" => include_str!("../../../ui/i18n/zh.json"),
        _ => include_str!("../../../ui/i18n/en.json"),
    };
    serde_json::from_str::<Value>(file)
        .ok()
        .and_then(|v| v.get(key).and_then(Value::as_str).map(String::from))
        .unwrap_or_else(|| key.to_string())
}
