//! Sarab's own process tree, CPU times, and whether a pid is Sarab.

use super::*;

/// Processes that belong to Sarab: this one and every process it started, at any depth
/// (WebView2's browser, renderers and its audio service).
pub(super) fn own_processes() -> std::collections::HashSet<u32> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    let mut parent = std::collections::HashMap::new();
    unsafe {
        if let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            let mut e = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut ok = Process32FirstW(snap, &mut e).is_ok();
            while ok {
                parent.insert(e.th32ProcessID, e.th32ParentProcessID);
                ok = Process32NextW(snap, &mut e).is_ok();
            }
            let _ = CloseHandle(snap);
        }
    }
    let me = unsafe { GetCurrentProcessId() };
    let mut own = std::collections::HashSet::from([me]);
    // A few passes reach grandchildren; a PID reused after its parent exited never links to us
    // because the chain stops at a PID that is not ours.
    for _ in 0..4 {
        for (&pid, &ppid) in &parent {
            if own.contains(&ppid) {
                own.insert(pid);
            }
        }
    }
    own
}

pub(super) fn filetime(f: windows::Win32::Foundation::FILETIME) -> u64 {
    (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime)
}

/// Idle and busy CPU time since boot, in 100 ns units, summed over every core.
pub fn cpu_times() -> (u64, u64) {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;
    let (mut idle, mut kernel, mut user) = (
        FILETIME::default(),
        FILETIME::default(),
        FILETIME::default(),
    );
    if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
        return (0, 0);
    }
    // Kernel time includes idle time.
    let idle = filetime(idle);
    (idle, filetime(kernel) + filetime(user) - idle)
}

/// CPU time used by Sarab and every process it started (WebView2 included), in 100 ns units.
pub fn own_cpu_time() -> u64 {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetProcessTimes;
    own_processes()
        .into_iter()
        .filter_map(|pid| unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let (mut c, mut e, mut k, mut u) = (
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
                FILETIME::default(),
            );
            let r = GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u);
            let _ = CloseHandle(h);
            r.ok().map(|_| filetime(k) + filetime(u))
        })
        .sum()
}

/// Is `pid` a running Sarab (and not some other program that got the same number later)?
pub fn is_sarab(pid: u32) -> bool {
    use windows::Win32::System::Threading::{QueryFullProcessImageNameW, PROCESS_NAME_WIN32};
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
                .is_ok();
        let _ = CloseHandle(h);
        ok && String::from_utf16_lossy(&buf[..len as usize])
            .to_ascii_lowercase()
            .ends_with("sarab.exe")
    }
}
