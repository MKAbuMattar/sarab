use super::*;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    pub raised: bool,
}

pub(in crate::os::windows) fn h(v: isize) -> HWND {
    HWND(v as *mut _)
}

pub(in crate::os::windows) fn raw(v: HWND) -> isize {
    v.0 as isize
}

pub fn find_desktop() -> Option<Desktop> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null()).ok()?;
        let raised =
            (GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32 & WS_EX_NOREDIRECTIONBITMAP.0) != 0;
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

pub fn attach(d: &Desktop, hwnd: HWND, r: RECT) -> windows::core::Result<()> {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let style = (style | WS_CHILD.0) & !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0);
        SetWindowLongPtrW(hwnd, GWL_STYLE, style as _);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            ((ex | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0) as _,
        );
        let parent = if d.raised {
            let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_LAYERED.0) as _);
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

pub(in crate::os::windows) fn children(parent: HWND) -> Vec<isize> {
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

pub fn refresh_desktop(d: Option<&Desktop>) {
    if d.is_some_and(|d| d.raised) {
        return;
    }
    unsafe {
        let _ = SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, None, SPIF_UPDATEINIFILE);
    }
}
