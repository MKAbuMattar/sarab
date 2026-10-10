use super::*;
use std::ffi::{c_int, c_long, c_uchar, c_uint, c_ulong, CString};
use x11_dl::xlib;

pub struct AppProcess {
    pub pid: u32,
    pub hwnd: Option<isize>,
    child: std::process::Child,
}

impl AppProcess {
    pub fn set_paused(&mut self, paused: bool) -> Result<(), String> {
        let signal = if paused { "-STOP" } else { "-CONT" };
        std::process::Command::new("kill")
            .args([signal, &self.pid.to_string()])
            .status()
            .map_err(|e| e.to_string())
            .and_then(|s| {
                s.success()
                    .then_some(())
                    .ok_or_else(|| format!("kill {signal} {} failed", self.pid))
            })
    }
}

impl Drop for AppProcess {
    fn drop(&mut self) {
        let _ = self.set_paused(false);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn launch_app(exe: &Path, args: &[&str]) -> Result<AppProcess, String> {
    if gdk::Display::default().is_some_and(|d| d.type_().name() == "GdkWaylandDisplay") {
        return Err("app wallpapers need an X11 session".into());
    }
    let child = std::process::Command::new(exe)
        .args(args)
        .env("GDK_BACKEND", "x11")
        .env("QT_QPA_PLATFORM", "xcb")
        .env("SDL_VIDEODRIVER", "x11")
        .env_remove("WAYLAND_DISPLAY")
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    Ok(AppProcess {
        pid: child.id(),
        hwnd: None,
        child,
    })
}

fn with_x<T>(f: impl FnOnce(&xlib::Xlib, *mut xlib::Display) -> T) -> Option<T> {
    let x = xlib::Xlib::open().ok()?;
    let d = unsafe { (x.XOpenDisplay)(std::ptr::null()) };
    if d.is_null() {
        return None;
    }
    let out = f(&x, d);
    unsafe { (x.XCloseDisplay)(d) };
    Some(out)
}

fn atom(x: &xlib::Xlib, d: *mut xlib::Display, name: &str) -> xlib::Atom {
    let name = CString::new(name).unwrap_or_default();
    unsafe { (x.XInternAtom)(d, name.as_ptr(), xlib::False) }
}

fn window_pid(
    x: &xlib::Xlib,
    d: *mut xlib::Display,
    w: xlib::Window,
    prop: xlib::Atom,
) -> Option<u32> {
    let (mut kind, mut format, mut n, mut after) = (0, 0, 0, 0);
    let mut data: *mut c_uchar = std::ptr::null_mut();
    let ok = unsafe {
        (x.XGetWindowProperty)(
            d,
            w,
            prop,
            0,
            1,
            xlib::False,
            xlib::XA_CARDINAL,
            &mut kind,
            &mut format,
            &mut n,
            &mut after,
            &mut data,
        )
    } == xlib::Success as c_int;
    if data.is_null() {
        return None;
    }
    let pid = (ok && format == 32 && n == 1).then(|| unsafe { *(data as *const c_ulong) } as u32);
    unsafe { (x.XFree)(data.cast()) };
    pid
}

fn find(
    x: &xlib::Xlib,
    d: *mut xlib::Display,
    w: xlib::Window,
    prop: xlib::Atom,
    pid: u32,
    depth: u8,
) -> Option<xlib::Window> {
    if window_pid(x, d, w, prop) == Some(pid) {
        let mut a: xlib::XWindowAttributes = unsafe { std::mem::zeroed() };
        if unsafe { (x.XGetWindowAttributes)(d, w, &mut a) } != 0 && a.map_state == xlib::IsViewable
        {
            return Some(w);
        }
    }
    if depth == 0 {
        return None;
    }
    let (mut root, mut parent) = (0, 0);
    let mut kids: *mut xlib::Window = std::ptr::null_mut();
    let mut n: c_uint = 0;
    if unsafe { (x.XQueryTree)(d, w, &mut root, &mut parent, &mut kids, &mut n) } == 0
        || kids.is_null()
    {
        return None;
    }
    let list = unsafe { std::slice::from_raw_parts(kids, n as usize) }.to_vec();
    unsafe { (x.XFree)(kids.cast()) };
    list.into_iter()
        .find_map(|k| find(x, d, k, prop, pid, depth - 1))
}

pub fn main_window(pid: u32) -> Option<HWND> {
    with_x(|x, d| {
        let prop = atom(x, d, "_NET_WM_PID");
        let root = unsafe { (x.XDefaultRootWindow)(d) };
        find(x, d, root, prop, pid, 3)
    })
    .flatten()
    .map(|w| HWND(w as usize as *mut _))
}

pub fn attach_app(_d: &Desktop, hwnd: HWND, r: RECT) -> Result<(), String> {
    let w = hwnd.0 as usize as xlib::Window;
    with_x(|x, d| unsafe {
        let set = |name: &str, values: &[&str]| {
            let atoms: Vec<c_long> = values.iter().map(|v| atom(x, d, v) as c_long).collect();
            (x.XChangeProperty)(
                d,
                w,
                atom(x, d, name),
                xlib::XA_ATOM,
                32,
                xlib::PropModeReplace,
                atoms.as_ptr().cast(),
                atoms.len() as c_int,
            );
        };
        (x.XUnmapWindow)(d, w);
        (x.XSync)(d, xlib::False);
        set("_NET_WM_WINDOW_TYPE", &["_NET_WM_WINDOW_TYPE_DESKTOP"]);
        set(
            "_NET_WM_STATE",
            &[
                "_NET_WM_STATE_BELOW",
                "_NET_WM_STATE_STICKY",
                "_NET_WM_STATE_SKIP_TASKBAR",
                "_NET_WM_STATE_SKIP_PAGER",
            ],
        );
        (x.XMapWindow)(d, w);
        (x.XMoveResizeWindow)(
            d,
            w,
            r.left,
            r.top,
            (r.right - r.left).max(1) as c_uint,
            (r.bottom - r.top).max(1) as c_uint,
        );
        (x.XLowerWindow)(d, w);
        (x.XFlush)(d);
    })
    .ok_or_else(|| "no X11 display".to_string())
}

pub fn show_app(hwnd: HWND, visible: bool) {
    let w = hwnd.0 as usize as xlib::Window;
    with_x(|x, d| unsafe {
        if visible {
            (x.XMapWindow)(d, w);
            (x.XLowerWindow)(d, w);
        } else {
            (x.XUnmapWindow)(d, w);
        }
        (x.XFlush)(d);
    });
}
