//! The preview: the action tried against a made-up input, explained.
//!
//! It reads top to bottom: what is being tried, what the action is set to
//! do, a chart of what happens (with every lane named and a time axis), what
//! the colours mean, and the events scripts would hear.

use std::rc::Rc;

use gpui::*;
use ui::button::{Button, ButtonVariants};
use ui::scroll::ScrollbarAxis;
use ui::{h_flex, v_flex, ActiveTheme, IconName, Sizable, StyledExt};

use crate::model::ActionAsset;
use crate::preview::{describe, log, peak, phase_meaning, simulate, verdict, Pattern, Sample, STEP};
use crate::runtime::system::Phase;
use crate::runtime::trigger::TriggerState;

use super::super::common::panel::dock_panel;
use super::super::form::section;
use super::ActionCtx;

pub struct PreviewPanel {
    ctx: ActionCtx,
    pattern: Pattern,
    cache: Option<(ActionAsset, Pattern, Rc<Vec<Sample>>)>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

dock_panel!(PreviewPanel, "input-action-preview", "Preview");

const LANE: f32 = 56.0;
/// Seconds between the chart's grid lines.
const GRID: f32 = 0.5;

impl PreviewPanel {
    pub fn new(ctx: ActionCtx, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&ctx.doc, |_, _, cx| cx.notify())];
        Self { ctx, pattern: Pattern::Hold, cache: None, focus_handle: cx.focus_handle(), _subscriptions: subscriptions }
    }

    /// The simulation for the action as it is now, rerun only when it changed.
    fn samples(&mut self, asset: &ActionAsset) -> Rc<Vec<Sample>> {
        if let Some((cached, pattern, samples)) = &self.cache {
            if cached == asset && *pattern == self.pattern {
                return samples.clone();
            }
        }
        let samples = Rc::new(simulate(asset, self.pattern));
        self.cache = Some((asset.clone(), self.pattern, samples.clone()));
        samples
    }
}

/// One lane of the chart, named and explained above, with a grid behind it.
fn lane(label: &'static str, hint: String, seconds: f32, paint: impl Fn(Bounds<Pixels>, &mut Window) + 'static, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let grid = theme.border;
    v_flex()
        .w_full()
        .gap_1()
        .child(
            v_flex()
                .child(div().text_sm().font_semibold().text_color(theme.foreground).child(label))
                .child(div().text_xs().text_color(theme.muted_foreground).child(hint)),
        )
        .child(
            div().w_full().h(px(LANE)).rounded(px(4.0)).bg(theme.background).border_1().border_color(theme.border).child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let (x0, y0, w, h) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y), f32::from(bounds.size.width), f32::from(bounds.size.height));
                        let mut t = GRID;
                        while t < seconds {
                            let x = x0 + t / seconds * w;
                            window.paint_quad(fill(Bounds::new(point(px(x), px(y0)), size(px(1.0), px(h))), grid.opacity(0.6)));
                            t += GRID;
                        }
                        paint(bounds, window);
                    },
                )
                .size_full(),
            ),
        )
}

/// A coloured square and what it means.
fn swatch(color: Hsla, text: &'static str, cx: &App) -> impl IntoElement {
    h_flex()
        .gap_1p5()
        .items_center()
        .child(div().size(px(10.0)).rounded(px(2.0)).bg(color))
        .child(div().text_xs().text_color(cx.theme().muted_foreground).child(text))
}

impl Render for PreviewPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let asset = self.ctx.asset(cx);
        let samples = self.samples(&asset);
        let seconds = self.pattern.seconds();
        let scale = peak(&samples);
        let (accent, good, warn, bad, muted) = (theme.info, theme.success, theme.warning, theme.danger, theme.muted_foreground);

        let patterns = h_flex().gap_1().flex_wrap().children(Pattern::ALL.map(|p| {
            let button = Button::new(SharedString::from(format!("preview-{}", p.label())))
                .label(p.label())
                .small()
                .tooltip(p.description())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.pattern = p;
                    cx.notify();
                }));
            if p == self.pattern {
                button.primary()
            } else {
                button.outline()
            }
        }));

        // What the action is set to do, as sentences.
        let rules = v_flex().w_full().gap_0p5().children(describe(&asset).into_iter().map(|line| {
            h_flex()
                .gap_1p5()
                .items_start()
                .child(div().text_sm().text_color(theme.muted_foreground).child("•"))
                .child(div().flex_1().min_w_0().text_sm().text_color(theme.foreground).child(line))
        }));

        // The chart.
        let bar = move |bounds: Bounds<Pixels>, t: f32, dt: f32, height: f32, color: Hsla, window: &mut Window| {
            if height <= 0.0 {
                return;
            }
            let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
            let x = f32::from(bounds.origin.x) + t / seconds * w;
            let bar_h = (h - 6.0) * height.clamp(0.0, 1.0);
            let y = f32::from(bounds.origin.y) + h - 3.0 - bar_h;
            window.paint_quad(fill(Bounds::new(point(px(x), px(y)), size(px((dt / seconds * w).max(1.0) + 0.5), px(bar_h))), color));
        };
        let (input_samples, value_samples, state_samples) = (samples.clone(), samples.clone(), samples.clone());
        let chart = v_flex()
            .w_full()
            .gap_3()
            .child(lane(
                "What you do",
                self.pattern.device().to_string(),
                seconds,
                move |bounds, window| {
                    for s in input_samples.iter() {
                        bar(bounds, s.time, STEP, s.input, accent.opacity(0.8), window);
                    }
                },
                cx,
            ))
            .child(lane(
                "The action's value",
                format!("what the action reads after its modifiers; full height is {scale:.2}"),
                seconds,
                move |bounds, window| {
                    for s in value_samples.iter() {
                        bar(bounds, s.time, STEP, crate::model::behavior::magnitude(s.value) / scale, accent, window);
                    }
                },
                cx,
            ))
            .child(lane(
                "What the action is doing",
                "idle, building up, or firing".to_string(),
                seconds,
                move |bounds, window| {
                    for s in state_samples.iter() {
                        let (height, color) = match s.state {
                            TriggerState::None => (0.0, muted),
                            TriggerState::Ongoing => (0.45, warn),
                            TriggerState::Triggered => (1.0, good),
                        };
                        bar(bounds, s.time, STEP, height, color, window);
                        if s.phases.contains(&Phase::Canceled) {
                            bar(bounds, s.time, STEP * 2.0, 1.0, bad, window);
                        }
                    }
                },
                cx,
            ));

        // Seconds along the bottom, under the lanes.
        let ticks = (0..=((seconds / GRID).floor() as usize)).map(|i| div().text_xs().text_color(theme.muted_foreground).child(format!("{:.1} s", i as f32 * GRID)));
        let axis = h_flex().w_full().justify_between().children(ticks);
        let legend = h_flex()
            .w_full()
            .flex_wrap()
            .gap_x_4()
            .gap_y_1()
            .child(swatch(warn, "building: started, not firing yet", cx))
            .child(swatch(good, "firing: scripts hear triggered", cx))
            .child(swatch(bad, "stopped before it fired", cx));

        // What scripts hear.
        let entries = log(&samples);
        let events = v_flex().w_full().gap_1().children(entries.into_iter().map(|e| {
            let color = match e.phase {
                Phase::Started => accent,
                Phase::Ongoing => warn,
                Phase::Triggered => good,
                Phase::Completed => muted,
                Phase::Canceled => bad,
            };
            let when = if e.frames > 1 { format!("{:.2} to {:.2} s", e.from, e.to) } else { format!("{:.2} s", e.from) };
            let every = if e.frames > 1 { format!(" ({} frames)", e.frames) } else { String::new() };
            h_flex()
                .w_full()
                .gap_2()
                .items_baseline()
                .child(div().w(px(120.0)).flex_shrink_0().text_xs().text_color(theme.muted_foreground).child(when))
                .child(div().w(px(80.0)).flex_shrink_0().text_sm().font_semibold().text_color(color).child(e.phase.word()))
                .child(div().flex_1().min_w_0().text_xs().text_color(theme.muted_foreground).child(format!("{}{every}", phase_meaning(e.phase))))
        }));

        let content = v_flex()
            .w_full()
            .p_3()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .child(section("Try it with", IconName::Activity, cx))
                    .child(patterns)
                    .child(div().text_sm().text_color(theme.foreground).child(self.pattern.story())),
            )
            .child(v_flex().gap_2().child(section("The action is set to", IconName::Settings, cx)).child(rules))
            .child(
                v_flex()
                    .gap_2()
                    .child(section("What happens", IconName::DashboardSpeed, cx))
                    .child(chart)
                    .child(axis)
                    .child(legend),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(section("What scripts hear", IconName::Flash, cx))
                    .child(div().text_sm().font_semibold().text_color(theme.foreground).child(verdict(&samples)))
                    .child(events),
            );

        v_flex().size_full().bg(theme.sidebar).track_focus(&self.focus_handle).child(
            v_flex().id("input-action-preview-scroll").size_full().scrollable(ScrollbarAxis::Vertical).child(content),
        )
    }
}
