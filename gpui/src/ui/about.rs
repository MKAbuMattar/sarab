//! The About page and the update banner.

use super::{Model, act, save, snap, t};
use crate::engine::wallpaper::library_dir;
use crate::engine::{ISSUES, WEBSITE, update};
use crate::model::settings;
use gpui_kit::component::StyledExt as _;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::dialog::{DialogClose, DialogFooter};
use gpui_kit::component::setting::{SettingField, SettingGroup, SettingItem, SettingPage};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _, WindowExt as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The Jordanian palette the app's colors come from.
const COLORS: [u32; 5] = [0x2B2233, 0xD9A36A, 0x2F5D6B, 0x8B1E2D, 0x6B7A4F];

fn open_path(which: &'static str, cx: &mut App) {
    act(cx, move |e| {
        let dir = match which {
            "config" => settings::config_dir(),
            _ => library_dir(&e.core.settings),
        };
        std::fs::create_dir_all(&dir).map_err(|err| err.to_string())?;
        // explorer.exe opens the folder in a new window.
        std::process::Command::new("explorer.exe")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|err| err.to_string())
    });
}

fn link(
    title: SharedString,
    caption: SharedString,
    id: &'static str,
    url: &'static str,
) -> SettingItem {
    SettingItem::new(
        title,
        SettingField::render(move |_, _, cx| {
            Button::new(id)
                .label(t(cx, "about.visit"))
                .icon(IconName::ExternalLink)
                .on_click(|_, _, cx| cx.open_url(url))
        }),
    )
    .description(caption)
}

fn update_status(cx: &App) -> SharedString {
    let m = Model::get(cx);
    let u = &snap(cx).update;
    match (&u.available, &u.error, u.checked) {
        (Some((v, _)), _, _) => m.tf("update.available", &[("v", v)]),
        (_, Some(e), _) => format!("{} {e}", m.t("update.failed")).into(),
        (_, _, Some(_)) => m.tf("update.latest", &[("v", env!("CARGO_PKG_VERSION"))]),
        _ => m.t("update.never"),
    }
}

pub fn page(cx: &App) -> SettingPage {
    let m = Model::get(cx);
    let webview = snap(cx).webview.clone();
    SettingPage::new(m.t("about.title"))
        .title_suffix(|_, cx| super::pause_button(cx))
        .icon(IconName::Info)
        .groups([
            SettingGroup::new()
                .item(SettingItem::render(|_, _, cx| {
                    let m = Model::get(cx);
                    h_flex()
                        .gap_4()
                        .child(img("sarab/logo.png").size(px(64.)))
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xl()
                                        .font_semibold()
                                        .child(format!("Sarab  {}", super::rtl::visual("سراب"))),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(m.tf(
                                            "about.version",
                                            &[("v", env!("CARGO_PKG_VERSION"))],
                                        )),
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(m.t("app.tagline")),
                                ),
                        )
                }))
                .footer(move |_, cx| t(cx, "about.description")),
            SettingGroup::new().title(m.t("about.details")).items([
                SettingItem::new(
                    m.t("about.license"),
                    SettingField::render(|_, _, _| "GNU GPL v3"),
                ),
                SettingItem::new(
                    m.t("about.colors"),
                    SettingField::render(|_, _, _| {
                        h_flex()
                            .gap_1()
                            .children(COLORS.map(|c| div().size(px(18.)).rounded_full().bg(rgb(c))))
                    }),
                )
                .description(m.t("about.colorsDesc")),
                SettingItem::new(
                    m.t("about.webview"),
                    SettingField::render(move |_, _, _| webview.clone()),
                ),
                link(
                    m.t("about.website"),
                    "github.com/MKAbuMattar/sarab".into(),
                    "website",
                    WEBSITE,
                ),
                link(
                    m.t("about.report"),
                    m.t("about.reportDesc"),
                    "issues",
                    ISSUES,
                ),
            ]),
            SettingGroup::new().title(m.t("about.updates")).items([
                SettingItem::new(
                    m.t("about.autoUpdate"),
                    SettingField::switch(
                        |cx: &App| snap(cx).settings.check_updates,
                        |v, cx: &mut App| save(cx, |s| s.check_updates = v),
                    ),
                )
                .description(m.t("about.autoUpdateDesc")),
                SettingItem::new(
                    update_status(cx),
                    SettingField::render(|_, _, cx| {
                        Button::new("check-now")
                            .label(t(cx, "about.checkNow"))
                            .on_click(|_, _, cx| {
                                act(cx, |e| {
                                    update::spawn_check(e.handle.clone());
                                    Ok(())
                                })
                            })
                    }),
                ),
            ]),
            SettingGroup::new().title(m.t("about.folders")).items([
                SettingItem::new(
                    m.t("about.config"),
                    SettingField::render(|_, _, cx| {
                        Button::new("open-config")
                            .label(t(cx, "about.open"))
                            .icon(IconName::FolderOpen)
                            .on_click(|_, _, cx| open_path("config", cx))
                    }),
                )
                .description(m.t("about.configDesc")),
                SettingItem::new(
                    m.t("about.library"),
                    SettingField::render(|_, _, cx| {
                        Button::new("open-library")
                            .label(t(cx, "about.open"))
                            .icon(IconName::FolderOpen)
                            .on_click(|_, _, cx| open_path("library", cx))
                    }),
                ),
            ]),
        ])
}

/// Shown on every page while a newer version exists.
pub fn update_banner(cx: &App) -> Option<AnyElement> {
    let m = Model::get(cx);
    let u = snap(cx).update.clone();
    let (version, notes) = u.available.clone()?;
    if m.update_later && u.progress.is_none() {
        return None;
    }
    let text = match u.progress {
        Some(p) => m.tf(
            "update.downloading",
            &[("v", &version), ("p", &p.to_string())],
        ),
        None => m.tf("update.available", &[("v", &version)]),
    };
    let actions = u.progress.is_none().then(|| {
        let title = m.tf("update.notesTitle", &[("v", &version)]);
        h_flex()
            .gap_2()
            .child(
                Button::new("update-now")
                    .primary()
                    .small()
                    .icon(IconName::ArrowDown)
                    .label(m.t("update.now"))
                    .on_click(|_, _, cx| {
                        act(cx, |e| {
                            update::spawn_install(e.handle.clone());
                            Ok(())
                        })
                    }),
            )
            .when(!notes.is_empty(), |row| {
                row.child(
                    Button::new("update-notes")
                        .ghost()
                        .small()
                        .label(m.t("update.notes"))
                        .on_click(move |_, window, cx| {
                            let (title, notes) = (title.clone(), notes.clone());
                            window.open_dialog(cx, move |d, _, cx| {
                                d.title(title.clone())
                                    .child(TextView::markdown("release-notes", notes.clone()))
                                    .footer(DialogFooter::new().child(DialogClose::new().child(
                                        Button::new("notes-close").label(t(cx, "dialog.close")),
                                    )))
                            });
                        }),
                )
            })
            .child(
                Button::new("update-later")
                    .ghost()
                    .small()
                    .label(m.t("update.later"))
                    .on_click(|_, _, cx| {
                        cx.global_mut::<Model>().update_later = true;
                        cx.refresh_windows();
                    }),
            )
    });
    Some(
        h_flex()
            .px_4()
            .py_2()
            .gap_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .child(IconName::RefreshCw)
            .child(div().flex_1().child(text))
            .children(actions)
            .into_any_element(),
    )
}
