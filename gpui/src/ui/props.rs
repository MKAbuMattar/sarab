//! "Customize": the controls a wallpaper declares in its properties.json, for one display.

use super::{Model, act, t};
use crate::engine::wallpaper;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::{DialogClose, DialogFooter};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::select::{Select, SelectEvent, SelectState};
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, IndexPath, WindowExt as _, h_flex, v_flex};
use gpui_kit::*;
use serde_json::{Map, Value};

enum Control {
    Slider(Entity<SliderState>),
    Switch(bool),
    Select(Entity<SelectState<Vec<SharedString>>>),
    Text(Entity<InputState>),
    Button(SharedString),
    Label,
}

struct Row {
    key: String,
    label: SharedString,
    value: SharedString,
    control: Control,
}

pub struct Props {
    display: usize,
    rows: Vec<Row>,
    _subscriptions: Vec<Subscription>,
}

fn send(display: usize, key: String, value: String, cx: &mut App) {
    act(cx, move |e| {
        wallpaper::set_prop(e, display, &key, &Value::String(value)).map(|_| ())
    });
}

impl Props {
    fn new(
        display: usize,
        ctls: Map<String, Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut subs = vec![];
        let mut rows = vec![];
        for (key, c) in ctls {
            let label: SharedString = c
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or(&key)
                .to_string()
                .into();
            let value = c.get("value").cloned().unwrap_or(Value::Null);
            let text = match &value {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                v => v.to_string(),
            };
            let num = |k: &str, d: f64| c.get(k).and_then(Value::as_f64).unwrap_or(d) as f32;
            let control = match c.get("type").and_then(Value::as_str).unwrap_or("") {
                "slider" => {
                    let state = cx.new(|_| {
                        SliderState::new()
                            .min(num("min", 0.))
                            .max(num("max", 100.))
                            .step(num("step", 1.))
                            .default_value(value.as_f64().unwrap_or(0.) as f32)
                    });
                    let k = key.clone();
                    subs.push(cx.subscribe(
                        &state,
                        move |this: &mut Props, _, ev: &SliderEvent, cx| {
                            if let SliderEvent::Change(v) = ev {
                                let v = v.start();
                                if let Some(r) = this.rows.iter_mut().find(|r| r.key == k) {
                                    r.value = format!("{v}").into();
                                }
                                send(this.display, k.clone(), v.to_string(), cx);
                                cx.notify();
                            }
                        },
                    ));
                    Control::Slider(state)
                }
                "checkbox" => Control::Switch(value.as_bool().unwrap_or(false)),
                "dropdown" | "scalerDropdown" => {
                    let items: Vec<SharedString> = c
                        .get("items")
                        .and_then(Value::as_array)
                        .map(|a| {
                            a.iter()
                                .map(|i| i.as_str().unwrap_or_default().to_string().into())
                                .collect()
                        })
                        .unwrap_or_default();
                    let at = value.as_u64().map(|i| IndexPath::new(i as usize));
                    let list = items.clone();
                    let state = cx.new(|cx| SelectState::new(items, at, window, cx));
                    let k = key.clone();
                    subs.push(cx.subscribe(
                        &state,
                        move |this: &mut Props, _, ev: &SelectEvent<Vec<SharedString>>, cx| {
                            let SelectEvent::Confirm(Some(v)) = ev else {
                                return;
                            };
                            if let Some(i) = list.iter().position(|x| x == v) {
                                send(this.display, k.clone(), i.to_string(), cx);
                            }
                        },
                    ));
                    Control::Select(state)
                }
                "button" => Control::Button(if text.is_empty() {
                    label.clone()
                } else {
                    text.clone().into()
                }),
                "label" => Control::Label,
                // textbox, color as a hex code, and folderDropdown as a plain file name.
                _ => {
                    let state =
                        cx.new(|cx| InputState::new(window, cx).default_value(text.clone()));
                    let k = key.clone();
                    subs.push(cx.subscribe(
                        &state,
                        move |this: &mut Props, state, ev: &InputEvent, cx| {
                            if matches!(ev, InputEvent::PressEnter { .. } | InputEvent::Blur) {
                                let v = state.read(cx).value().to_string();
                                send(this.display, k.clone(), v, cx);
                            }
                        },
                    ));
                    Control::Text(state)
                }
            };
            rows.push(Row {
                key,
                label,
                value: text.into(),
                control,
            });
        }
        Props {
            display,
            rows,
            _subscriptions: subs,
        }
    }
}

impl Render for Props {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.rows.is_empty() {
            return v_flex().child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(t(cx, "props.none")),
            );
        }
        v_flex()
            .gap_4()
            .children(self.rows.iter().enumerate().map(|(n, r)| {
                let id = SharedString::from(format!("prop-{}", r.key));
                let label = r.label.clone();
                let key = r.key.clone();
                let display = self.display;
                match &r.control {
                    Control::Slider(s) => v_flex()
                        .gap_1()
                        .child(
                            h_flex()
                                .justify_between()
                                .child(label)
                                .child(r.value.clone()),
                        )
                        .child(Slider::new(s))
                        .into_any_element(),
                    Control::Switch(on) => Switch::new(id)
                        .label(label)
                        .checked(*on)
                        .on_change(cx.listener(move |this, v: &bool, _, cx| {
                            if let Control::Switch(on) = &mut this.rows[n].control {
                                *on = *v;
                            }
                            send(display, key.clone(), v.to_string(), cx);
                            cx.notify();
                        }))
                        .into_any_element(),
                    Control::Select(s) => v_flex()
                        .gap_1()
                        .child(label)
                        .child(Select::new(s))
                        .into_any_element(),
                    Control::Text(s) => v_flex()
                        .gap_1()
                        .child(label)
                        .child(Input::new(s))
                        .into_any_element(),
                    Control::Button(text) => Button::new(id)
                        .label(text.clone())
                        .on_click(move |_, _, cx| send(display, key.clone(), "true".into(), cx))
                        .into_any_element(),
                    Control::Label => div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(label)
                        .into_any_element(),
                }
            }))
    }
}

/// Load the display's controls from the engine, then show them in a dialog.
pub fn open(display: usize, window: &mut Window, cx: &mut App) {
    let rx = Model::get(cx)
        .engine
        .ask(move |e| wallpaper::props_for(&e.core, display));
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        let Ok(ctls) = rx.await else { return };
        let _ = cx.update_window(handle, |_, window, cx| {
            let view = cx.new(|cx| Props::new(display, ctls, window, cx));
            window.open_dialog(cx, move |d, _, cx| {
                d.title(t(cx, "props.title"))
                    .w(px(440.))
                    .child(view.clone())
                    .footer(
                        DialogFooter::new().child(
                            DialogClose::new().child(
                                Button::new("props-done")
                                    .primary()
                                    .label(t(cx, "props.close")),
                            ),
                        ),
                    )
            });
        });
    })
    .detach();
}
