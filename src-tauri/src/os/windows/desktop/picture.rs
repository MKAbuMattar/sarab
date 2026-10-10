use super::*;

pub(in crate::os::windows) fn wallpaper_api() -> windows::core::Result<IDesktopWallpaper> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)
    }
}

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

pub fn set_lock_screen(path: &std::path::Path) -> Result<(), String> {
    use windows::Storage::StorageFile;
    use windows::System::UserProfile::LockScreen;
    (|| -> windows::core::Result<()> {
        let file = StorageFile::GetFileFromPathAsync(&HSTRING::from(path.as_os_str()))?.join()?;
        LockScreen::SetImageFileAsync(&file)?.join()
    })()
    .map_err(|e| format!("lock screen: {e}"))
}
