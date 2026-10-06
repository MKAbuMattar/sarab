//! The settings window: theme, backdrop, and opening it.

use super::*;

pub(crate) const AUTOSTART_FLAG: &str = "--autostart";

pub(crate) fn theme_of(name: &str) -> Option<tauri::Theme> {
    match name {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    }
}

/// Acrylic needs Windows 10 1903+, Mica Windows 11. Anything else gets the solid base color.
pub(crate) fn effects_for(name: &str) -> Option<tauri::utils::config::WindowEffectsConfig> {
    use tauri::window::{Effect, EffectsBuilder};
    let build = windows_version::OsVersion::current().build;
    let effect = match name {
        "acrylic" if build >= 18362 => Effect::Acrylic,
        "mica" if build >= 22000 => Effect::Mica,
        _ => return None,
    };
    Some(EffectsBuilder::new().effect(effect).build())
}

/// Effective theme for the page. WebView2 keeps the color scheme per profile, which every Sarab
/// window shares, so the page is told explicitly instead of trusting prefers-color-scheme.
pub(crate) fn page_theme(app: &AppHandle, setting: &str) -> &'static str {
    match setting {
        "light" => "light",
        "dark" => "dark",
        _ => match app.get_webview_window("main").and_then(|w| w.theme().ok()) {
            Some(tauri::Theme::Dark) => "dark",
            _ => "light",
        },
    }
}

/// Callers pass the settings because they may already hold the core lock.
pub(crate) fn open_ui(app: &AppHandle, s: &settings::Settings) {
    let (theme, backdrop) = (s.theme.as_str(), s.backdrop.as_str());
    update::check_if_stale(app, s.check_updates);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
        return;
    }
    let mut b = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("Sarab")
        .inner_size(1060.0, 720.0)
        .min_inner_size(760.0, 500.0)
        .theme(theme_of(theme))
        // Always transparent so the backdrop can change without reopening; the page paints a solid base when it is "solid".
        .transparent(true);
    if let Some(fx) = effects_for(backdrop) {
        b = b.effects(fx);
    }
    // Built on demand and destroyed on close, so it costs nothing while nobody looks at it.
    if let Err(e) = b.build() {
        log(format!("open ui: {e}"));
    }
}
