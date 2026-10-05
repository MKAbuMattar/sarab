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

/// Whether windows together hide `area`: every point of a 12 by 12 grid lies inside one of
/// them. Two windows snapped side by side cover a display as well as one maximized window does.
// ponytail: grid sampling, so a gap narrower than a grid cell (area/12) still counts as covered.
fn covered(area: &RECT, wins: &[RECT]) -> bool {
    const N: i32 = 12;
    let (w, h) = (area.right - area.left, area.bottom - area.top);
    if w <= 0 || h <= 0 {
        return false;
    }
    (0..N).all(|i| {
        (0..N).all(|j| {
            let x = area.left + w * (2 * i + 1) / (2 * N);
            let y = area.top + h * (2 * j + 1) / (2 * N);
            wins.iter()
                .any(|r| r.left <= x && x < r.right && r.top <= y && y < r.bottom)
        })
    })
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
    let rects: Vec<RECT> = app_windows().into_iter().map(|(_, r)| r).collect();
    let mut covered: Vec<bool> = mons
        .iter()
        .map(|m| self::covered(&m.work, &rects))
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
        // The Windows screensaver hides the desktop just as the lock screen does.
        locked: is_locked() || screensaver_running(),
        remote: unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0,
        foreground_app,
        desktop_focused,
        covered,
        // Set by the tick, which keeps the readings between calls.
        cpu_busy: false,
    }
}

/// Is the Windows screensaver on screen right now?
fn screensaver_running() -> bool {
    let mut on = BOOL(0);
    unsafe {
        SystemParametersInfoW(
            SPI_GETSCREENSAVERRUNNING,
            0,
            Some(&mut on as *mut _ as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    }
    .is_ok()
        && on.as_bool()
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

/// Processes that belong to Sarab: this one and every process it started, at any depth
/// (WebView2's browser, renderers and its audio service).
fn own_processes() -> std::collections::HashSet<u32> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let mut parent = std::collections::HashMap::new();
    unsafe {
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut e = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap, &mut e).is_ok();
            while ok {
                parent.insert(e.th32ProcessID, e.th32ParentProcessID);
                ok = Process32NextW(snap, &mut e).is_ok();
            }
            let _ = CloseHandle(snap);
        }
    }
    let me = unsafe { GetCurrentProcessId() };
    let mut own = std::collections::HashSet::from([me]);
    // A few passes reach grandchildren; a PID reused after its parent exited never links to us
    // because the chain stops at a PID that is not ours.
    for _ in 0..4 {
        for (&pid, &ppid) in &parent {
            if own.contains(&ppid) {
                own.insert(pid);
            }
        }
    }
    own
}

/// Is another app making sound right now? Only sessions that are active and above a whisper
/// count, so a paused player or a silent stream does not mute the wallpaper.
pub fn other_audio_playing() -> bool {
    use windows::core::Interface;
    use windows::Win32::Media::Audio::Endpoints::IAudioMeterInformation;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, AudioSessionStateActive, IAudioSessionControl2, IAudioSessionManager2,
        IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    let own = own_processes();
    let run = || -> windows::core::Result<bool> {
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let mgr: IAudioSessionManager2 = dev.Activate(CLSCTX_ALL, None)?;
            let list = mgr.GetSessionEnumerator()?;
            for i in 0..list.GetCount()? {
                let s = list.GetSession(i)?;
                if s.GetState()? != AudioSessionStateActive {
                    continue;
                }
                let pid = s.cast::<IAudioSessionControl2>()?.GetProcessId()?;
                // pid 0 is the system sounds session.
                if pid == 0 || own.contains(&pid) {
                    continue;
                }
                if s.cast::<IAudioMeterInformation>()?.GetPeakValue()? > 0.01 {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    };
    run().unwrap_or(false)
}

// ---- system information for web wallpapers ----

fn filetime(f: windows::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

/// Idle and busy CPU time since boot, in 100 ns units, summed over every core.
pub fn cpu_times() -> (u64, u64) {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;
    let (mut idle, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
        return (0, 0);
    }
    // Kernel time includes idle time.
    let idle = filetime(idle);
    (idle, filetime(kernel) + filetime(user) - idle)
}

/// Available and total memory in bytes.
pub fn memory() -> (u64, u64) {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut m) }.map_or((0, 0), |_| (m.ullAvailPhys, m.ullTotalPhys))
}

/// Bytes received and sent since boot by physical network adapters that are up. Filter and
/// virtual interfaces are skipped, since they count the same traffic again.
pub fn net_octets() -> (u64, u64) {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
    let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
    if unsafe { GetIfTable2(&mut table) }.is_err() || table.is_null() {
        return (0, 0);
    }
    let rows = unsafe {
        std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize)
    };
    const LOOPBACK: u32 = 24;
    let (mut down, mut up) = (0, 0);
    for r in rows {
        let hardware = r.InterfaceAndOperStatusFlags._bitfield & 1 != 0;
        if hardware && r.Type != LOOPBACK && r.OperStatus == IfOperStatusUp {
            down += r.InOctets;
            up += r.OutOctets;
        }
    }
    unsafe { FreeMibTable(table as *const _) };
    (down, up)
}

/// The CPU's marketing name, from the registry.
pub fn cpu_name() -> String {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ};
    let mut buf = [0u16; 128];
    let mut len = (buf.len() * 2) as u32;
    let ok = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            w!(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0"),
            w!("ProcessorNameString"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut len),
        )
    }
    .is_ok();
    let n = if ok {
        (len as usize / 2).saturating_sub(1)
    } else {
        0
    };
    String::from_utf16_lossy(&buf[..n]).trim().to_string()
}

/// The primary display adapter's name.
pub fn gpu_name() -> String {
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE,
    };
    let mut i = 0;
    loop {
        let mut d = DISPLAY_DEVICEW {
            cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32,
            ..Default::default()
        };
        if !unsafe { EnumDisplayDevicesW(PCWSTR::null(), i, &mut d, 0) }.as_bool() {
            return String::new();
        }
        if d.StateFlags.contains(DISPLAY_DEVICE_PRIMARY_DEVICE) {
            let n = d
                .DeviceString
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(d.DeviceString.len());
            return String::from_utf16_lossy(&d.DeviceString[..n]);
        }
        i += 1;
    }
}

/// The track Windows media controls report now (Spotify, a browser tab, the Media Player app),
/// as title, artist, album, album artist and cover art bytes. None when nothing plays.
/// Blocks on WinRT calls, so call it off the main thread.
pub fn now_playing() -> Option<(String, String, String, String, Vec<u8>)> {
    use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager;
    use windows::Storage::Streams::DataReader;
    let manager = Manager::RequestAsync().ok()?.join().ok()?;
    let session = manager.GetCurrentSession().ok()?;
    let p = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let s = |r: windows::core::Result<HSTRING>| r.map(|h| h.to_string()).unwrap_or_default();
    let art = (|| -> windows::core::Result<Vec<u8>> {
        let stream = p.Thumbnail()?.OpenReadAsync()?.join()?;
        let size = stream.Size()?.min(4 << 20) as u32;
        let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
        reader.LoadAsync(size)?.join()?;
        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes)?;
        Ok(bytes)
    })()
    .unwrap_or_default();
    Some((
        s(p.Title()),
        s(p.Artist()),
        s(p.AlbumTitle()),
        s(p.AlbumArtist()),
        art,
    ))
}

/// The Windows folder picker. None when cancelled. Runs its own message loop, so call it from
/// a thread that may block.
pub fn pick_folder(
    owner: Option<HWND>,
    start: Option<&std::path::Path>,
) -> Option<std::path::PathBuf> {
    use windows::Win32::System::Com::{CoTaskMemFree, CoUninitialize};
    use windows::Win32::UI::Shell::{
        FileOpenDialog, IFileOpenDialog, IShellItem, SHCreateItemFromParsingName, FOS_PICKFOLDERS,
        SIGDN_FILESYSPATH,
    };
    unsafe {
        let init = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let pick = || -> windows::core::Result<std::path::PathBuf> {
            let d: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_ALL)?;
            d.SetOptions(d.GetOptions()? | FOS_PICKFOLDERS)?;
            if let Some(start) = start {
                if let Ok(item) =
                    SHCreateItemFromParsingName::<_, _, IShellItem>(&HSTRING::from(start), None)
                {
                    let _ = d.SetFolder(&item);
                }
            }
            d.Show(owner)?;
            let p = d.GetResult()?.GetDisplayName(SIGDN_FILESYSPATH)?;
            let s = p.to_string();
            CoTaskMemFree(Some(p.0 as *const _));
            Ok(s.map_err(|_| windows::core::Error::empty())?.into())
        };
        let r = pick().ok();
        if init {
            CoUninitialize();
        }
        r
    }
}

// ---- thumbnails ----

use windows::Win32::Graphics::Imaging::{IWICBitmapSource, IWICImagingFactory};

fn wic() -> windows::core::Result<IWICImagingFactory> {
    use windows::Win32::Graphics::Imaging::CLSID_WICImagingFactory;
    use windows::Win32::System::Com::CLSCTX_INPROC_SERVER;
    unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
}

/// Write `source` as a PNG no wider than `max` pixels, keeping its shape.
fn save_png(
    f: &IWICImagingFactory,
    source: &IWICBitmapSource,
    dest: &std::path::Path,
    max: u32,
) -> windows::core::Result<()> {
    use windows::core::Interface;
    use windows::Win32::Foundation::GENERIC_WRITE;
    use windows::Win32::Graphics::Imaging::{
        GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA, WICBitmapEncoderNoCache,
        WICBitmapInterpolationModeFant,
    };
    unsafe {
        let (mut w, mut h) = (0, 0);
        source.GetSize(&mut w, &mut h)?;
        let scaled: IWICBitmapSource = if w > max {
            let s = f.CreateBitmapScaler()?;
            s.Initialize(
                source,
                max,
                ((u64::from(h) * u64::from(max)) / u64::from(w.max(1))).max(1) as u32,
                WICBitmapInterpolationModeFant,
            )?;
            s.cast()?
        } else {
            source.clone()
        };
        scaled.GetSize(&mut w, &mut h)?;
        let stream = f.CreateStream()?;
        stream.InitializeFromFilename(&HSTRING::from(dest.as_os_str()), GENERIC_WRITE.0)?;
        let enc = f.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        enc.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let (mut frame, mut bag) = (None, None);
        enc.CreateNewFrame(&mut frame, &mut bag)?;
        let frame = frame.ok_or_else(windows::core::Error::empty)?;
        frame.Initialize(bag.as_ref())?;
        frame.SetSize(w, h)?;
        let mut fmt = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut fmt)?;
        frame.WriteSource(&scaled, std::ptr::null())?;
        frame.Commit()?;
        enc.Commit()
    }
}

/// The Explorer thumbnail of `src` (a video, GIF or picture) saved as a PNG at `dest`.
pub fn shell_thumbnail(
    src: &std::path::Path,
    dest: &std::path::Path,
    size: i32,
) -> Result<(), String> {
    use windows::core::Interface;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::{DeleteObject, HPALETTE};
    use windows::Win32::Graphics::Imaging::WICBitmapIgnoreAlpha;
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
    };
    let run = || -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let item: IShellItemImageFactory =
                SHCreateItemFromParsingName(&HSTRING::from(src.as_os_str()), None)?;
            let hbm = item.GetImage(SIZE { cx: size, cy: size }, SIIGBF_BIGGERSIZEOK)?;
            let f = wic()?;
            // Thumbnails are opaque; some providers leave the alpha byte at 0.
            let bmp = f.CreateBitmapFromHBITMAP(hbm, HPALETTE::default(), WICBitmapIgnoreAlpha);
            let _ = DeleteObject(hbm.into());
            save_png(&f, &bmp?.cast()?, dest, size as u32)
        }
    };
    run().map_err(|e| format!("thumbnail of {}: {e}", src.display()))
}

/// Re-save the PNG at `src` as `dest`, no wider than `max` pixels.
pub fn shrink_png(src: &std::path::Path, dest: &std::path::Path, max: u32) -> Result<(), String> {
    use windows::core::Interface;
    use windows::Win32::Foundation::GENERIC_READ;
    use windows::Win32::Graphics::Imaging::WICDecodeMetadataCacheOnDemand;
    let run = || -> windows::core::Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let f = wic()?;
            let d = f.CreateDecoderFromFilename(
                &HSTRING::from(src.as_os_str()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnDemand,
            )?;
            let frame = d.GetFrame(0)?;
            save_png(&f, &frame.cast()?, dest, max)
        }
    };
    run().map_err(|e| e.to_string())
}

/// Capture what the default output plays (WASAPI loopback) while `wanted`, as mono float
/// samples, and hand the last 1024 to `each` about every 33 ms. Silence is sent as zeros once.
pub fn loopback(
    wanted: &std::sync::atomic::AtomicBool,
    each: &mut dyn FnMut(&[f32]),
) -> windows::core::Result<()> {
    use std::sync::atomic::Ordering;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator,
        MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
        AUDCLNT_STREAMFLAGS_LOOPBACK,
    };
    use windows::Win32::System::Com::CoTaskMemFree;
    unsafe {
        let _ = CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let dev = en.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let client: IAudioClient = dev.Activate(CLSCTX_ALL, None)?;
        let fmt = client.GetMixFormat()?;
        let (channels, bits, tag) = (
            (*fmt).nChannels as usize,
            (*fmt).wBitsPerSample,
            (*fmt).wFormatTag,
        );
        // The shared-mode mix format is 32-bit float on every Windows version Sarab supports.
        let float = bits == 32 && (tag == 3 || tag == 0xFFFE);
        let init = client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_LOOPBACK,
            10_000_000,
            0,
            fmt,
            None,
        );
        CoTaskMemFree(Some(fmt as *const _));
        init?;
        if !float || channels == 0 {
            return Err(windows::core::Error::new(
                windows::core::HRESULT(-1),
                "the mix format is not 32-bit float",
            ));
        }
        let cap: IAudioCaptureClient = client.GetService()?;
        client.Start()?;
        let mut ring: Vec<f32> = Vec::with_capacity(4096);
        let mut silent_sent = false;
        while wanted.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(33));
            let mut got = false;
            while cap.GetNextPacketSize()? > 0 {
                let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
                cap.GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 == 0 && !data.is_null() {
                    let s =
                        std::slice::from_raw_parts(data as *const f32, frames as usize * channels);
                    ring.extend(
                        s.chunks(channels)
                            .map(|f| f.iter().sum::<f32>() / channels as f32),
                    );
                    got = true;
                }
                cap.ReleaseBuffer(frames)?;
            }
            if ring.len() > 4096 {
                ring.drain(..ring.len() - 1024);
            }
            if got {
                silent_sent = false;
                each(&ring[ring.len().saturating_sub(1024)..]);
            } else if !silent_sent {
                ring.clear();
                each(&[]);
                silent_sent = true;
            }
        }
        let _ = client.Stop();
        Ok(())
    }
}

/// Milliseconds since the last key press or mouse move anywhere in this session, and the tick
/// count of that input, which changes whenever the user touches anything.
pub fn last_input() -> (u32, u32) {
    use windows::Win32::System::SystemInformation::GetTickCount;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    let mut li = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    if !unsafe { GetLastInputInfo(&mut li) }.as_bool() {
        return (0, 0);
    }
    (unsafe { GetTickCount() }.wrapping_sub(li.dwTime), li.dwTime)
}

// ---- mouse input for interactive wallpapers ----

/// Where wallpapers sit and which window takes their input: (screen rect, input window).
pub type Targets = std::sync::Arc<std::sync::Mutex<Vec<(RECT, isize)>>>;

/// The point in a wallpaper's own coordinates, when the cursor is over the bare desktop and
/// inside one of the wallpapers. Over any app window nothing is forwarded.
fn forward_target(
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

static FORWARD: std::sync::OnceLock<Targets> = std::sync::OnceLock::new();

unsafe extern "system" fn mouse_hook(
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

// ---- update notification ----

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The update toast: a reminder, so it stays on screen until answered, with two buttons. A click
/// on the toast itself sends "open"; the buttons send "install" and "later".
pub fn update_toast_xml(title: &str, body: &str, install: &str, later: &str) -> String {
    format!(
        concat!(
            r#"<toast scenario="reminder" launch="open" activationType="foreground">"#,
            r#"<visual><binding template="ToastGeneric"><text>{}</text><text>{}</text></binding></visual>"#,
            r#"<actions><action content="{}" arguments="install" activationType="foreground"/>"#,
            r#"<action content="{}" arguments="later" activationType="foreground"/></actions></toast>"#
        ),
        xml_escape(title),
        xml_escape(body),
        xml_escape(install),
        xml_escape(later)
    )
}

/// Show `xml` as a toast from `app_id` and call `on_answer` with the arguments of whatever the
/// user clicked. The toast is kept alive here, or Windows drops its click events.
pub fn show_toast(
    app_id: &str,
    xml: &str,
    on_answer: impl Fn(String) + Send + Sync + 'static,
) -> windows::core::Result<()> {
    use windows::core::Interface;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::Foundation::TypedEventHandler;
    use windows::UI::Notifications::{
        ToastActivatedEventArgs, ToastNotification, ToastNotificationManager,
    };
    static SHOWN: std::sync::Mutex<Option<ToastNotification>> = std::sync::Mutex::new(None);
    let doc = XmlDocument::new()?;
    doc.LoadXml(&HSTRING::from(xml))?;
    let toast = ToastNotification::CreateToastNotification(&doc)?;
    toast.Activated(&TypedEventHandler::new(
        move |_, args: windows::core::Ref<windows::core::IInspectable>| {
            let what = args
                .as_ref()
                .and_then(|a| a.cast::<ToastActivatedEventArgs>().ok())
                .and_then(|a| a.Arguments().ok())
                .map(|h| h.to_string())
                .unwrap_or_default();
            on_answer(what);
            Ok(())
        },
    ))?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)?;
    *SHOWN.lock().unwrap() = Some(toast);
    Ok(())
}

/// CPU time used by Sarab and every process it started (WebView2 included), in 100 ns units.
pub fn own_cpu_time() -> u64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetProcessTimes;
    own_processes()
        .into_iter()
        .filter_map(|pid| unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let (mut c, mut e, mut k, mut u) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let r = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
            let _ = CloseHandle(h);
            r.ok().map(|_| filetime(k) + filetime(u))
        })
        .sum()
}

// ---- app wallpapers ----

/// A program running as a wallpaper. It lives in a job object that ends it, and anything it
/// started, when this is dropped, and also when Sarab exits or crashes.
pub struct AppProcess {
    job: isize,
    pub pid: u32,
    /// Its window, once it has one and sits behind the icons.
    pub hwnd: Option<isize>,
}

impl Drop for AppProcess {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(windows::Win32::Foundation::HANDLE(self.job as _));
        }
    }
}

pub fn launch_app(exe: &std::path::Path, args: &[&str]) -> Result<AppProcess, String> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::*;
    let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let mut p = AppProcess {
        job: job.0 as isize,
        pid: 0,
        hwnd: None,
    };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        )
    }
    .map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir(exe.parent().unwrap_or(std::path::Path::new(".")))
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    // ponytail: the program runs a moment before it joins the job, so a process it starts in
    // that moment escapes; start it suspended and resume it after joining if that ever matters.
    if let Err(e) = unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())) } {
        let _ = child.kill();
        return Err(e.to_string());
    }
    p.pid = child.id();
    Ok(p)
}

/// The visible, unowned top-level window of process `pid`, once it has one.
pub fn main_window(pid: u32) -> Option<HWND> {
    unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
        let f = &mut *(l.0 as *mut (u32, Option<HWND>));
        let mut p = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut p));
        if p == f.0
            && IsWindowVisible(hwnd).as_bool()
            && GetWindow(hwnd, GW_OWNER).map_or(true, |o| o.is_invalid())
        {
            f.1 = Some(hwnd);
            return false.into();
        }
        true.into()
    }
    let mut f: (u32, Option<HWND>) = (pid, None);
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut f as *mut _ as isize));
    }
    f.1
}

// ---- command line ----

/// `path` (a PATH value) with `dir` added at the end or taken out. Everything else stays exactly
/// as it was, %VARIABLES% and empty entries included.
pub fn path_with(path: &str, dir: &str, add: bool) -> String {
    let same = |p: &str| {
        p.trim()
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(dir.trim_end_matches('\\'))
    };
    let present = path.split(';').any(same);
    if add == present {
        path.to_string()
    } else if add {
        if path.is_empty() {
            dir.to_string()
        } else if path.ends_with(';') {
            // Keep the trailing ; so taking the folder out again gives back the same string.
            format!("{path}{dir};")
        } else {
            format!("{path};{dir}")
        }
    } else {
        path.split(';')
            .filter(|p| !same(p))
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// Add Sarab's folder to the user's PATH, or take it out, so `sarab` works in any new terminal.
/// The installer and uninstaller call this; NSIS cannot do it, since its strings stop at 1024
/// characters and would cut a long PATH short.
pub fn set_on_path(add: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::*;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or("no folder")?
        .to_string_lossy()
        .into_owned();
    let mut key = HKEY::default();
    unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Environment"),
            None,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &mut key,
        )
    }
    .ok()
    .map_err(|e| e.to_string())?;
    // Read without expanding, so %USERPROFILE% and the like are written back as they were.
    let mut len = 0u32;
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND;
    let found =
        unsafe { RegGetValueW(key, None, w!("Path"), flags, None, None, Some(&mut len)) }.is_ok();
    let mut buf = vec![0u16; (len as usize / 2).max(1)];
    if found {
        unsafe {
            RegGetValueW(
                key,
                None,
                w!("Path"),
                flags,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut len),
            )
        }
        .ok()
        .map_err(|e| e.to_string())?;
    }
    let cur = String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]);
    let cur = if found { cur } else { String::new() };
    let next = path_with(&cur, &dir, add);
    let r = if next == cur {
        Ok(())
    } else {
        let wide: Vec<u16> = next.encode_utf16().chain([0]).collect();
        let bytes =
            unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        unsafe { RegSetValueExW(key, w!("Path"), None, REG_EXPAND_SZ, Some(bytes)) }
            .ok()
            .map_err(|e| e.to_string())
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    r?;
    // Tell Explorer, so terminals opened from now on see the new PATH.
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(w!("Environment").as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
    Ok(())
}

/// Print `msg` in the terminal that started Sarab, if one did. A release build is a windowed
/// app, which has no console of its own.
pub fn tell_terminal(msg: &str) {
    use std::io::Write;
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok() {
        let _ = writeln!(std::io::stderr(), "\n{msg}");
    }
}

/// Installed memory in bytes, 0 if Windows will not say.
pub fn total_ram() -> u64 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut m = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    unsafe { GlobalMemoryStatusEx(&mut m) }.map_or(0, |_| m.ullTotalPhys)
}

/// Move a file or folder to the Recycle Bin, so a deleted wallpaper can be restored.
pub fn recycle(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };
    // The API takes a list ending in two NULs.
    let from: Vec<u16> = path.as_os_str().encode_wide().chain([0, 0]).collect();
    let mut op = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    let code = unsafe { SHFileOperationW(&mut op) };
    if code != 0 || op.fAnyOperationsAborted.as_bool() || path.exists() {
        return Err(format!(
            "could not move {} to the Recycle Bin (code {code})",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn tiled_windows_cover_a_display() {
        let work = r(0, 0, 1920, 1040);
        // One maximized window (its frame sits a few pixels outside the work area).
        assert!(covered(&work, &[r(-7, -7, 1927, 1047)]));
        // Two windows snapped left and right.
        assert!(covered(&work, &[r(0, 0, 960, 1040), r(960, 0, 1920, 1040)]));
        // Four quarters.
        let q = [
            r(0, 0, 960, 520),
            r(960, 0, 1920, 520),
            r(0, 520, 960, 1040),
            r(960, 520, 1920, 1040),
        ];
        assert!(covered(&work, &q));
        // Half the screen is still desktop.
        assert!(!covered(&work, &[r(0, 0, 960, 1040)]));
        // A window on the other display does not count.
        assert!(!covered(&work, &[r(1920, 0, 3840, 1040)]));
        // A strip of desktop left between two windows is seen.
        assert!(!covered(
            &work,
            &[r(0, 0, 800, 1040), r(1100, 0, 1920, 1040)]
        ));
        assert!(!covered(&work, &[]));
    }

    #[test]
    fn shell_thumbnail_writes_png() {
        let d = std::env::temp_dir().join(format!("sarab-test-thumb-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // A 64 by 32 24-bit BMP, written by hand so the test needs no fixture.
        let (w, h) = (64u32, 32u32);
        let row = (w * 3).div_ceil(4) * 4;
        let mut bmp = vec![];
        bmp.extend(b"BM");
        bmp.extend((54 + row * h).to_le_bytes());
        bmp.extend([0u8; 4]);
        bmp.extend(54u32.to_le_bytes());
        bmp.extend(40u32.to_le_bytes());
        bmp.extend(w.to_le_bytes());
        bmp.extend(h.to_le_bytes());
        bmp.extend(1u16.to_le_bytes());
        bmp.extend(24u16.to_le_bytes());
        bmp.extend([0u8; 24]);
        for y in 0..h {
            for x in 0..row {
                bmp.push(if x % 3 == 0 { 200 } else { (y * 7) as u8 });
            }
        }
        std::fs::write(d.join("pic.bmp"), &bmp).unwrap();
        let out = d.join("thumbnail.png");
        shell_thumbnail(&d.join("pic.bmp"), &out, 48).unwrap();
        let png = std::fs::read(&out).unwrap();
        assert!(png.starts_with(b"\x89PNG"), "a PNG");
        let small = d.join("small.png");
        shrink_png(&out, &small, 16).unwrap();
        let p = std::fs::read(&small).unwrap();
        // IHDR width, big-endian, at byte 16.
        assert_eq!(u32::from_be_bytes(p[16..20].try_into().unwrap()), 16);
        assert!(shell_thumbnail(&d.join("missing.mp4"), &out, 48).is_err());
    }

    #[test]
    fn forward_only_over_desktop() {
        let r = |left, top, right, bottom| RECT {
            left,
            top,
            right,
            bottom,
        };
        let t = [(r(0, 0, 1920, 1080), 11), (r(1920, 0, 3840, 1080), 22)];
        assert_eq!(forward_target(100, 50, true, &t), Some((11, 100, 50)));
        assert_eq!(
            forward_target(2000, 70, true, &t),
            Some((22, 80, 70)),
            "second display, own coordinates"
        );
        assert_eq!(
            forward_target(100, 50, false, &t),
            None,
            "over an app window"
        );
        assert_eq!(
            forward_target(5000, 50, true, &t),
            None,
            "outside every wallpaper"
        );
        assert_eq!(
            forward_target(1920, 0, true, &t).map(|f| f.0),
            Some(22),
            "edges belong to the right"
        );
    }

    #[test]
    fn screensaver_running_reads_the_real_state() {
        // Tests run while someone (or CI) works, so no screensaver is on screen.
        assert!(!screensaver_running());
    }

    #[test]
    fn update_toast_has_buttons_and_escapes() {
        let x = update_toast_xml(
            "Sarab 0.0.6 is available",
            "Fixes & <more>",
            "Update now",
            "Later",
        );
        assert!(x.contains(r#"scenario="reminder""#), "stays until answered");
        assert!(x.contains(r#"launch="open""#));
        assert!(x.contains(r#"arguments="install""#) && x.contains(r#"arguments="later""#));
        assert!(x.contains("Fixes &amp; &lt;more&gt;"));
        let doc = windows::Data::Xml::Dom::XmlDocument::new().unwrap();
        doc.LoadXml(&HSTRING::from(x)).expect("well-formed XML");
    }

    #[test]
    fn app_wallpaper_ends_with_its_job() {
        use windows::Win32::System::Threading::{
            GetExitCodeProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };
        let ping = std::path::Path::new(r"C:\Windows\System32\PING.EXE");
        let p = launch_app(ping, &["-n", "60", "127.0.0.1"]).unwrap();
        let pid = p.pid;
        let alive = || unsafe {
            let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
                return false;
            };
            let mut code = 0;
            let _ = GetExitCodeProcess(h, &mut code);
            let _ = CloseHandle(h);
            code == 259 // STILL_ACTIVE
        };
        assert!(alive(), "running");
        drop(p);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!alive(), "closing the job ends the program");
        assert!(launch_app(std::path::Path::new("C:/no/such.exe"), &[]).is_err());
    }

    #[test]
    fn on_path_adds_once_and_removes_only_ours() {
        let dir = r"C:\Users\a\AppData\Local\Sarab";
        assert_eq!(path_with("", dir, true), dir);
        assert_eq!(
            path_with(r"%USERPROFILE%\bin;", dir, true),
            format!(r"%USERPROFILE%\bin;{dir};")
        );
        for orig in ["", r"C:\x", r"C:\x;", r"%USERPROFILE%\bin;;C:\y;"] {
            let back = path_with(&path_with(orig, dir, true), dir, false);
            assert_eq!(back, orig, "add then remove is exact");
        }
        let with = path_with(r"C:\x;;C:\y", dir, true);
        assert_eq!(with, format!(r"C:\x;;C:\y;{dir}"), "keeps empty entries");
        assert_eq!(path_with(&with, dir, true), with, "added once");
        assert_eq!(
            path_with(&with.to_uppercase(), dir, true),
            with.to_uppercase(),
            "case does not matter"
        );
        assert_eq!(
            path_with(&format!(r"{dir}\;C:\x"), dir, false),
            r"C:\x",
            "trailing slash still ours"
        );
        assert_eq!(path_with(&with, dir, false), r"C:\x;;C:\y");
        assert_eq!(
            path_with(r"C:\x;C:\Sarab2", r"C:\Sarab", false),
            r"C:\x;C:\Sarab2",
            "a longer name is not ours"
        );
        // The installer calls both, outside an update.
        let hooks = include_str!("../../windows/hooks.nsh");
        assert!(hooks.contains("--add-to-path") && hooks.contains("--remove-from-path"));
    }

    #[test]
    fn recycle_moves_to_bin() {
        // Leaves one small folder named sarab-test-recycle-<pid> in the Recycle Bin.
        let d = std::env::temp_dir().join(format!("sarab-test-recycle-{}", std::process::id()));
        std::fs::create_dir_all(d.join("inner")).unwrap();
        std::fs::write(d.join("inner").join("a.txt"), "x").unwrap();
        recycle(&d).unwrap();
        assert!(!d.exists());
        assert!(recycle(&d).is_err(), "nothing left to recycle");
    }
}
