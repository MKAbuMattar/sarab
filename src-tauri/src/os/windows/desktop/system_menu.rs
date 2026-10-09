use super::*;
use windows::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, IsZoomed, PostMessageW, SendMessageW, HTCAPTION, HTSYSMENU, SC_CLOSE, SC_KEYMENU,
    SC_MAXIMIZE, SC_MINIMIZE, SC_MOUSEMENU, SC_MOVE, SC_RESTORE, SC_SIZE, WM_CONTEXTMENU,
    WM_NCHITTEST, WM_SYSCOMMAND,
};

/// Where to open the window menu, in client pixels, and the window's state.
pub struct SystemMenu {
    pub x: i32,
    pub y: i32,
    pub keyboard: bool,
    pub maximized: bool,
}

/// Replaces the title bar's system menu with the app's own: a right-click on the title bar,
/// a click on its icon, and Alt+Space call `open` instead of showing the Windows menu.
pub fn own_system_menu(hwnd: HWND, open: fn(SystemMenu)) {
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(subclass), 0x5A2AB, open as usize);
    }
}

/// Runs a window menu choice the way the Windows menu would.
pub fn system_command(hwnd: HWND, action: &str) {
    let sc = match action {
        "restore" => SC_RESTORE,
        "move" => SC_MOVE,
        "size" => SC_SIZE,
        "minimize" => SC_MINIMIZE,
        "maximize" => SC_MAXIMIZE,
        "close" => SC_CLOSE,
        _ => return,
    };
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_SYSCOMMAND, WPARAM(sc as usize), LPARAM(0));
    }
}

unsafe extern "system" fn subclass(
    h: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    let open: fn(SystemMenu) = std::mem::transmute(data);
    let at = |screen: POINT, keyboard: bool| {
        let mut p = screen;
        let _ = ScreenToClient(h, &mut p);
        open(SystemMenu {
            x: p.x.max(0),
            y: p.y.max(0),
            keyboard,
            maximized: IsZoomed(h).as_bool(),
        });
    };
    let cursor = || {
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        p
    };
    match msg {
        // A right-click on the title bar reaches DefWindowProc as WM_CONTEXTMENU.
        WM_CONTEXTMENU if l.0 != -1 => {
            let hit = SendMessageW(h, WM_NCHITTEST, None, Some(l)).0 as u32;
            if matches!(hit, HTCAPTION | HTSYSMENU) {
                at(cursor(), false);
                return LRESULT(0);
            }
        }
        WM_SYSCOMMAND => match (w.0 as u32) & 0xFFF0 {
            SC_KEYMENU if l.0 == b' ' as isize => {
                at(POINT::default(), true);
                return LRESULT(0);
            }
            SC_MOUSEMENU => {
                at(cursor(), false);
                return LRESULT(0);
            }
            _ => {}
        },
        _ => {}
    }
    DefSubclassProc(h, msg, w, l)
}
