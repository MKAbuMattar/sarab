//! Wallpaper window labels, and finding a display's window.

use super::*;

pub(super) fn new_label(i: usize) -> String {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    format!(
        "wp-{i}-{}",
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

pub(super) fn window(app: &AppHandle, d: &Display) -> Option<WebviewWindow> {
    app.get_webview_window(d.label.as_deref()?)
}
