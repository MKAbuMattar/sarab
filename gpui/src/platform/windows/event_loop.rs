//! Wake-up for the engine thread's message loop.

use windows::Win32::Foundation::{HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateEventW, SetEvent};
use windows::Win32::UI::WindowsAndMessaging::*;

/// An auto-reset event. Nested message loops (WebView2 creation pumps its own) never consume it,
/// so a job queued meanwhile is still seen by the engine's own loop.
#[derive(Clone, Copy)]
pub struct Waker(isize);

impl Waker {
    pub fn new() -> Waker {
        let h = unsafe { CreateEventW(None, false, false, None) }.expect("CreateEvent");
        Waker(h.0 as isize)
    }
    pub fn wake(&self) {
        unsafe {
            let _ = SetEvent(HANDLE(self.0 as *mut _));
        }
    }
    /// Wait until woken, a window message arrives, or `ms` pass. Then dispatch pending messages.
    /// Returns true when woken.
    pub fn wait(&self, ms: u32) -> bool {
        let h = [HANDLE(self.0 as *mut _)];
        let r =
            unsafe { MsgWaitForMultipleObjectsEx(Some(&h), ms, QS_ALLINPUT, MWMO_INPUTAVAILABLE) };
        let mut msg = MSG::default();
        unsafe {
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        r == WAIT_OBJECT_0
    }
}
