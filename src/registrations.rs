//! What the input plugin registers: the one place it submits its
//! contributions, grouped by the engine seam each plugs into. The runtime
//! part is what a game links; the editor part comes with the `editor`
//! feature.

use pulsar_scenedb::component_id;
use pulsar_world_registry::ComponentTickRegistration;

use crate::runtime::component::{PlayerInputComponent, PLAYER_INPUT_CLASS_NAME};

// ── Component classes ───────────────────────────────────────────────────────

pulsar_world_registry::submit_world_component!(PlayerInputComponent);

// ── Input runtime ───────────────────────────────────────────────────────────

// One world-wide tick a frame reads the devices, evaluates the active
// mappings and publishes the `input::` events (`crate::runtime`).
inventory::submit! {
    ComponentTickRegistration {
        type_name: "input_plugin::driver",
        class_name: PLAYER_INPUT_CLASS_NAME,
        handler_events: &[],
        component_type: component_id::<PlayerInputComponent>,
        tick: crate::runtime::tick,
        end_play: None,
    }
}

// ── Game session ────────────────────────────────────────────────────────────

// The session declares the project's action events, a possession puts the
// level's mapping to work on the pawn, and the end releases the devices
// (`crate::runtime::session`).
inventory::submit! {
    engine_backend::scene::game_hooks::GameSessionRegistration {
        origin: pulsar_plugins::origin!(),
        name: "input.player",
        begin: Some(crate::runtime::session::begin),
        possess: Some(crate::runtime::session::possess),
        end: Some(crate::runtime::end_session),
    }
}

// ── Editor ──────────────────────────────────────────────────────────────────

/// What the editor adds: the action and mapping editors and file types, and
/// the Player Input component's "Add component" entry, as a linked editor
/// provider.
#[cfg(feature = "editor")]
mod editor {
    use std::sync::Arc;

    use crate::builtin_provider::InputBuiltinProvider;

    inventory::submit! {
        plugin_manager::PluginEditorProvider {
            origin: pulsar_plugins::origin!(),
            id: InputBuiltinProvider::ID,
            create: || Arc::new(InputBuiltinProvider),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_component_class_is_registered() {
        assert!(pulsar_world_registry::component_id_for_class(PLAYER_INPUT_CLASS_NAME).is_some());
    }

    /// Every registration of the plugin names this crate, so it resolves to
    /// the plugin's id.
    #[test]
    fn every_registration_resolves_to_the_plugin() {
        let ours: Vec<_> = pulsar_plugins::contributions()
            .into_iter()
            .filter(|summary| summary.origin == pulsar_plugins::origin!())
            .collect();
        assert!(ours
            .iter()
            .any(|summary| summary.category == "component.class"));
        for summary in &ours {
            assert_eq!(
                pulsar_plugins::plugin_of(summary.origin).map(|plugin| plugin.id.as_str()),
                Some("pulsar.input"),
                "{summary:?}"
            );
        }
    }
}
