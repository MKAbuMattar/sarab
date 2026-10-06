use super::*;

pub struct AppProcess {
    job: isize,
    pub pid: u32,
    pub hwnd: Option<isize>,
}

impl Drop for AppProcess {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(windows::Win32::Foundation::HANDLE(self.job as _));
        }
    }
}

pub fn launch_app(exe: &std::path::Path, args: &[&str]) -> Result<AppProcess, String> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::*;
    let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let mut p = AppProcess {
        job: job.0 as isize,
        pid: 0,
        hwnd: None,
    };
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    unsafe {
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const _,
            std::mem::size_of_val(&limits) as u32,
        )
    }
    .map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .current_dir(exe.parent().unwrap_or(std::path::Path::new(".")))
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    if let Err(e) = unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())) } {
        let _ = child.kill();
        return Err(e.to_string());
    }
    p.pid = child.id();
    Ok(p)
}

pub fn main_window(pid: u32) -> Option<HWND> {
    unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
        let f = &mut *(l.0 as *mut (u32, Option<HWND>));
        let mut p = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut p));
        if p == f.0
            && IsWindowVisible(hwnd).as_bool()
            && GetWindow(hwnd, GW_OWNER).map_or(true, |o| o.is_invalid())
        {
            f.1 = Some(hwnd);
            return false.into();
        }
        true.into()
    }
    let mut f: (u32, Option<HWND>) = (pid, None);
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(&mut f as *mut _ as isize));
    }
    f.1
}
