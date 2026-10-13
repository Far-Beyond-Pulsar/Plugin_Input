//! Small pieces for laying out settings forms.

use gpui::prelude::FluentBuilder;
use gpui::*;
use ui::{h_flex, v_flex, ActiveTheme, Icon, IconName, StyledExt};

/// A titled group of settings.
pub fn section(title: impl Into<SharedString>, icon: IconName, cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .w_full()
        .gap_1p5()
        .child(
            h_flex()
                .w_full()
                .gap_1p5()
                .items_center()
                .pb_1()
                .border_b_1()
                .border_color(theme.border)
                .child(Icon::new(icon).size(px(13.0)).text_color(theme.info))
                .child(div().text_sm().font_semibold().text_color(theme.foreground).child(title.into())),
        )
}

/// A setting: its name and what it does on the left, its control on the right.
pub fn setting(label: impl Into<SharedString>, hint: impl Into<SharedString>, control: impl IntoElement, cx: &App) -> Div {
    let theme = cx.theme();
    let (label, hint): (SharedString, SharedString) = (label.into(), hint.into());
    let id = SharedString::from(format!("setting-{label}"));
    h_flex()
        .w_full()
        .gap_3()
        .items_center()
        .child(
            div()
                .id(id)
                .flex_1()
                .min_w_0()
                .text_sm()
                .text_color(theme.foreground)
                .child(label)
                .when(!hint.is_empty(), |d| d.tooltip(move |window, cx| ui::tooltip::Tooltip::new(hint.clone()).build(window, cx))),
        )
        .child(div().w(px(170.0)).flex_shrink_0().child(control))
}

/// A line of small muted text.
pub fn note(text: impl Into<SharedString>, cx: &App) -> Div {
    div().w_full().text_xs().text_color(cx.theme().muted_foreground).child(text.into())
}

/// A small rounded label.
pub fn chip(text: impl Into<SharedString>, cx: &App) -> Div {
    let theme = cx.theme();
    div().px_1p5().py_0p5().rounded(px(4.0)).bg(theme.secondary).text_xs().text_color(theme.muted_foreground).child(text.into())
}
