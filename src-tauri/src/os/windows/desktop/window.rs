use super::*;

pub fn handle(w: &tauri::WebviewWindow) -> Option<HWND> {
    w.hwnd().ok()
}

pub fn os_build() -> u32 {
    windows_version::OsVersion::current().build
}

/// Native menus (the title bar's system menu, the tray menu) in Sarab's theme instead of
/// Windows' app mode: uxtheme's SetPreferredAppMode (ordinal 135) and FlushMenuThemes (136).
pub fn menu_theme(theme: &str) {
    use windows::core::{w, PCSTR};
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    // PreferredAppMode: 1 follows Windows, 2 always dark, 3 always light.
    let mode: i32 = match theme {
        "dark" => 2,
        "light" => 3,
        _ => 1,
    };
    unsafe {
        let Ok(ux) = LoadLibraryW(w!("uxtheme.dll")) else {
            return;
        };
        if let Some(f) = GetProcAddress(ux, PCSTR(135 as *const u8)) {
            let set: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(f);
            set(mode);
        }
        if let Some(f) = GetProcAddress(ux, PCSTR(136 as *const u8)) {
            let flush: unsafe extern "system" fn() = std::mem::transmute(f);
            flush();
        }
    }
}
