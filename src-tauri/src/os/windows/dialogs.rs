//! The Windows folder picker.

use super::*;

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
