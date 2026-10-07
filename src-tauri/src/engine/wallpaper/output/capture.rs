use super::*;

pub const THUMBNAIL: &str = "thumbnail.png";

pub static APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

pub(in crate::engine::wallpaper) static FAILED: Mutex<std::collections::BTreeSet<PathBuf>> =
    Mutex::new(std::collections::BTreeSet::new());

static CAPTURING: Mutex<std::collections::BTreeSet<PathBuf>> =
    Mutex::new(std::collections::BTreeSet::new());

pub(in crate::engine::wallpaper) fn claim_capture(dir: &Path) -> bool {
    CAPTURING.lock().unwrap().insert(dir.to_path_buf())
}

pub(in crate::engine::wallpaper) fn release_capture(dir: &Path) {
    CAPTURING.lock().unwrap().remove(dir);
}

pub fn make_thumbnails(lib: &[Wallpaper]) {
    use std::sync::atomic::{AtomicBool, Ordering};
    static RUNNING: AtomicBool = AtomicBool::new(false);
    let Some(app) = APP.get() else { return };
    let failed = FAILED.lock().unwrap().clone();
    let todo: Vec<(PathBuf, PathBuf)> = lib
        .iter()
        .filter(|w| !w.preset && w.thumb.is_none() && !failed.contains(&w.dir))
        .filter(|w| matches!(w.info.r#type, Kind::Video | Kind::Gif | Kind::Picture))
        .filter_map(|w| match w.target() {
            Some(Target::File(f)) if f.is_file() => Some((f, w.dir.clone())),
            _ => None,
        })
        .collect();
    if todo.is_empty() || RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        for (src, dir) in todo {
            match os::shell_thumbnail(&src, &dir.join(THUMBNAIL), 480)
                .and_then(|()| library::set_thumbnail(&dir, THUMBNAIL))
            {
                Ok(()) => log(format!("thumbnail made for {}", dir.display())),
                Err(e) => {
                    log(e);
                    FAILED.lock().unwrap().insert(dir);
                }
            }
        }
        RUNNING.store(false, Ordering::SeqCst);
        later(&app, |app, core| {
            crate::rescan(core);
            let _ = app.emit_to("main", "changed", ());
        });
    });
}

pub(in crate::engine::wallpaper) fn released(path: &Path) -> bool {
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

pub fn capture_png(win: &WebviewWindow, path: PathBuf, done: impl FnOnce(bool) + Send + 'static) {
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

pub(in crate::engine::wallpaper) fn capture_thumbnail(app: &AppHandle, core: &Core, lbl: &str) {
    let Some(d) = core
        .displays
        .iter()
        .find(|d| d.label.as_deref() == Some(lbl))
    else {
        return;
    };
    let Some(w) = d.wallpaper.as_deref().and_then(|id| core.find(id)).cloned() else {
        return;
    };
    if w.preset || w.thumb.is_some() || !matches!(w.info.r#type, Kind::Web | Kind::Url) {
        return;
    }
    let Some(win) = app.get_webview_window(lbl) else {
        return;
    };
    if !claim_capture(&w.dir) {
        return;
    }
    let full = std::env::temp_dir().join(format!("sarab-capture-{}-{lbl}.png", std::process::id()));
    let (a, done) = (app.clone(), full.clone());
    capture_png(&win, full, move |ok| {
        if !ok {
            release_capture(&w.dir);
            return;
        }
        std::thread::spawn(move || {
            let r = os::shrink_png(&done, &w.dir.join(THUMBNAIL), 480)
                .and_then(|()| library::set_thumbnail(&w.dir, THUMBNAIL));
            let _ = fs::remove_file(&done);
            release_capture(&w.dir);
            match r {
                Ok(()) => later(&a, |app, core| {
                    crate::rescan(core);
                    let _ = app.emit_to("main", "changed", ());
                }),
                Err(e) => log(format!("thumbnail capture: {e}")),
            }
        });
    });
}

pub(in crate::engine::wallpaper) fn frame_path(key: &str) -> PathBuf {
    cfg("frames").join(format!("{}.png", key_file(key)))
}

pub fn keep_frames_then_exit(app: &AppHandle, core: &Core) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let shots: Vec<(WebviewWindow, os::Monitor, PathBuf)> = core
        .displays
        .iter()
        .filter_map(|d| Some((window(app, d)?, d.mon.clone(), frame_path(&d.mon.key))))
        .collect();
    if shots.is_empty() {
        app.exit(0);
        return;
    }
    let _ = fs::create_dir_all(cfg("frames"));
    let left = std::sync::Arc::new(AtomicUsize::new(shots.len()));
    for (win, mon, path) in shots {
        let (a, left, file) = (app.clone(), left.clone(), path.clone());
        capture_png(&win, path, move |ok| {
            if ok {
                if let Err(e) = os::set_picture(&mon, &file.to_string_lossy()) {
                    log(format!("keep last frame on {}: {e}", mon.key));
                }
            }
            if left.fetch_sub(1, Ordering::SeqCst) == 1 {
                a.exit(0);
            }
        });
    }
    let a = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(4));
        a.exit(0);
    });
}

pub fn screenshot(app: &AppHandle, core: &Core, i: usize, path: PathBuf) -> Result<(), String> {
    let d = core.displays.get(i).ok_or("no such display")?;
    let win = window(app, d).ok_or("that display shows no web, video or GIF wallpaper")?;
    let shown = path.display().to_string();
    capture_png(&win, path, move |ok| {
        log(if ok {
            format!("screenshot saved to {shown}")
        } else {
            format!("screenshot to {shown} failed")
        });
    });
    Ok(())
}
