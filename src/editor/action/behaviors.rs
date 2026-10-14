//! The action's triggers and modifiers, side by side under the settings.

use std::rc::Rc;

use gpui::*;
use ui::{v_flex, ActiveTheme, IconName};

use crate::model::behavior::{Modifier, Tolerant, Trigger};

use super::super::action_choices;
use super::super::behavior_ui::{behavior_list, ListModel};
use super::super::fields::Fields;
use super::super::form::section;
use super::ActionCtx;

pub struct TriggersPanel {
    ctx: ActionCtx,
    fields: Fields<Self>,
    _subscriptions: Vec<Subscription>,
}

impl TriggersPanel {
    pub fn new(ctx: ActionCtx, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&ctx.doc, |_, _, cx| cx.notify()),
            cx.observe(&ctx.lib, |_, _, cx| cx.notify()),
        ];
        Self {
            ctx,
            fields: Fields::default(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for TriggersPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let asset = self.ctx.asset(cx);
        let actions = action_choices(&self.ctx.library_now(cx), Some(&asset.id));
        let model: ListModel<Self, Trigger> = ListModel {
            get: Rc::new(|this: &Self, cx: &App| this.ctx.doc.read(cx).asset().triggers.clone()),
            set: Rc::new(
                |this: &mut Self,
                 list: Vec<Tolerant<Trigger>>,
                 typed: bool,
                 cx: &mut Context<Self>| {
                    this.ctx
                        .edit("Change triggers", typed.then_some("triggers"), cx, |a| {
                            a.triggers = list
                        })
                },
            ),
        };
        let list = behavior_list(
            "action-trigger",
            &mut self.fields,
            &asset.triggers,
            model,
            &actions,
            "None: fires every frame the input is held.",
            window,
            cx,
        );
        v_flex()
            .flex_1()
            .min_w(px(320.0))
            .p_2()
            .rounded(px(8.0))
            .bg(theme.secondary.opacity(0.4))
            .border_1()
            .border_color(theme.border)
            .child(section("Triggers: when it fires", IconName::Flash, cx).child(list))
    }
}

pub struct ModifiersPanel {
    ctx: ActionCtx,
    fields: Fields<Self>,
    _subscriptions: Vec<Subscription>,
}

impl ModifiersPanel {
    pub fn new(ctx: ActionCtx, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&ctx.doc, |_, _, cx| cx.notify())];
        Self {
            ctx,
            fields: Fields::default(),
            _subscriptions: subscriptions,
        }
    }
}

impl Render for ModifiersPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let asset = self.ctx.asset(cx);
        let model: ListModel<Self, Modifier> = ListModel {
            get: Rc::new(|this: &Self, cx: &App| this.ctx.doc.read(cx).asset().modifiers.clone()),
            set: Rc::new(
                |this: &mut Self,
                 list: Vec<Tolerant<Modifier>>,
                 typed: bool,
                 cx: &mut Context<Self>| {
                    this.ctx
                        .edit("Change modifiers", typed.then_some("modifiers"), cx, |a| {
                            a.modifiers = list
                        })
                },
            ),
        };
        let list = behavior_list(
            "action-modifier",
            &mut self.fields,
            &asset.modifiers,
            model,
            &[],
            "None: the value passes through as the keys report it.",
            window,
            cx,
        );
        v_flex()
            .flex_1()
            .min_w(px(320.0))
            .p_2()
            .rounded(px(8.0))
            .bg(theme.secondary.opacity(0.4))
            .border_1()
            .border_color(theme.border)
            .child(
                section("Modifiers: how the value is shaped", IconName::Settings, cx).child(list),
            )
    }
}
