//! The action's own settings, laid out as three compact cards across the
//! page: who it is, what it reports, how input is shared.

use gpui::*;
use ui::button::{Button, ButtonVariants};
use ui::input::{NumberInput, TextInput};
use ui::switch::Switch;
use ui::{h_flex, v_flex, ActiveTheme, IconName, Sizable};

use crate::model::action::to_identifier;
use crate::model::behavior::ValueType;
use crate::runtime::events;
use crate::runtime::system::Phase;

use super::super::fields::{Fields, NumberSpec};
use super::super::form::{note, section, setting};
use super::ActionCtx;

pub struct SettingsPanel {
    ctx: ActionCtx,
    fields: Fields<Self>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsPanel {
    pub fn new(ctx: ActionCtx, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&ctx.doc, |_, _, cx| cx.notify())];
        Self { ctx, fields: Fields::default(), _subscriptions: subscriptions }
    }

    fn value_type_button(&self, value_type: ValueType, current: ValueType, cx: &mut Context<Self>) -> impl IntoElement {
        let button = Button::new(SharedString::from(format!("value-type-{}", value_type.short())))
            .label(value_type.short())
            .small()
            .tooltip(format!("{}\n{}", value_type.label(), value_type.description()))
            .on_click(cx.listener(move |this, _, _, cx| this.ctx.edit("Change value type", None, cx, |a| a.value_type = value_type)));
        div().flex_1().child(if value_type == current { button.primary().w_full() } else { button.outline().w_full() })
    }

    /// The five event names as one row of copyable chips.
    fn events_row(&self, name: &str, cx: &App) -> impl IntoElement {
        let theme = cx.theme();
        h_flex().w_full().flex_wrap().gap_1().items_center().child(div().text_xs().text_color(theme.muted_foreground).child(format!("input::{name}."))).children(
            Phase::ALL.iter().map(|&phase| {
                let copy = events::qualified(name, phase);
                Button::new(SharedString::from(format!("copy-event-{}", phase.word())))
                    .label(phase.word())
                    .ghost()
                    .xsmall()
                    .tooltip(format!("Copy {copy}"))
                    .on_click(move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(copy.clone())))
            }),
        )
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

        let identity = section("Identity", IconName::Component, cx)
            .child(setting("Script name", "What scripts call this action. Empty uses the file name.", TextInput::new(&identifier).small(), cx))
            .child(setting("Description", "What this action is for.", TextInput::new(&description).small(), cx))
            .child(self.events_row(&name, cx))
            .child(note("Events carry entity, value, x, y, z, elapsed, triggered_for.", cx));

        let current = asset.value_type;
        let value = section("Value", IconName::DashboardSpeed, cx)
            .child(h_flex().w_full().gap_1().children(ValueType::ALL.map(|t| self.value_type_button(t, current, cx))))
            .child(note(current.description(), cx));

        let consume = Switch::new("consume-input").checked(asset.consume_input).on_click(cx.listener(|this, checked: &bool, _, cx| {
            let checked = *checked;
            this.ctx.edit("Change consume input", None, cx, |a| a.consume_input = checked)
        }));
        let paused = Switch::new("when-paused").checked(asset.trigger_when_paused).on_click(cx.listener(|this, checked: &bool, _, cx| {
            let checked = *checked;
            this.ctx.edit("Change trigger when paused", None, cx, |a| a.trigger_when_paused = checked)
        }));
        let behavior = section("Behavior", IconName::Settings, cx)
            .child(setting("Consume input", "While active, keys it uses are hidden from mappings with a lower priority.", consume, cx))
            .child(setting("Trigger when paused", "Keep working while the game is paused, for menus.", paused, cx))
            .child(setting("Actuation threshold", "How far from rest a value must be to count as pressed.", NumberInput::new(&threshold).small(), cx));

        let card = |content: Div| v_flex().flex_1().min_w(px(280.0)).p_2().rounded(px(8.0)).bg(theme.secondary.opacity(0.4)).border_1().border_color(theme.border).child(content);
        h_flex().w_full().flex_wrap().gap_2().items_start().child(card(identity)).child(card(value)).child(card(behavior))
    }
}
