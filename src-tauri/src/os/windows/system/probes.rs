use super::*;

pub(in crate::os::windows) const DESKTOP_CLASSES: [&str; 5] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "SysListView32",
];

pub(in crate::os::windows) fn class_of(hwnd: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = unsafe { GetClassNameW(hwnd, &mut buf) } as usize;
    String::from_utf16_lossy(&buf[..n])
}

pub(in crate::os::windows) fn pid_of(hwnd: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

pub(in crate::os::windows) fn exe_name(pid: u32) -> Option<String> {
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

pub(in crate::os::windows) fn frame(hwnd: HWND) -> Option<RECT> {
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

pub(in crate::os::windows) fn covered(area: &RECT, wins: &[RECT]) -> bool {
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

pub(in crate::os::windows) fn app_windows() -> Vec<(HWND, RECT)> {
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
        locked: is_locked() || screensaver_running(),
        remote: unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0,
        foreground_app,
        desktop_focused,
        covered,
        cpu_busy: false,
    }
}

pub(in crate::os::windows) fn screensaver_running() -> bool {
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

pub(in crate::os::windows) fn is_locked() -> bool {
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
