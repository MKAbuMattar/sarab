//! Single instance: a later `sarab ...` hands its arguments to the running Sarab over a named pipe.

use std::io::Write;
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Storage::FileSystem::{
    FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_INBOUND, ReadFile,
};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_WAIT,
};
use windows::core::HSTRING;

const PIPE: &str = r"\\.\pipe\com.mkabumattar.sarab";

/// Hand `args` to a Sarab that is already running. Returns false when none is.
pub fn send_to_running(args: &[String]) -> bool {
    let Ok(mut pipe) = std::fs::OpenOptions::new().write(true).open(PIPE) else {
        return false;
    };
    let body = serde_json::to_vec(args).unwrap_or_default();
    pipe.write_all(&body).is_ok()
}

/// The pipe, claimed by this process: it is the running Sarab.
pub struct Server(isize);

/// Claim the pipe. None when another Sarab owns it (it may have started at the same moment).
pub fn claim() -> Option<Server> {
    let pipe = unsafe {
        CreateNamedPipeW(
            &HSTRING::from(PIPE),
            PIPE_ACCESS_INBOUND | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            1,
            0,
            64 * 1024,
            0,
            None,
        )
    };
    (!pipe.is_invalid()).then_some(Server(pipe.0 as isize))
}

impl Server {
    /// Every later `sarab ...` hands its arguments to `on_args`, one client at a time.
    pub fn serve(self, on_args: impl Fn(Vec<String>) + Send + 'static) {
        std::thread::spawn(move || {
            let pipe = HANDLE(self.0 as *mut _);
            loop {
                let mut body = vec![];
                let mut buf = [0u8; 4096];
                unsafe {
                    let _ = ConnectNamedPipe(pipe, None);
                    loop {
                        let mut n = 0;
                        if ReadFile(pipe, Some(&mut buf), Some(&mut n), None).is_err() || n == 0 {
                            break;
                        }
                        body.extend_from_slice(&buf[..n as usize]);
                    }
                    let _ = DisconnectNamedPipe(pipe);
                }
                if let Ok(args) = serde_json::from_slice::<Vec<String>>(&body) {
                    on_args(args);
                }
            }
        });
    }
}
