use crate::engine::wallpaper::{Core, Shared};
use tauri::{AppHandle, Manager};

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
