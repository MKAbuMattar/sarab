//! Run on the main thread, or after the current core lock is released.

use crate::engine::wallpaper::{Core, Shared};
use tauri::{AppHandle, Manager};

/// Run `f` on the main thread and wait for its result. Safe to call from the main thread too.
pub fn on_main<T: Send + 'static>(
    app: &AppHandle,
    f: impl FnOnce(&AppHandle) -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    let a = app.clone();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(&a));
    })
    .expect("event loop gone");
    rx.recv().expect("main thread task dropped")
}

/// Queue `f` behind whatever the main thread is doing now. WebView2 callbacks can fire inside
/// nested message loops while the core lock is held; going through the event loop avoids re-entry.
pub fn later(app: &AppHandle, f: impl FnOnce(&AppHandle, &mut Core) + Send + 'static) {
    let a = app.clone();
    std::thread::spawn(move || {
        let a2 = a.clone();
        let _ = a.run_on_main_thread(move || {
            let st = a2.state::<Shared>();
            let mut core = st.lock().unwrap();
            f(&a2, &mut core);
        });
    });
}
