use super::*;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Desktop {
    pub progman: isize,
    pub workerw: isize,
    pub defview: isize,
    pub raised: bool,
}

pub fn find_desktop() -> Option<Desktop> {
    None
}

pub fn desktop_alive(_d: &Desktop) -> bool {
    false
}

pub fn attach(_d: &Desktop, _hwnd: HWND, _r: RECT) -> Result<(), String> {
    Err(NOT_YET.into())
}

pub fn ensure_order(_d: &Desktop, _wins: &[HWND]) -> bool {
    false
}

pub fn show(_hwnd: HWND, _visible: bool) {}

pub fn refresh_desktop(_d: Option<&Desktop>) {}
