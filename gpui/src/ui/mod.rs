//! The settings window: Library, Settings and About pages in gpui-kit's Settings component.
//! It only shows the engine's last snapshot and sends requests; it never owns wallpaper state.

mod about;
pub mod i18n;
mod library;
mod look;
mod preferences;
mod props;
pub mod rtl;

use crate::engine::{Engine, Handle, Snapshot};
use crate::model::settings::Settings;
use gpui_kit::component::button::Button;
use gpui_kit::component::setting::Settings as SettingsPanel;
use gpui_kit::component::slider::{SliderEvent, SliderState};
use gpui_kit::component::{ActiveTheme as _, IconName, alert::Alert, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use i18n::Strings;
use std::sync::Arc;

/// gpui-kit's icons plus Sarab's own logo.
pub struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        match path {
            "sarab/logo.png" => Ok(Some(
                include_bytes!("../../assets/logo.png").as_slice().into(),
            )),
            _ => gpui_kit::assets::Assets.load(path),
        }
    }
    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        gpui_kit::assets::Assets.list(path)
    }
}

/// Application state the window renders from.
pub struct Model {
    pub engine: Handle,
    pub snap: Option<Arc<Snapshot>>,
    pub strings: Strings,
    pub window: Option<AnyWindowHandle>,
    /// Display the Library page acts on; None means every display.
    pub selected: Option<usize>,
    pub error: Option<String>,
    /// "Later" on the update banner hides it until the window opens again.
    pub update_later: bool,
}

impl Global for Model {}

impl Model {
    pub fn get(cx: &App) -> &Model {
        cx.global::<Model>()
    }
    pub fn t(&self, key: &str) -> SharedString {
        rtl::visual(self.strings.get(key)).into()
    }
    pub fn tf(&self, key: &str, vars: &[(&str, &str)]) -> SharedString {
        rtl::visual(&self.strings.fmt(key, vars)).into()
    }
}

/// The current snapshot. The window opens only after the first one arrives.
pub fn snap(cx: &App) -> Arc<Snapshot> {
    Model::get(cx).snap.clone().expect("snapshot")
}

pub fn t(cx: &App, key: &str) -> SharedString {
    Model::get(cx).t(key)
}

/// Run `f` on the engine; a failure shows in the window's error banner.
pub fn act(cx: &mut App, f: impl FnOnce(&mut Engine) -> Result<(), String> + Send + 'static) {
    let rx = Model::get(cx).engine.ask(f);
    cx.spawn(async move |cx| {
        let err = rx.await.ok().and_then(Result::err);
        cx.update(|cx| {
            cx.global_mut::<Model>().error = err;
            cx.refresh_windows();
        });
    })
    .detach();
}

/// Change one setting: shown at once, saved by the engine.
pub fn save(cx: &mut App, change: impl FnOnce(&mut Settings)) {
    let m = cx.global_mut::<Model>();
    let Some(snap) = &m.snap else { return };
    let mut next = (**snap).clone();
    let before = next.settings.clone();
    change(&mut next.settings);
    let new = next.settings.clone();
    m.snap = Some(Arc::new(next));
    if new.language != before.language {
        m.strings = Strings::load(&new.language);
    }
    if new.theme != before.theme || new.backdrop != before.backdrop {
        cx.defer(|cx| look::apply(None, cx));
    }
    act(cx, move |e| e.save_settings(new));
    cx.refresh_windows();
}

pub fn init(engine: Handle, cx: &mut App) {
    cx.set_global(Model {
        engine,
        snap: None,
        strings: Strings::load("en"),
        window: None,
        selected: None,
        error: None,
        update_later: false,
    });
}

pub fn set_state(snap: Arc<Snapshot>, cx: &mut App) {
    let m = cx.global_mut::<Model>();
    if m.snap
        .as_ref()
        .is_none_or(|s| s.settings.language != snap.settings.language)
    {
        m.strings = Strings::load(&snap.settings.language);
    }
    if m.selected.is_some_and(|i| i >= snap.displays.len()) {
        m.selected = None;
    }
    m.snap = Some(snap);
    cx.refresh_windows();
}

pub fn open(cx: &mut App) {
    if Model::get(cx).snap.is_none() {
        return;
    }
    if let Some(w) = Model::get(cx).window
        && w.update(cx, |_, window, _| window.activate_window())
            .is_ok()
    {
        return;
    }
    cx.global_mut::<Model>().update_later = false;
    let options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("Sarab".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::centered(size(px(1060.), px(720.)), cx)),
        window_min_size: Some(size(px(760.), px(500.))),
        window_background: look::backdrop(&snap(cx).settings.backdrop),
        app_id: Some("com.mkabumattar.sarab".into()),
        ..Default::default()
    };
    let opened =
        gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| Main::new(window, cx)));
    match opened {
        Ok((w, _)) => {
            cx.global_mut::<Model>().window = Some(w);
            look::apply(None, cx);
            cx.activate(true);
        }
        Err(e) => crate::engine::wallpaper::log(format!("open ui: {e}")),
    }
}

struct Main {
    library: library::Library,
    volume: Entity<SliderState>,
    _subscriptions: Vec<Subscription>,
}

impl Main {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subs = vec![
            // "Use system setting" follows Windows switching between light and dark while open.
            cx.observe_window_appearance(window, |_, window, cx| {
                if snap(cx).settings.theme == "system" {
                    look::apply(Some(window), cx);
                }
            }),
            cx.on_release(|_, cx| cx.global_mut::<Model>().window = None),
        ];
        let mut subs = subs;
        let start = snap(cx).settings.volume as f32;
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(start)
        });
        // Saved when the thumb is let go, so a drag is one save, not a hundred.
        subs.push(cx.subscribe(&volume, |_, _, ev: &SliderEvent, cx| {
            if let SliderEvent::Release(v) = ev {
                let v = v.start().round().clamp(0., 100.) as u8;
                save(cx, |s| s.volume = v);
            }
        }));
        Self {
            library: library::Library::new(window, cx),
            volume,
            _subscriptions: subs,
        }
    }
}

impl Render for Main {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let m = Model::get(cx);
        let err = m.error.clone();
        let banner = about::update_banner(cx);
        let pages = vec![
            self.library.page(window, cx),
            preferences::page(self.volume.clone(), cx),
            about::page(cx),
        ];
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            // Files dropped on the window join the library; a .zip is imported as a package.
            .on_drop(|paths: &ExternalPaths, _, cx| {
                let paths: Vec<String> = paths
                    .paths()
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                act(cx, move |e| {
                    for p in paths {
                        let r = if p.to_lowercase().ends_with(".zip") {
                            e.import(&p)
                        } else {
                            e.resolve(&p).map(|_| ())
                        };
                        if let Err(err) = r {
                            e.changed();
                            return Err(format!("{p}: {err}"));
                        }
                    }
                    e.changed();
                    Ok(())
                })
            })
            .children(banner)
            .when_some(err, |this, err| {
                this.child(h_flex().p_2().child(
                    Alert::error("error", format!("{}: {err}", t(cx, "error.prefix"))).on_close(
                        |_, _, cx| {
                            cx.global_mut::<Model>().error = None;
                            cx.refresh_windows();
                        },
                    ),
                ))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(SettingsPanel::new("sarab").pages(pages)),
            )
    }
}

/// Pause or resume every wallpaper; in each page header, as the web window had it in the sidebar.
pub fn pause_button(cx: &App) -> Button {
    let paused = snap(cx).manual == Some(true);
    Button::new("pause")
        .icon(if paused {
            IconName::Play
        } else {
            IconName::Pause
        })
        .label(if paused {
            t(cx, "pause.resume")
        } else {
            t(cx, "pause.button")
        })
        .on_click(|_, _, cx| act(cx, |e| e.run_command(crate::model::cli::Command::Toggle)))
}
