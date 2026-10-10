use super::*;

pub fn capture_preview(
    win: &tauri::WebviewWindow,
    path: PathBuf,
    done: impl FnOnce(bool) + Send + 'static,
) {
    use webkit2gtk::{SnapshotOptions, SnapshotRegion, WebViewExt};
    let done = std::sync::Arc::new(std::sync::Mutex::new(Some(done)));
    let finish = done.clone();
    let asked = win.with_webview(move |wv| {
        let finish = finish.clone();
        wv.inner().snapshot(
            SnapshotRegion::Visible,
            SnapshotOptions::NONE,
            None::<&gtk::gio::Cancellable>,
            move |shot| {
                let ok = shot.ok().is_some_and(|s| {
                    let Ok(img) = gtk::cairo::ImageSurface::try_from(s) else {
                        return false;
                    };
                    let (w, h) = (img.width(), img.height());
                    gdk::pixbuf_get_from_surface(&img, 0, 0, w, h)
                        .is_some_and(|p| p.savev(&path, "png", &[]).is_ok())
                });
                if let Some(f) = finish.lock().unwrap().take() {
                    f(ok);
                }
            },
        );
    });
    if asked.is_err() {
        if let Some(f) = done.lock().unwrap().take() {
            f(false);
        }
    }
}
