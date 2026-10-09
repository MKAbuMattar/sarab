use super::*;
use glib::object::ObjectType;

pub fn handle(w: &tauri::WebviewWindow) -> Option<HWND> {
    w.gtk_window().ok().map(|g| HWND(g.as_ptr() as *mut _))
}

pub fn os_build() -> u32 {
    0
}

/// GTK menus already follow the theme Tauri sets on the window.
pub fn menu_theme(_theme: &str) {}

pub(in crate::os::linux) fn gtk_window(h: HWND) -> gtk::Window {
    unsafe { glib::translate::from_glib_none(h.0 as *mut gtk::ffi::GtkWindow) }
}
