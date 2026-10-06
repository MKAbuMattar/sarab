//! Mouse input forwarded to interactive wallpapers.

use super::*;

/// Where wallpapers sit and which window takes their input: (screen rect, input window).
pub type Targets = std::sync::Arc<std::sync::Mutex<Vec<(RECT, isize)>>>;

/// The point in a wallpaper's own coordinates, when the cursor is over the bare desktop and
/// inside one of the wallpapers. Over any app window nothing is forwarded.
pub(in crate::os::windows) fn forward_target(
    x: i32,
    y: i32,
    over_desktop: bool,
    targets: &[(RECT, isize)],
) -> Option<(isize, i32, i32)> {
    if !over_desktop {
        return None;
    }
    targets
        .iter()
        .find(|(r, _)| r.left <= x && x < r.right && r.top <= y && y < r.bottom)
        .map(|(r, h)| (*h, x - r.left, y - r.top))
}

/// The child window WebView2 draws into and reads input from.
pub fn input_window(top: HWND) -> Option<isize> {
    unsafe extern "system" fn cb(h: HWND, out: LPARAM) -> BOOL {
        if class_of(h) == "Chrome_RenderWidgetHostHWND" {
            unsafe { *(out.0 as *mut isize) = raw(h) };
            return false.into();
        }
        true.into()
    }
    let mut found = 0isize;
    unsafe {
        let _ = EnumChildWindows(Some(top), Some(cb), LPARAM(&mut found as *mut _ as isize));
    }
    (found != 0).then_some(found)
}

/// A window's rect on screen.
pub fn window_rect(h: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe { GetWindowRect(h, &mut r) }.ok()?;
    Some(r)
}

pub(in crate::os::windows) static FORWARD: std::sync::OnceLock<Targets> =
    std::sync::OnceLock::new();

pub(in crate::os::windows) unsafe extern "system" fn mouse_hook(
    code: i32,
    wp: WPARAM,
    lp: LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    if code >= 0 {
        let msg = wp.0 as u32;
        if matches!(msg, WM_MOUSEMOVE | WM_LBUTTONDOWN | WM_LBUTTONUP) {
            let info = unsafe { &*(lp.0 as *const MSLLHOOKSTRUCT) };
            let under = unsafe { WindowFromPoint(info.pt) };
            let over_desktop = DESKTOP_CLASSES.contains(&class_of(under).as_str())
                || class_of(under) == "SHELLDLL_DefView";
            if let Some(t) = FORWARD.get() {
                let found = forward_target(info.pt.x, info.pt.y, over_desktop, &t.lock().unwrap());
                if let Some((h, x, y)) = found {
                    let keys = if msg == WM_LBUTTONDOWN { 1 } else { 0 }; // MK_LBUTTON
                    let pos = ((y as u32 & 0xFFFF) << 16) | (x as u32 & 0xFFFF);
                    unsafe {
                        let _ =
                            PostMessageW(Some(self::h(h)), msg, WPARAM(keys), LPARAM(pos as isize));
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

/// Run the mouse hook on its own thread until `running` turns false. Only one runs at a time.
pub fn forward_mouse(targets: Targets, running: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    use std::sync::atomic::Ordering;
    // The core keeps one target list for the life of the app; the hook reads it from here.
    let _ = FORWARD.set(targets);
    std::thread::spawn(move || unsafe {
        let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), None, 0) else {
            return;
        };
        let mut msg = MSG::default();
        while running.load(Ordering::Relaxed) {
            // The hook only runs while this thread pumps messages.
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = MsgWaitForMultipleObjects(None, false, 100, QS_ALLINPUT);
        }
        let _ = UnhookWindowsHookEx(hook);
    });
}
