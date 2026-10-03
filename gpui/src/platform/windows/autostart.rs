//! Start with Windows: the HKCU Run value.

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::{PCWSTR, w};

const RUN: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
/// The same value the installer's uninstall hook removes.
const RUN_VALUE: PCWSTR = w!("Sarab");

fn run_key(access: windows::Win32::System::Registry::REG_SAM_FLAGS) -> Option<HKEY> {
    let mut key = HKEY::default();
    unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN, None, access, &mut key) }
        .is_ok()
        .then_some(key)
}

pub fn enabled() -> bool {
    let Some(key) = run_key(KEY_READ) else {
        return false;
    };
    let found = unsafe { RegQueryValueExW(key, RUN_VALUE, None, None, None, None) }.is_ok();
    unsafe {
        let _ = RegCloseKey(key);
    }
    found
}

pub fn set(enable: bool, flag: &str) -> Result<(), String> {
    let key = run_key(KEY_SET_VALUE).ok_or("cannot open the Run key")?;
    let r = if enable {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let cmd = format!("\"{}\" {flag}", exe.display());
        let wide: Vec<u16> = cmd.encode_utf16().chain([0]).collect();
        let bytes =
            unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        unsafe { RegSetValueExW(key, RUN_VALUE, None, REG_SZ, Some(bytes)) }
    } else {
        match unsafe { RegDeleteValueW(key, RUN_VALUE) } {
            e if e == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND => Default::default(),
            e => e,
        }
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    r.ok().map_err(|e| e.to_string())
}
