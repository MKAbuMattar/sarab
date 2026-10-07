use super::*;

pub struct AppProcess {
    job: isize,
    pub pid: u32,
    pub hwnd: Option<isize>,
    paused: bool,
}

impl AppProcess {
    pub fn set_paused(&mut self, paused: bool) -> Result<(), String> {
        if self.paused == paused {
            return Ok(());
        }
        let pids = self.pids()?;
        for_each_thread(&pids, |t| unsafe {
            use windows::Win32::System::Threading::{ResumeThread, SuspendThread};
            if paused {
                SuspendThread(t);
            } else {
                ResumeThread(t);
            }
        })?;
        self.paused = paused;
        Ok(())
    }

    pub fn pids(&self) -> Result<Vec<u32>, String> {
        job_pids(self.job)
    }
}

fn job_pids(job: isize) -> Result<Vec<u32>, String> {
    use windows::Win32::System::JobObjects::{
        JobObjectBasicProcessIdList, QueryInformationJobObject, JOBOBJECT_BASIC_PROCESS_ID_LIST,
    };
    const MAX: usize = 256;
    let size =
        std::mem::size_of::<JOBOBJECT_BASIC_PROCESS_ID_LIST>() + MAX * std::mem::size_of::<usize>();
    let mut buf = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
    let list = buf.as_mut_ptr() as *mut JOBOBJECT_BASIC_PROCESS_ID_LIST;
    unsafe {
        QueryInformationJobObject(
            Some(windows::Win32::Foundation::HANDLE(job as _)),
            JobObjectBasicProcessIdList,
            list as *mut _,
            (buf.len() * std::mem::size_of::<usize>()) as u32,
            None,
        )
        .map_err(|e| e.to_string())?;
        let n = (*list).NumberOfProcessIdsInList as usize;
        let ids = std::ptr::addr_of!((*list).ProcessIdList) as *const usize;
        Ok((0..n).map(|i| *ids.add(i) as u32).collect())
    }
}

pub fn for_each_thread(
    pids: &[u32],
    mut f: impl FnMut(windows::Win32::Foundation::HANDLE),
) -> Result<(), String> {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows::Win32::System::Threading::{OpenThread, THREAD_SUSPEND_RESUME};
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0).map_err(|e| e.to_string())?;
        let mut e = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        let mut more = Thread32First(snap, &mut e).is_ok();
        while more {
            if pids.contains(&e.th32OwnerProcessID) {
                if let Ok(t) = OpenThread(THREAD_SUSPEND_RESUME, false, e.th32ThreadID) {
                    f(t);
                    let _ = CloseHandle(t);
                }
            }
            more = Thread32Next(snap, &mut e).is_ok();
        }
        let _ = CloseHandle(snap);
    }
    Ok(())
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
    use std::os::windows::process::CommandExt;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::*;
    let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }.map_err(|e| e.to_string())?;
    let mut p = AppProcess {
        job: job.0 as isize,
        pid: 0,
        hwnd: None,
        paused: false,
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
        .creation_flags(windows::Win32::System::Threading::CREATE_SUSPENDED.0)
        .spawn()
        .map_err(|e| format!("{}: {e}", exe.display()))?;
    if let Err(e) = unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())) } {
        let _ = child.kill();
        return Err(e.to_string());
    }
    p.pid = child.id();
    for_each_thread(&[p.pid], |t| unsafe {
        windows::Win32::System::Threading::ResumeThread(t);
    })?;
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
