//! Windows backend: WorkerW embedding, pause probes, OS picture wallpaper.

use crate::pause::Signals;
use windows::core::{w, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MapWindowPoints, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_SWITCHDESKTOP,
};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{
    DesktopWallpaper, IDesktopWallpaper, SHQueryUserNotificationState, QUNS_PRESENTATION_MODE,
    QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub key: String,
    pub rect: RECT,
    pub work: RECT,
}

/// Displays sorted left to right, then top to bottom, so `--display 0` is the leftmost.
pub fn monitors() -> Vec<Monitor> {
    unsafe extern "system" fn cb(h: HMONITOR, _: HDC, _: *mut RECT, out: LPARAM) -> BOOL {
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if unsafe { GetMonitorInfoW(h, &mut mi as *mut _ as *mut _) }.as_bool() {
            let n = mi
                .szDevice
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(mi.szDevice.len());
            let list = unsafe { &mut *(out.0 as *mut Vec<Monitor>) };
            list.push(Monitor {
                key: String::from_utf16_lossy(&mi.szDevice[..n]),
                rect: mi.monitorInfo.rcMonitor,
                work: mi.monitorInfo.rcWork,
            });
        }
        true.into()
    }
    let mut list: Vec<Monitor> = vec![];
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut list as *mut _ as isize));
    }
    list.sort_by_key(|m| (m.rect.left, m.rect.top));
    list
}

// ---- desktop embedding ----

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    /// Win11 24H2+: Progman has WS_EX_NOREDIRECTIONBITMAP and hosts DefView and WorkerW as children.
    pub raised: bool,
}

fn h(v: isize) -> HWND {
    HWND(v as *mut _)
}
fn raw(v: HWND) -> isize {
    v.0 as isize
}

pub fn find_desktop() -> Option<Desktop> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null()).ok()?;
        let raised =
            (GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32 & WS_EX_NOREDIRECTIONBITMAP.0) != 0;
        // 0x052C asks Progman to spawn the WorkerW behind the icons. Harmless if it already exists.
        SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(1),
            SMTO_NORMAL,
            1000,
            None,
        );

        struct Found {
            defview: isize,
            workerw: isize,
        }
        unsafe extern "system" fn cb(top: HWND, out: LPARAM) -> BOOL {
            let f = unsafe { &mut *(out.0 as *mut Found) };
            if let Ok(dv) =
                unsafe { FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null()) }
            {
                f.defview = raw(dv);
                // Classic layout: the WorkerW we want is the top-level sibling after the one holding DefView.
                f.workerw =
                    unsafe { FindWindowExW(None, Some(top), w!("WorkerW"), PCWSTR::null()) }
                        .map(raw)
                        .unwrap_or(0);
            }
            true.into()
        }
        let mut f = Found {
            defview: 0,
            workerw: 0,
        };
        let _ = EnumWindows(Some(cb), LPARAM(&mut f as *mut _ as isize));
        if raised {
            f.workerw = FindWindowExW(Some(progman), None, w!("WorkerW"), PCWSTR::null())
                .map(raw)
                .unwrap_or(0);
        }
        (f.workerw != 0).then_some(Desktop {
            progman: raw(progman),
            workerw: f.workerw,
            defview: f.defview,
            raised,
        })
    }
}

pub fn desktop_alive(d: &Desktop) -> bool {
    unsafe { IsWindow(Some(h(d.progman))).as_bool() && IsWindow(Some(h(d.workerw))).as_bool() }
}

/// Parent `hwnd` under the desktop icons and size it to `r` (screen coordinates, physical pixels).
pub fn attach(d: &Desktop, hwnd: HWND, r: RECT) -> windows::core::Result<()> {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let style = (style | WS_CHILD.0) & !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as isize);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        // Tool window: no taskbar button, no alt-tab.
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ((ex | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0) as isize,
        );
        let parent = if d.raised {
            // Raised desktop (24H2+): Microsoft's guidance is a layered child z-ordered between DefView and WorkerW.
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_LAYERED.0) as isize);
            SetLayeredWindowAttributes(
                hwnd,
                windows::Win32::Foundation::COLORREF(0),
                255,
                LWA_ALPHA,
            )?;
            d.progman
        } else {
            d.workerw
        };
        SetParent(hwnd, Some(h(parent)))?;
        let mut pts = [POINT {
            x: r.left,
            y: r.top,
        }];
        MapWindowPoints(None, Some(h(parent)), &mut pts);
        let after = if d.raised { Some(h(d.defview)) } else { None };
        let mut flags = SWP_NOACTIVATE | SWP_FRAMECHANGED;
        if !d.raised {
            flags |= SWP_NOZORDER;
        }
        SetWindowPos(
            hwnd,
            after,
            pts[0].x,
            pts[0].y,
            r.right - r.left,
            r.bottom - r.top,
            flags,
        )?;
        if d.raised {
            // WorkerW must stay the bottom child or it paints over us.
            SetWindowPos(
                h(d.workerw),
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )?;
        }
        Ok(())
    }
}

/// Children of Progman, top to bottom.
fn children(parent: HWND) -> Vec<isize> {
    let mut out = vec![];
    let mut c = unsafe { GetWindow(parent, GW_CHILD) };
    while let Ok(w) = c {
        if w.is_invalid() {
            break;
        }
        out.push(raw(w));
        c = unsafe { GetWindow(w, GW_HWNDNEXT) };
    }
    out
}

/// Raised desktop: keep every wallpaper between DefView (icons) and WorkerW (the plain
/// Windows wallpaper). If Explorer ever re-stacks Progman's children, WorkerW would paint
/// over us and the user would see the Windows wallpaper. Returns true when it had to repair.
pub fn ensure_order(d: &Desktop, wins: &[HWND]) -> bool {
    if !d.raised {
        return false;
    }
    let order = children(h(d.progman));
    let at = |w: isize| order.iter().position(|&x| x == w);
    let (Some(def), Some(wk)) = (at(d.defview), at(d.workerw)) else {
        return false;
    };
    let mut repaired = false;
    unsafe {
        for &w in wins {
            let ok = at(raw(w)).is_some_and(|i| def < i && i < wk);
            if !ok {
                let _ = SetWindowPos(
                    w,
                    Some(h(d.defview)),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
                repaired = true;
            }
            if !IsWindowVisible(w).as_bool() {
                show(w, true);
                repaired = true;
            }
        }
        if wk + 1 != order.len() || repaired {
            let _ = SetWindowPos(
                h(d.workerw),
                Some(HWND_BOTTOM),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            repaired |= wk + 1 != order.len();
        }
    }
    repaired
}

pub fn show(hwnd: HWND, visible: bool) {
    unsafe {
        let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
    }
}

/// Repaint the desktop so no stale wallpaper frame stays behind. Skipped on the raised desktop,
/// where it would destroy the WorkerW.
pub fn refresh_desktop(d: Option<&Desktop>) {
    if d.is_some_and(|d| d.raised) {
        return;
    }
    unsafe {
        let _ = SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, None, SPIF_UPDATEINIFILE);
    }
}

// ---- probes ----

const DESKTOP_CLASSES: [&str; 5] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "SysListView32",
];

fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn pid_of(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 260];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
                .is_ok();
        let _ = CloseHandle(p);
        ok.then(|| {
            let full = String::from_utf16_lossy(&buf[..len as usize]);
            full.rsplit('\\').next().unwrap_or(&full).to_string()
        })
    }
}

fn frame(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut r as *mut _ as *mut _,
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .ok()?;
    Some(r)
}

fn contains(outer: &RECT, inner: &RECT) -> bool {
    outer.left <= inner.left
        && outer.top <= inner.top
        && outer.right >= inner.right
        && outer.bottom >= inner.bottom
}

/// Visible, uncloaked, unminimized top-level windows of other processes, with their frame rects.
fn app_windows() -> Vec<(HWND, RECT)> {
    unsafe extern "system" fn cb(hwnd: HWND, out: LPARAM) -> BOOL {
        let list = unsafe { &mut *(out.0 as *mut Vec<(HWND, RECT)>) };
        unsafe {
            if !IsWindowVisible(hwnd).as_bool()
                || IsIconic(hwnd).as_bool()
                || pid_of(hwnd) == GetCurrentProcessId()
            {
                return true.into();
            }
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            // Click-through overlays and tool windows never count as covering the desktop.
            if ex & (WS_EX_TRANSPARENT.0 | WS_EX_TOOLWINDOW.0) != 0 {
                return true.into();
            }
            let mut cloaked = 0u32;
            let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
            if cloaked != 0 || DESKTOP_CLASSES.contains(&class_of(hwnd).as_str()) {
                return true.into();
            }
            if let Some(r) = frame(hwnd) {
                list.push((hwnd, r));
            }
        }
        true.into()
    }
    let mut list = vec![];
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut list as *mut _ as isize));
    }
    list
}

pub fn signals(mons: &[Monitor]) -> Signals {
    let wins = app_windows();
    let mut covered: Vec<bool> = mons
        .iter()
        .map(|m| wins.iter().any(|(_, r)| contains(r, &m.work)))
        .collect();

    let fg = unsafe { GetForegroundWindow() };
    let fg_ok = !fg.is_invalid();
    // Exclusive-fullscreen games may not report a normal frame; the shell still knows.
    if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
        if fg_ok && (state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE) {
            let hm = unsafe { MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST) };
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if unsafe { GetMonitorInfoW(hm, &mut mi as *mut _ as *mut _) }.as_bool() {
                for (i, m) in mons.iter().enumerate() {
                    if m.rect == mi.monitorInfo.rcMonitor {
                        covered[i] = true;
                    }
                }
            }
        }
    }

    let own = unsafe { GetCurrentProcessId() };
    let desktop_focused =
        !fg_ok || pid_of(fg) == own || DESKTOP_CLASSES.contains(&class_of(fg).as_str());
    let foreground_app = if fg_ok && !desktop_focused {
        exe_name(pid_of(fg))
    } else {
        None
    };

    let mut ps = SYSTEM_POWER_STATUS::default();
    let power = unsafe { GetSystemPowerStatus(&mut ps) }.is_ok();

    Signals {
        manual: None,
        on_battery: power && ps.ACLineStatus == 0,
        power_saver: power && ps.SystemStatusFlag == 1,
        locked: is_locked(),
        remote: unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0,
        foreground_app,
        desktop_focused,
        covered,
    }
}

/// The lock screen and UAC prompts switch to the secure desktop, which we cannot open.
fn is_locked() -> bool {
    match unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_SWITCHDESKTOP) } {
        Ok(d) => {
            unsafe {
                let _ = CloseDesktop(d);
            }
            false
        }
        Err(_) => true,
    }
}

// ---- picture wallpaper ----

fn wallpaper_api() -> windows::core::Result<IDesktopWallpaper> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
    }
}

/// IDesktopWallpaper monitor ids for the given monitor rects.
fn monitor_id(api: &IDesktopWallpaper, m: &Monitor) -> windows::core::Result<PWSTR> {
    unsafe {
        for i in 0..api.GetMonitorDevicePathCount()? {
            let id = api.GetMonitorDevicePathAt(i)?;
            if api.GetMonitorRECT(PCWSTR(id.0))? == m.rect {
                return Ok(id);
            }
        }
    }
    Err(windows::core::Error::from_hresult(
        windows::Win32::Foundation::E_INVALIDARG,
    ))
}

pub fn get_picture(m: &Monitor) -> Option<String> {
    let api = wallpaper_api().ok()?;
    unsafe {
        let id = monitor_id(&api, m).ok()?;
        api.GetWallpaper(PCWSTR(id.0)).ok()?.to_string().ok()
    }
}

pub fn set_picture(m: &Monitor, path: &str) -> windows::core::Result<()> {
    let api = wallpaper_api()?;
    unsafe {
        let id = monitor_id(&api, m)?;
        api.SetWallpaper(PCWSTR(id.0), &HSTRING::from(path))
    }
}
