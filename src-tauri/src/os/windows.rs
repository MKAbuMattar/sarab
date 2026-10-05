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
