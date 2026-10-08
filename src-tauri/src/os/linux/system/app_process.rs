use super::*;

pub struct AppProcess {
    pub pid: u32,
    pub hwnd: Option<isize>,
}

impl AppProcess {
    pub fn set_paused(&mut self, _paused: bool) -> Result<(), String> {
        Err(NOT_YET.into())
    }
}

pub fn launch_app(_exe: &Path, _args: &[&str]) -> Result<AppProcess, String> {
    Err("app wallpapers run on Windows only".into())
}

pub fn main_window(_pid: u32) -> Option<HWND> {
    None
}
