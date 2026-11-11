//! What the input plugin registers: the one place it registers its
//! contributions (`build`, called by the root's `build`), grouped by the
//! engine seam each plugs into. The runtime part is what a game links; the
//! editor part comes with the `editor` feature.

use engine_backend::scene::{ClassSpec, ComponentsBuilder};
use pulsar_world_registry::{systems::COMPONENTS, System};

use crate::runtime::component::PlayerInputComponent;

/// Everything the runtime part registers: the Player Input class with its
/// device system, and the game session.
pub(crate) fn build(p: &mut pulsar_plugins::PluginBuilder) {
    // ── Component classes ───────────────────────────────────────────────────

    // One world-wide system a frame reads the devices, evaluates the active
    // mappings and publishes the `input::` events (`crate::runtime`).
    p.components().class::<PlayerInputComponent>(ClassSpec {
        systems: &[System::new("pulsar.input.devices", crate::runtime::run).after(&[COMPONENTS])],
        ..ClassSpec::default()
    });

    // ── Game session ────────────────────────────────────────────────────────

    // The session declares the project's action events, a possession puts the
    // level's mapping to work on the pawn, and the end releases the devices
    // (`crate::runtime::session`).
    p.add(engine_backend::scene::game_hooks::GameSessionRegistration {
        origin: p.origin(),
        name: "input.player",
        begin: Some(crate::runtime::session::begin),
        possess: Some(crate::runtime::session::possess),
        end: Some(crate::runtime::end_session),
    });

    // ── Editor ──────────────────────────────────────────────────────────────

    #[cfg(feature = "editor")]
    editor::build(p);
}

/// What the editor adds: the action and mapping editors and file types, and
/// the Player Input component's "Add component" entry, as a linked editor
/// provider; and the level's player input mapping in World Settings.
#[cfg(feature = "editor")]
mod editor {
    use std::sync::Arc;

    use crate::builtin_provider::InputBuiltinProvider;

    pub(super) fn build(p: &mut pulsar_plugins::PluginBuilder) {
        p.add(plugin_manager::PluginEditorProvider {
            origin: p.origin(),
            id: InputBuiltinProvider::ID,
            create: || Arc::new(InputBuiltinProvider),
        });

        // The `.inputmap` the player controller brings to its pawn when it
        // possesses it (`runtime::session::possess` reads it from the level).
        p.add(ui_level_editor::extend::WorldSettingField {
            origin: p.origin(),
            id: "input.player_mapping",
            section: "Input",
            label: "Player Input Mapping",
            order: 0,
            key: crate::runtime::session::PLAYER_INPUT_MAPPING_KEY,
            kind: ui_level_editor::extend::WorldSettingKind::AssetPath {
                extensions: &["inputmap"],
                help: "The keys, mouse and pad the Player Character hears once the Player Controller possesses it.",
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::component::PLAYER_INPUT_CLASS_NAME;

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
