//! The plain windows that host wallpaper webviews, and the folders each webview may read.

use std::num::NonZeroIsize;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, w};
use wry::raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // Never take focus: a click on the desktop must not activate the wallpaper.
    if msg == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

/// A hidden, borderless tool window. `os::attach` later reparents it under the desktop icons.
pub fn create_host() -> windows::core::Result<HWND> {
    unsafe {
        let inst = GetModuleHandleW(None)?;
        let class = w!("SarabWallpaperHost");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(host_proc),
            hInstance: inst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        // Fails harmlessly with "class already exists" after the first window.
        RegisterClassW(&wc);
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class,
            w!("sarab-wallpaper"),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(inst.into()),
            None,
        )
    }
}

pub fn destroy(hwnd: HWND) {
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
}

/// Lets wry build a webview inside a window it did not create.
pub struct Host(pub HWND);

impl HasWindowHandle for Host {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let h = NonZeroIsize::new(self.0.0 as isize).ok_or(HandleError::Unavailable)?;
        let raw = RawWindowHandle::Win32(Win32WindowHandle::new(h));
        // SAFETY: the HWND outlives the webview; the engine destroys the webview first.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

/// Serve a folder to one webview at `https://{host}/`. WebView2 reads the files itself, with
/// range requests, so 4K video streams straight from disk without passing through Rust.
pub fn map_folder(
    wv: &wry::WebView,
    host: &str,
    dir: &std::path::Path,
) -> windows::core::Result<()> {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW, ICoreWebView2_3,
    };
    use windows::core::Interface;
    use wry::WebViewExtWindows;
    let core = wv.webview().cast::<ICoreWebView2_3>()?;
    unsafe {
        core.SetVirtualHostNameToFolderMapping(
            &HSTRING::from(host),
            &HSTRING::from(dir.as_os_str()),
            COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_ALLOW,
        )
    }
}
