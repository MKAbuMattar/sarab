//! The Settings page: appearance, pausing, performance, apps, startup.

use super::{Model, act, save, snap, t};
use crate::engine::AUTOSTART_FLAG;
use crate::model::settings::Settings;
use crate::platform::autostart;
use gpui_kit::component::IconName;
use gpui_kit::component::h_flex;
use gpui_kit::component::setting::{SettingField, SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::slider::{Slider, SliderState};
use gpui_kit::*;

fn switch(
    title: &str,
    get: fn(&Settings) -> bool,
    set: fn(&mut Settings, bool),
    cx: &App,
) -> SettingItem {
    SettingItem::new(
        t(cx, title),
        SettingField::switch(
            move |cx: &App| get(&snap(cx).settings),
            move |v, cx: &mut App| save(cx, |s| set(s, v)),
        ),
    )
}

fn dropdown(
    title: &str,
    options: Vec<(SharedString, SharedString)>,
    get: fn(&Settings) -> String,
    set: fn(&mut Settings, String),
    cx: &App,
) -> SettingItem {
    SettingItem::new(
        t(cx, title),
        SettingField::dropdown(
            options,
            move |cx: &App| get(&snap(cx).settings).into(),
            move |v: SharedString, cx: &mut App| save(cx, |s| set(s, v.to_string())),
        ),
    )
}

/// One app name per line in the old window; here a comma-separated list.
fn apps(
    title: &str,
    get: fn(&Settings) -> &Vec<String>,
    set: fn(&mut Settings, Vec<String>),
    cx: &App,
) -> SettingItem {
    SettingItem::new(
        t(cx, title),
        SettingField::input(
            move |cx: &App| get(&snap(cx).settings).join(", ").into(),
            move |v: SharedString, cx: &mut App| {
                let list = v
                    .split([',', '\n'])
                    .map(str::trim)
                    .filter(|x| !x.is_empty())
                    .map(String::from)
                    .collect();
                save(cx, |s| set(s, list))
            },
        ),
    )
    .layout(Axis::Vertical)
    .description("notepad.exe, code.exe")
}

pub fn page(volume: Entity<SliderState>, cx: &App) -> SettingPage {
    let m = Model::get(cx);
    let opt = |v: &str, key: &str| (SharedString::from(v.to_string()), m.t(key));
    let fullscreen = snap(cx).settings.pause_fullscreen;
    SettingPage::new(m.t("settings.title"))
        .title_suffix(|_, cx| super::pause_button(cx))
        .icon(IconName::Settings)
        .groups([
            SettingGroup::new()
                .title(m.t("settings.appearance"))
                .items([
                    dropdown(
                        "settings.theme",
                        vec![
                            opt("system", "theme.system"),
                            opt("light", "theme.light"),
                            opt("dark", "theme.dark"),
                        ],
                        |s| s.theme.clone(),
                        |s, v| s.theme = v,
                        cx,
                    ),
                    dropdown(
                        "settings.backdrop",
                        vec![
                            opt("acrylic", "backdrop.acrylic"),
                            opt("mica", "backdrop.mica"),
                            opt("solid", "backdrop.solid"),
                        ],
                        |s| s.backdrop.clone(),
                        |s, v| s.backdrop = v,
                        cx,
                    )
                    .description(m.t("settings.backdropDesc")),
                    dropdown(
                        "settings.language",
                        vec![
                            ("en".into(), "English".into()),
                            ("ar".into(), super::rtl::visual("العربية").into()),
                        ],
                        |s| s.language.clone(),
                        |s, v| s.language = v,
                        cx,
                    ),
                ]),
            SettingGroup::new().title(m.t("settings.pausing")).items([
                switch(
                    "settings.pauseFullscreen",
                    |s| s.pause_fullscreen,
                    |s, v| s.pause_fullscreen = v,
                    cx,
                )
                .description(m.t("settings.pauseFullscreenDesc")),
                switch(
                    "settings.perDisplay",
                    |s| s.per_display,
                    |s, v| s.per_display = v,
                    cx,
                )
                .disabled(!fullscreen),
                switch(
                    "settings.pauseFocus",
                    |s| s.pause_focus,
                    |s, v| s.pause_focus = v,
                    cx,
                ),
                switch(
                    "settings.pauseBattery",
                    |s| s.pause_battery,
                    |s, v| s.pause_battery = v,
                    cx,
                ),
                switch(
                    "settings.pausePowerSaver",
                    |s| s.pause_power_saver,
                    |s, v| s.pause_power_saver = v,
                    cx,
                ),
                switch(
                    "settings.pauseRemote",
                    |s| s.pause_remote,
                    |s, v| s.pause_remote = v,
                    cx,
                ),
            ]),
            SettingGroup::new()
                .title(m.t("settings.performance"))
                .items([
                    dropdown(
                        "settings.fps",
                        vec![
                            ("15".into(), "15".into()),
                            ("30".into(), "30".into()),
                            ("60".into(), "60".into()),
                            opt("0", "settings.fpsUnlimited"),
                        ],
                        |s| s.fps.to_string(),
                        |s, v| s.fps = v.parse().unwrap_or(30),
                        cx,
                    )
                    .description(m.t("settings.fpsDesc")),
                    SettingItem::new(
                        m.t("settings.volume"),
                        SettingField::render(move |_, _, cx| {
                            h_flex()
                                .w(px(220.))
                                .gap_3()
                                .child(div().flex_1().child(Slider::new(&volume)))
                                .child(div().w(px(28.)).child(snap(cx).settings.volume.to_string()))
                        }),
                    ),
                ]),
            SettingGroup::new().title(m.t("settings.apps")).items([
                apps(
                    "settings.appPause",
                    |s| &s.app_pause,
                    |s, v| s.app_pause = v,
                    cx,
                ),
                apps(
                    "settings.appPlay",
                    |s| &s.app_play,
                    |s, v| s.app_play = v,
                    cx,
                ),
            ]),
            SettingGroup::new()
                .title(m.t("settings.startup"))
                .item(SettingItem::new(
                    m.t("settings.autostart"),
                    SettingField::switch(
                        |cx: &App| snap(cx).autostart,
                        |v, cx: &mut App| {
                            act(cx, move |e| {
                                let r = autostart::set(v, AUTOSTART_FLAG);
                                e.changed();
                                r
                            })
                        },
                    ),
                )),
        ])
}
