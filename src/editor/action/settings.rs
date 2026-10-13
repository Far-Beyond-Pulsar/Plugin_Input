//! The action's own settings, as one band across the top of the page: every
//! setting is a small labelled field, so they flow along the width instead of
//! stacking down it.

use gpui::*;
use ui::button::{Button, ButtonVariants};
use ui::input::{NumberInput, TextInput};
use ui::switch::Switch;
use ui::{h_flex, v_flex, ActiveTheme, Sizable, StyledExt};

use crate::model::action::to_identifier;
use crate::model::behavior::ValueType;
use crate::runtime::events;
use crate::runtime::system::Phase;

use super::super::common::panel::dock_panel;
use super::super::fields::{Fields, NumberSpec};
use super::super::form::{field, setting};
use ui::scroll::ScrollbarAxis;
use super::ActionCtx;

pub struct SettingsPanel {
    ctx: ActionCtx,
    fields: Fields<Self>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

dock_panel!(SettingsPanel, "input-action-settings", "Action");

impl SettingsPanel {
    pub fn new(ctx: ActionCtx, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&ctx.doc, |_, _, cx| cx.notify())];
        Self { ctx, fields: Fields::default(), focus_handle: cx.focus_handle(), _subscriptions: subscriptions }
    }

    fn value_type_button(&self, value_type: ValueType, current: ValueType, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new(SharedString::from(format!("value-type-{}", value_type.short())))
            .label(value_type.short())
            .small()
            .tooltip(format!("{}\n{}", value_type.label(), value_type.description()))
            .on_click(cx.listener(move |this, _, _, cx| this.ctx.edit("Change value type", None, cx, |a| a.value_type = value_type)));
        div().w(px(56.0)).child(if value_type == current { button.primary().w_full() } else { button.outline().w_full() })
    }

    /// The five event names, one under another, each copies on click.
    fn events(&self, name: &str) -> impl IntoElement {
        v_flex().w_full().gap_1().children(Phase::ALL.iter().map(|&phase| {
            let copy = events::qualified(name, phase);
            Button::new(SharedString::from(format!("copy-event-{}", phase.word())))
                .label(copy.clone())
                .outline()
                .xsmall()
                .w_full()
                .tooltip("Copy the event name")
                .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())))
        }))
    }
}

impl Render for SettingsPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let asset = self.ctx.asset(cx);
        let stem = self.ctx.stem(cx);
        let name = asset.event_name(&stem);

        let identifier = self.fields.text(
            "identifier".into(),
            false,
            &to_identifier(&stem),
            &asset.identifier,
            |this, cx| Some(this.ctx.asset(cx).identifier),
            |this, text, cx| this.ctx.edit("Rename script name", Some("identifier"), cx, |a| a.identifier = text.trim().to_string()),
            window,
            cx,
        );
        let description = self.fields.text(
            "description".into(),
            false,
            "What this action is for",
            &asset.description,
            |this, cx| Some(this.ctx.asset(cx).description),
            |this, text, cx| this.ctx.edit("Change description", Some("description"), cx, |a| a.description = text),
            window,
            cx,
        );
        let threshold = self.fields.number(
            "threshold".into(),
            NumberSpec { step: 0.05, min: 0.0, max: 1.0, decimals: 2 },
            Some(asset.actuation_threshold),
            |this, cx| Some(this.ctx.asset(cx).actuation_threshold),
            |this, v, cx| this.ctx.edit("Change threshold", Some("threshold"), cx, |a| a.actuation_threshold = v),
            window,
            cx,
        );

        let current = asset.value_type;
        let types = h_flex().gap_1().children(ValueType::ALL.map(|t| self.value_type_button(t, current, cx)));
        let consume = Switch::new("consume-input").checked(asset.consume_input).on_click(cx.listener(|this, checked: &bool, _, cx| {
            let checked = *checked;
            this.ctx.edit("Change consume input", None, cx, |a| a.consume_input = checked)
        }));
        let paused = Switch::new("when-paused").checked(asset.trigger_when_paused).on_click(cx.listener(|this, checked: &bool, _, cx| {
            let checked = *checked;
            this.ctx.edit("Change trigger when paused", None, cx, |a| a.trigger_when_paused = checked)
        }));

        let rows = v_flex()
            .w_full()
            .p_3()
            .gap_3()
            .child(field("Script name", TextInput::new(&identifier).small(), cx))
            .child(field("Description", TextInput::new(&description).small(), cx))
            .child(field("Reports", types, cx))
            .child(field("Actuation threshold", NumberInput::new(&threshold).small(), cx))
            .child(setting("Consume input", "Keys this action uses are hidden from lower-priority mappings while it is active.", consume, cx))
            .child(setting("Works when paused", "Keeps working while the game is paused, for menus.", paused, cx))
            .child(field(format!("Events scripts hear (each carries entity, value, x, y, z, elapsed, triggered_for)"), self.events(&name), cx));
        v_flex().size_full().bg(theme.sidebar).track_focus(&self.focus_handle).child(v_flex().id("input-action-settings-scroll").size_full().scrollable(ScrollbarAxis::Vertical).child(rows))
    }
}
