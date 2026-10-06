//! Delete to the Recycle Bin.

use super::*;

/// Move a file or folder to the Recycle Bin, so a deleted wallpaper can be restored.
pub fn recycle(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::UI::Shell::{
        SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE,
        SHFILEOPSTRUCTW,
    };
    // The API rejects the \\?\ form that fs::canonicalize returns, so pass the plain path.
    let plain = path.to_string_lossy();
    let plain = match plain.strip_prefix(r"\\?\UNC\") {
        Some(rest) => format!(r"\\{rest}"),
        None => plain.strip_prefix(r"\\?\").unwrap_or(&plain).to_string(),
    };
    // The API takes a list ending in two NULs.
    let from: Vec<u16> = std::ffi::OsStr::new(&plain)
        .encode_wide()
        .chain([0, 0])
        .collect();
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
