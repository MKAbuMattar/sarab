use super::*;
use gtk::prelude::*;
use gtk_layer_shell::{Edge, Layer, LayerShell};

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    pub raised: bool,
}

pub(in crate::os::linux) fn on_wayland() -> bool {
    gdk::Display::default().is_some_and(|d| d.type_().name() == "GdkWaylandDisplay")
}

/// GNOME on Wayland has no layer-shell, but its XWayland honours desktop-type windows,
/// so Sarab runs there as an X11 app. Call before GTK starts.
pub fn pick_backend() {
    let gnome = std::env::var("XDG_CURRENT_DESKTOP")
        .is_ok_and(|d| d.split(':').any(|p| p.eq_ignore_ascii_case("gnome")));
    if gnome
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("GDK_BACKEND").is_none()
    {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

pub fn find_desktop() -> Option<Desktop> {
    let ok = match gdk::Display::default() {
        Some(_) if on_wayland() => gtk_layer_shell::is_supported(),
        Some(_) => true,
        None => false,
    };
    ok.then(Desktop::default)
}

pub fn desktop_alive(_d: &Desktop) -> bool {
    true
}

pub fn attach(_d: &Desktop, hwnd: HWND, r: RECT) -> Result<(), String> {
    let win = gtk_window(hwnd);
    if win.is_visible() {
        win.hide();
    }
    if on_wayland() {
        layer(&win, r);
        return Ok(());
    }
    win.set_type_hint(gdk::WindowTypeHint::Desktop);
    win.set_decorated(false);
    win.set_skip_taskbar_hint(true);
    win.set_skip_pager_hint(true);
    win.set_accept_focus(false);
    win.set_focus_on_map(false);
    win.set_keep_below(true);
    win.stick();
    win.move_(r.left, r.top);
    win.resize((r.right - r.left).max(1), (r.bottom - r.top).max(1));
    Ok(())
}

/// A layer-shell surface on the background layer, filling one output.
fn layer(win: &gtk::Window, r: RECT) {
    if !win.is_layer_window() {
        if win.is_realized() {
            win.unrealize();
        }
        win.init_layer_shell();
        win.set_layer(Layer::Background);
        win.set_namespace("sarab-wallpaper");
        win.set_exclusive_zone(-1);
        win.set_keyboard_interactivity(false);
        for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
            win.set_anchor(edge, true);
        }
    }
    if let Some(m) = gdk::Display::default().and_then(|d| d.monitor_at_point(r.left, r.top)) {
        win.set_monitor(&m);
    }
}

pub fn ensure_order(_d: &Desktop, _wins: &[HWND]) -> bool {
    false
}

pub fn show(hwnd: HWND, visible: bool) {
    let win = gtk_window(hwnd);
    if visible {
        win.show_all();
    } else {
        win.hide();
    }
}

pub fn refresh_desktop(_d: Option<&Desktop>) {}

pub fn span_per_output() -> bool {
    on_wayland()
}
