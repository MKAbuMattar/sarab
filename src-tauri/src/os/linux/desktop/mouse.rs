use super::*;

pub type Targets = std::sync::Arc<std::sync::Mutex<Vec<(RECT, isize)>>>;

pub fn input_window(_top: HWND) -> Option<isize> {
    None
}

pub fn window_rect(_h: HWND) -> Option<RECT> {
    None
}

pub fn forward_mouse(_targets: Targets, _running: std::sync::Arc<std::sync::atomic::AtomicBool>) {}
