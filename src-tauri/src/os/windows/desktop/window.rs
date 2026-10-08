use super::*;

pub fn handle(w: &tauri::WebviewWindow) -> Option<HWND> {
    w.hwnd().ok()
}

pub fn os_build() -> u32 {
    windows_version::OsVersion::current().build
}
