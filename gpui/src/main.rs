#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod model;
mod platform;
mod ui;

use engine::ToUi;
use futures::StreamExt;
use gpui_kit::*;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // A second `sarab ...` hands its arguments to the running Sarab and exits.
    let server = loop {
        if platform::instance::send_to_running(&args) {
            return;
        }
        // Lost a race with a Sarab starting at the same moment: hand over to it instead.
        if let Some(server) = platform::instance::claim() {
            break server;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    // reqwest is built without a default TLS provider; install one for the whole app.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let (to_ui, mut from_engine) = futures::channel::mpsc::unbounded::<ToUi>();
    let engine = engine::start(args, to_ui);
    let h = engine.clone();
    server.serve(move |args| h.run(move |e| e.handle_args(&args)));

    gpui_kit::application()
        .with_assets(ui::AppAssets)
        // Closing the window leaves Sarab in the tray; only Quit ends it.
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            gpui_kit::init(cx);
            ui::init(engine, cx);
            cx.spawn(async move |cx| {
                while let Some(msg) = from_engine.next().await {
                    match msg {
                        ToUi::State(snap) => cx.update(|cx| ui::set_state(snap, cx)),
                        ToUi::Open => cx.update(ui::open),
                        ToUi::Quit => {
                            cx.update(|cx| cx.quit());
                            break;
                        }
                    }
                }
            })
            .detach();
        });
}
