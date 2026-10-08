use super::*;
use gtk::prelude::*;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    pub raised: bool,
}

pub(in crate::os::linux) fn session_is_x11() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_none()
        && std::env::var("XDG_SESSION_TYPE").map_or(true, |t| t != "wayland")
        && std::env::var_os("DISPLAY").is_some()
}

pub fn find_desktop() -> Option<Desktop> {
    session_is_x11().then(Desktop::default)
}

pub fn desktop_alive(_d: &Desktop) -> bool {
    true
}

pub fn attach(_d: &Desktop, hwnd: HWND, r: RECT) -> Result<(), String> {
    let win = gtk_window(hwnd);
    if win.is_visible() {
        win.hide();
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
