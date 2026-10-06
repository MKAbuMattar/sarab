//! App wallpapers: a program whose window sits behind the icons.

use super::*;

/// Run a program as display `i`'s wallpaper. Its window appears when the program is ready, so a
/// thread waits for it (up to 20 s) and then moves it behind the icons.
// ponytail: app wallpapers keep running while others rest; suspending the process would cover
// Frozen and Covered too.
pub(super) fn start_app(
    app: &AppHandle,
    core: &mut Core,
    i: usize,
    w: &Wallpaper,
) -> Result<(), String> {
    core.desktop.ok_or("desktop layer not found")?;
    let Some(Target::File(exe)) = w.target() else {
        return Err("app wallpaper has no file".into());
    };
    if !exe.is_file() {
        return Err(format!("missing file {}", exe.display()));
    }
    let p = os::launch_app(&exe, &[])?;
    let pid = p.pid;
    core.displays[i].app = Some(p);
    let a = app.clone();
    std::thread::spawn(move || {
        for _ in 0..80 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            if let Some(hwnd) = os::main_window(pid) {
                let hwnd = hwnd.0 as isize;
                later(&a, move |_, core| attach_app(core, i, pid, hwnd));
                return;
            }
        }
        log(format!("app wallpaper {pid} showed no window in 20 s"));
    });
    Ok(())
}

pub(super) fn attach_app(core: &mut Core, i: usize, pid: u32, hwnd: isize) {
    // The display may have changed wallpaper while the program started.
    if core
        .displays
        .get(i)
        .and_then(|d| d.app.as_ref())
        .map(|p| p.pid)
        != Some(pid)
    {
        return;
    }
    let Some(desk) = core.desktop else {
        return;
    };
    let h = windows::Win32::Foundation::HWND(hwnd as _);
    match os::attach(&desk, h, wallpaper_rect(core, i)) {
        Ok(()) => {
            os::show(h, true);
            if let Some(p) = core.displays[i].app.as_mut() {
                p.hwnd = Some(hwnd);
            }
        }
        Err(e) => {
            log(format!("app wallpaper on display {i}: {e}"));
            core.displays[i].error = Some(e.to_string());
            core.displays[i].app = None;
        }
    }
}
