use crate::common::log;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::WebviewWindow;

pub fn released(path: &Path) -> bool {
    use std::os::windows::fs::OpenOptionsExt;
    for _ in 0..60 {
        if fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .is_ok()
        {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    false
}

pub fn capture_preview(
    win: &WebviewWindow,
    path: PathBuf,
    done: impl FnOnce(bool) + Send + 'static,
) {
    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    use windows::Win32::System::Com::{STGM_CREATE, STGM_WRITE};
    use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
    let done = std::sync::Arc::new(Mutex::new(Some(done)));
    let fail = done.clone();
    let started = win.with_webview(move |pw| unsafe {
        let run = || -> windows::core::Result<()> {
            let stream = SHCreateStreamOnFileEx(
                &windows::core::HSTRING::from(path.as_os_str()),
                (STGM_CREATE | STGM_WRITE).0,
                0x80,
                true,
                None,
            )?;
            let keep = stream.clone();
            let d = done.clone();
            let written = path.clone();
            let handler = CapturePreviewCompletedHandler::create(Box::new(move |r| {
                drop(keep);
                let ok = r.is_ok();
                if let Some(f) = d.lock().unwrap().take() {
                    let written = written.clone();
                    std::thread::spawn(move || f(ok && released(&written)));
                }
                Ok(())
            }));
            pw.controller().CoreWebView2()?.CapturePreview(
                COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                &stream,
                &handler,
            )
        };
        if let Err(e) = run() {
            log(format!("capture: {e}"));
            if let Some(f) = done.lock().unwrap().take() {
                f(false);
            }
        }
    });
    if started.is_err() {
        if let Some(f) = fail.lock().unwrap().take() {
            f(false);
        }
    }
}
