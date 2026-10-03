//! Theme, Sarab's colors, and the window backdrop. Values match the web window it replaces:
//! Jordanian Identity Colors for accent and danger, WinUI fills for surfaces.

use super::Model;
use gpui_kit::component::{ActiveTheme as _, Theme, ThemeMode};
use gpui_kit::*;

/// Dead Sea Blue, the light accent: 6.53:1 on #F3F3F3, white text on it 7.24:1.
const DEAD_SEA: u32 = 0x2F5D6B;
/// Wadi Rum Sand, the dark accent: 7.29:1 on #202020, #1A1A1A text on it 7.79:1.
const WADI_RUM_SAND: u32 = 0xD9A36A;
/// Keffiyeh Red for destructive buttons, white text on it 9.05:1.
const KEFFIYEH: u32 = 0x8B1E2D;

pub fn backdrop(name: &str) -> WindowBackgroundAppearance {
    let build = windows_version::OsVersion::current().build;
    match name {
        // Acrylic needs Windows 10 1903+, Mica Windows 11. Anything else gets the solid base color.
        "acrylic" if build >= 18362 => WindowBackgroundAppearance::Blurred,
        "mica" if build >= 22000 => WindowBackgroundAppearance::MicaBackdrop,
        _ => WindowBackgroundAppearance::Opaque,
    }
}

fn c(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn ca(hex_rgba: u32) -> Hsla {
    rgba(hex_rgba).into()
}

/// Theme mode, Sarab's tokens, and the backdrop, from the settings. "system" follows Windows.
pub fn apply(window: Option<&mut Window>, cx: &mut App) {
    let Some(snap) = Model::get(cx).snap.clone() else {
        return;
    };
    let s = &snap.settings;
    match s.theme.as_str() {
        "light" => Theme::change(ThemeMode::Light, None, cx),
        "dark" => Theme::change(ThemeMode::Dark, None, cx),
        _ => Theme::sync_system_appearance(window, cx),
    }
    let bg = backdrop(&s.backdrop);
    let see_through = bg != WindowBackgroundAppearance::Opaque;
    let dark = cx.theme().mode.is_dark();
    Theme::update(cx, |t| {
        let k = &mut t.colors;
        let (accent, hover, on) = if dark {
            (c(WADI_RUM_SAND), c(0xE2B482), c(0x1A1A1A))
        } else {
            (c(DEAD_SEA), c(0x3A6B7A), c(0xFFFFFF))
        };
        k.primary = accent;
        k.primary_hover = hover;
        k.primary_active = hover;
        k.primary_foreground = on;
        k.button_primary = accent;
        k.button_primary_hover = hover;
        k.button_primary_active = hover;
        k.button_primary_foreground = on;
        k.ring = accent;
        k.link = accent;
        k.link_hover = hover;
        k.link_active = hover;
        k.switch = accent;
        k.slider_bar = accent;
        k.slider_thumb = on;
        k.progress_bar = accent;
        k.sidebar_primary = accent;
        k.sidebar_primary_foreground = on;
        k.danger = c(KEFFIYEH);
        k.button_danger = c(KEFFIYEH);
        k.button_danger_hover = c(0x9E2535);
        k.button_danger_active = c(0x9E2535);
        k.button_danger_foreground = c(0xFFFFFF);
        if see_through {
            // Windows draws Acrylic or Mica behind the window; surfaces stay translucent over it.
            k.background = ca(0x00000000);
            k.sidebar = ca(0x00000000);
            k.title_bar = ca(0x00000000);
            k.group_box = if dark { ca(0xFFFFFF0D) } else { ca(0xFFFFFFB3) };
        }
    });
    if let Some(w) = Model::get(cx).window {
        let _ = w.update(cx, |_, window, _| window.set_background_appearance(bg));
    }
}
