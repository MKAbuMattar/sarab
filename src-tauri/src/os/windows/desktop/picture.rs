//! The Windows picture wallpaper, per display (IDesktopWallpaper).

use super::*;

pub(in crate::os::windows) fn wallpaper_api() -> windows::core::Result<IDesktopWallpaper> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
    }
}

/// IDesktopWallpaper monitor ids for the given monitor rects.
pub(in crate::os::windows) fn monitor_id(
    api: &IDesktopWallpaper,
    m: &Monitor,
) -> windows::core::Result<PWSTR> {
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
