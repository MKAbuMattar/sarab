//! The Library page: which wallpaper runs on which display, and the wallpapers to choose from.

use super::{Model, act, props, snap, t};
use crate::engine::wallpaper;
use crate::engine::{DisplayView, PresetView};
use crate::model::cli::Command;
use crate::model::library::Wallpaper;
use crate::model::pause::State;
use gpui_kit::component::button::{Button, ButtonVariant, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::setting::{SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::component::{Selectable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

pub struct Library {
    target: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

fn kind_icon(kind: &str) -> IconName {
    match kind {
        "web" => IconName::Globe,
        "url" => IconName::ExternalLink,
        "video" => IconName::Play,
        "gif" | "picture" => IconName::Frame,
        _ => IconName::File,
    }
}

fn title_of(id: &str, cx: &App) -> String {
    let title = snap(cx)
        .library
        .iter()
        .find(|w| w.id == id)
        .and_then(|w| w.info.title.clone());
    super::rtl::visual(&title.unwrap_or_else(|| id.to_string()))
}

fn reason(d: &DisplayView, cx: &App) -> SharedString {
    let key = serde_json::to_value(d.reason)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    t(cx, &format!("reason.{key}"))
}

fn state_tag(state: State, cx: &App) -> Tag {
    let (tag, key) = match state {
        State::Play => (Tag::success(), "state.Play"),
        State::Frozen => (Tag::warning(), "state.Frozen"),
        State::Covered => (Tag::secondary(), "state.Covered"),
    };
    tag.small().child(t(cx, key))
}

/// Set a wallpaper (a library id, file, folder or URL) on the chosen display, or on all of them.
fn set_on_display(target: String, cx: &mut App) {
    let display = Model::get(cx).selected;
    act(cx, move |e| e.run_command(Command::Set { target, display }));
}

impl Library {
    pub fn new(window: &mut Window, cx: &mut Context<super::Main>) -> Self {
        let target = cx.new(|cx| InputState::new(window, cx).placeholder(t(cx, "add.placeholder")));
        let sub = cx.subscribe_in(&target, window, |this, _, ev, window, cx| {
            if let InputEvent::PressEnter { .. } = ev {
                this.library.add(window, cx);
            }
        });
        Library {
            target,
            _subscriptions: vec![sub],
        }
    }

    fn add(&self, window: &mut Window, cx: &mut App) {
        let value = self.target.read(cx).value().trim().to_string();
        if value.is_empty() {
            return;
        }
        self.target.update(cx, |s, cx| s.set_value("", window, cx));
        set_on_display(value, cx);
    }

    pub fn page(&self, _: &mut Window, cx: &mut Context<super::Main>) -> SettingPage {
        let m = Model::get(cx);
        let s = snap(cx);
        let target = self.target.clone();
        let add =
            Rc::new(cx.listener(|this, _: &ClickEvent, window, cx| this.library.add(window, cx)));
        let items: Vec<SettingItem> = if s.library.is_empty() {
            vec![SettingItem::render(|_, _, cx| {
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "library.empty"))
            })]
        } else {
            s.library.iter().cloned().map(wallpaper_item).collect()
        };
        SettingPage::new(m.t("library.title"))
            .title_suffix(|_, cx| super::pause_button(cx))
            .icon(IconName::GalleryVerticalEnd)
            .default_open(true)
            .groups([
                SettingGroup::new()
                    .title(m.t("displays.title"))
                    .item(SettingItem::render(|_, _, cx| displays(cx))),
                SettingGroup::new()
                    .title(m.t("add.button"))
                    .item(SettingItem::render(move |_, _, cx| {
                        let add = add.clone();
                        h_flex()
                            .w_full()
                            .gap_2()
                            .child(div().flex_1().child(Input::new(&target)))
                            .child(
                                Button::new("browse")
                                    .icon(IconName::FolderOpen)
                                    .label(t(cx, "add.browse"))
                                    .on_click(|_, _, cx| browse(cx)),
                            )
                            .child(
                                Button::new("add")
                                    .primary()
                                    .icon(IconName::Plus)
                                    .label(t(cx, "add.button"))
                                    .on_click(move |ev, window, cx| add(ev, window, cx)),
                            )
                    }))
                    .footer(|_, cx| t(cx, "add.hint")),
                SettingGroup::new().title(m.t("library.title")).items(items),
                SettingGroup::new()
                    .title(m.t("presets.title"))
                    .items(s.presets.iter().cloned().map(preset_item))
                    .footer(|_, cx| t(cx, "presets.hint")),
            ])
    }
}

/// The native file picker; the chosen file or folder joins the library and goes on the display.
fn browse(cx: &mut App) {
    let rx = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: None,
    });
    cx.spawn(async move |cx| {
        if let Ok(Ok(Some(paths))) = rx.await
            && let Some(p) = paths.into_iter().next()
        {
            cx.update(|cx| {
                let p = p.to_string_lossy().into_owned();
                if p.to_lowercase().ends_with(".zip") {
                    act(cx, move |e| e.run_command(Command::Import(p)));
                } else {
                    set_on_display(p, cx);
                }
            });
        }
    })
    .detach();
}

fn wallpaper_item(w: Wallpaper) -> SettingItem {
    let keywords = [w.info.title.clone().unwrap_or_default(), w.kind.to_string()];
    SettingItem::render(move |_, _, cx| {
        let m = Model::get(cx);
        let title = super::rtl::visual(&w.info.title.clone().unwrap_or_else(|| w.id.clone()));
        let kind = m.t(&format!("type.{}", w.kind));
        let caption: SharedString = if w.preset {
            format!("{kind} · {}", m.t("presets.builtIn")).into()
        } else {
            kind
        };
        let id = w.id.clone();
        h_flex()
            .w_full()
            .gap_3()
            .child(kind_icon(w.kind))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().truncate().child(title.clone()))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(caption),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("set-{}", w.id)))
                    .label(m.t("library.set"))
                    .on_click({
                        let id = id.clone();
                        move |_, _, cx| set_on_display(id.clone(), cx)
                    }),
            )
            .when(!w.preset, |row| {
                row.child(
                    Button::new(SharedString::from(format!("delete-{}", w.id)))
                        .ghost()
                        .icon(IconName::Delete)
                        .tooltip(m.t("library.delete"))
                        .on_click(move |_, window, cx| {
                            confirm_delete(id.clone(), title.clone(), window, cx)
                        }),
                )
            })
    })
    .keywords(keywords)
}

fn confirm_delete(id: String, title: String, window: &mut Window, cx: &mut App) {
    window.open_alert_dialog(cx, move |alert, _, cx| {
        let m = Model::get(cx);
        let id = id.clone();
        alert
            .title(m.tf("library.deleteTitle", &[("title", &title)]))
            .description(m.t("library.deleteBody"))
            .confirm()
            .ok_text(m.t("library.delete"))
            .ok_variant(ButtonVariant::Danger)
            .cancel_text(m.t("dialog.cancel"))
            .on_ok(move |_, _, cx| {
                let id = id.clone();
                act(cx, move |e| {
                    let r = wallpaper::remove(e, &id);
                    e.changed();
                    r
                });
                true
            })
    });
}

fn preset_item(p: PresetView) -> SettingItem {
    let keywords = [p.video.title.clone(), "4K".into()];
    SettingItem::render(move |_, _, cx| {
        let m = Model::get(cx);
        let v = &p.video;
        let mb = (v.size / 1_000_000).to_string();
        let id = v.id.clone();
        let action: AnyElement = if p.installed {
            Button::new(SharedString::from(format!("preset-set-{id}")))
                .label(m.t("library.set"))
                .on_click(move |_, _, cx| set_on_display(id.clone(), cx))
                .into_any_element()
        } else if let Some(pct) = p.progress {
            div()
                .text_sm()
                .child(m.tf("presets.downloading", &[("p", &pct.to_string())]))
                .into_any_element()
        } else {
            Button::new(SharedString::from(format!("preset-get-{id}")))
                .icon(IconName::ArrowDown)
                .label(m.tf("presets.get", &[("mb", &mb)]))
                .on_click(move |_, _, cx| {
                    let id = id.clone();
                    act(cx, move |e| e.run_command(Command::Preset(id)))
                })
                .into_any_element()
        };
        h_flex()
            .w_full()
            .gap_3()
            .child(IconName::Play)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(v.title.clone())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "{}×{} · {} fps · {mb} MB · {}",
                                v.width, v.height, v.fps, v.credit
                            )),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(v.description.clone()),
                    ),
            )
            .child(action)
    })
    .keywords(keywords)
}

/// The displays drawn to scale like Settings > System > Display, then what the chosen one is doing.
fn displays(cx: &App) -> AnyElement {
    let m = Model::get(cx);
    let s = snap(cx);
    let ds = &s.displays;
    let header = h_flex().w_full().gap_2().child(
        Button::new("all-displays")
            .icon(IconName::LayoutDashboard)
            .label(m.t("displays.all"))
            .selected(m.selected.is_none())
            .on_click(|_, _, cx| {
                cx.global_mut::<Model>().selected = None;
                cx.refresh_windows();
            }),
    );

    const W: f32 = 520.;
    const H: f32 = 150.;
    const GAP: f32 = 8.;
    let monitors = if ds.is_empty() {
        div().into_any_element()
    } else {
        let min_x = ds.iter().map(|d| d.x).min().unwrap_or(0);
        let min_y = ds.iter().map(|d| d.y).min().unwrap_or(0);
        let max_x = ds.iter().map(|d| d.x + d.width).max().unwrap_or(1);
        let max_y = ds.iter().map(|d| d.y + d.height).max().unwrap_or(1);
        let gaps = GAP * (ds.len() - 1) as f32;
        let scale = ((W - gaps) / (max_x - min_x) as f32).min(H / (max_y - min_y) as f32);
        let mut order: Vec<usize> = (0..ds.len()).collect();
        order.sort_by_key(|&i| ds[i].x);
        let box_w = (max_x - min_x) as f32 * scale + gaps;
        let box_h = (max_y - min_y) as f32 * scale;
        div()
            .relative()
            .mx_auto()
            .w(px(box_w))
            .h(px(box_h))
            .children(ds.iter().enumerate().map(|(i, d)| {
                let slot = order.iter().position(|&o| o == i).unwrap_or(0) as f32;
                let selected = m.selected == Some(i);
                let label = d
                    .wallpaper
                    .as_deref()
                    .map_or(m.t("displays.empty"), |id| title_of(id, cx).into());
                div()
                    .id(SharedString::from(format!("display-{}", d.key)))
                    .absolute()
                    .left(px((d.x - min_x) as f32 * scale + slot * GAP))
                    .top(px((d.y - min_y) as f32 * scale))
                    .w(px(d.width as f32 * scale))
                    .h(px(d.height as f32 * scale))
                    .p_2()
                    .rounded(cx.theme().radius)
                    .border_2()
                    .border_color(if selected {
                        cx.theme().primary
                    } else {
                        cx.theme().border
                    })
                    .bg(cx.theme().secondary)
                    .cursor_pointer()
                    .hover(|s| s.bg(cx.theme().secondary_hover))
                    .on_click(move |_, _, cx| {
                        let m = cx.global_mut::<Model>();
                        m.selected = if m.selected == Some(i) { None } else { Some(i) };
                        cx.refresh_windows();
                    })
                    .child(
                        v_flex()
                            .size_full()
                            .justify_between()
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(div().font_semibold().child((i + 1).to_string()))
                                    .when(d.wallpaper.is_some(), |r| {
                                        r.child(state_tag(d.state, cx))
                                    }),
                            )
                            .child(div().text_sm().truncate().child(label)),
                    )
            }))
            .into_any_element()
    };

    let idx = m.selected.or((ds.len() == 1).then_some(0));
    let detail = match idx.and_then(|i| ds.get(i).map(|d| (i, d))) {
        None => {
            let held = ds
                .iter()
                .find(|d| d.wallpaper.is_some() && d.state != State::Play);
            h_flex()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(held.map_or(m.t("detail.pickHint"), |d| reason(d, cx))),
                )
                .when(held.is_some(), |r| r.child(play_anyway(cx)))
                .when(s.manual == Some(false), |r| r.child(automatic(cx)))
        }
        Some((i, d)) => {
            let mut text = format!("{} · ", m.tf("displays.n", &[("n", &(i + 1).to_string())]));
            match &d.wallpaper {
                Some(id) => text += &format!("{} · {}", title_of(id, cx), reason(d, cx)),
                None => text += &m.t("displays.empty"),
            }
            if let Some(e) = &d.error {
                text += &format!(" · {}: {e}", m.t("error.prefix"));
            }
            let on = d.wallpaper.is_some();
            h_flex()
                .gap_2()
                .child(div().flex_1().text_sm().child(text))
                .when(on && d.state != State::Play, |r| r.child(play_anyway(cx)))
                .when(s.manual == Some(false), |r| r.child(automatic(cx)))
                .when(on, |r| {
                    r.child(
                        Button::new("customize")
                            .label(m.t("library.customize"))
                            .on_click(move |_, window, cx| props::open(i, window, cx)),
                    )
                    .child(
                        Button::new("close-display")
                            .ghost()
                            .label(m.t("displays.close"))
                            .on_click(move |_, _, cx| {
                                act(cx, move |e| {
                                    e.run_command(Command::Close { display: Some(i) })
                                })
                            }),
                    )
                })
        }
    };
    v_flex()
        .w_full()
        .gap_3()
        .child(header)
        .child(monitors)
        .child(detail)
        .into_any_element()
}

fn play_anyway(cx: &App) -> Button {
    Button::new("play-anyway")
        .icon(IconName::Play)
        .label(t(cx, "detail.playAnyway"))
        .on_click(|_, _, cx| act(cx, |e| e.run_command(Command::Play)))
}

fn automatic(cx: &App) -> Button {
    Button::new("automatic")
        .label(t(cx, "detail.auto"))
        .on_click(|_, _, cx| act(cx, |e| e.run_command(Command::Resume)))
}
