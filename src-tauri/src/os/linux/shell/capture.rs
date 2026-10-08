use super::*;

pub fn capture_preview(
    _win: &tauri::WebviewWindow,
    _path: PathBuf,
    done: impl FnOnce(bool) + Send + 'static,
) {
    done(false);
}
