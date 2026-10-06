use super::*;

pub fn path_with(path: &str, dir: &str, add: bool) -> String {
    let same = |p: &str| {
        p.trim()
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(dir.trim_end_matches('\\'))
    };
    let present = path.split(';').any(same);
    if add == present {
        path.to_string()
    } else if add {
        if path.is_empty() {
            dir.to_string()
        } else if path.ends_with(';') {
            format!("{path}{dir};")
        } else {
            format!("{path};{dir}")
        }
    } else {
        path.split(';')
            .filter(|p| !same(p))
            .collect::<Vec<_>>()
            .join(";")
    }
}

pub fn set_on_path(add: bool) -> Result<(), String> {
    use windows::Win32::System::Registry::*;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or("no folder")?
        .to_string_lossy()
        .into_owned();
    let mut key = HKEY::default();
    unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Environment"),
            None,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            &mut key,
        )
    }
    .ok()
    .map_err(|e| e.to_string())?;
    let mut len = 0u32;
    let flags = RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND;
    let found =
        unsafe { RegGetValueW(key, None, w!("Path"), flags, None, None, Some(&mut len)) }.is_ok();
    let mut buf = vec![0u16; (len as usize / 2).max(1)];
    if found {
        unsafe {
            RegGetValueW(
                key,
                None,
                w!("Path"),
                flags,
                None,
                Some(buf.as_mut_ptr() as *mut _),
                Some(&mut len),
            )
        }
        .ok()
        .map_err(|e| e.to_string())?;
    }
    let cur = String::from_utf16_lossy(&buf[..(len as usize / 2).saturating_sub(1)]);
    let cur = if found { cur } else { String::new() };
    let next = path_with(&cur, &dir, add);
    let r = if next == cur {
        Ok(())
    } else {
        let wide: Vec<u16> = next.encode_utf16().chain([0]).collect();
        let bytes =
            unsafe { std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2) };
        unsafe { RegSetValueExW(key, w!("Path"), None, REG_EXPAND_SZ, Some(bytes)) }
            .ok()
            .map_err(|e| e.to_string())
    };
    unsafe {
        let _ = RegCloseKey(key);
    }
    r?;
    let twin = exe.with_extension("com");
    if add {
        let bytes = std::fs::read(&exe).map_err(|e| e.to_string())?;
        std::fs::write(&twin, console_twin(bytes)?)
            .map_err(|e| format!("{}: {e}", twin.display()))?;
    } else {
        let _ = std::fs::remove_file(&twin);
    }
    unsafe {
        let _ = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(w!("Environment").as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            5000,
            None,
        );
    }
    Ok(())
}

pub fn console_twin(mut exe: Vec<u8>) -> Result<Vec<u8>, String> {
    let at = subsystem_at(&exe).ok_or("not a Windows program")?;
    exe[at..at + 2].copy_from_slice(&3u16.to_le_bytes());
    Ok(exe)
}

pub(in crate::os::windows) fn subsystem_at(exe: &[u8]) -> Option<usize> {
    let pe = u32::from_le_bytes(exe.get(0x3C..0x40)?.try_into().ok()?) as usize;
    (exe.get(pe..pe + 4)? == b"PE\0\0").then_some(())?;
    let at = pe + 24 + 68;
    exe.get(at..at + 2).map(|_| at)
}

pub fn tell_terminal(msg: &str) {
    use std::io::Write;
    use windows::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
    if unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok() {
        let _ = writeln!(std::io::stderr(), "\n{msg}");
    }
}
