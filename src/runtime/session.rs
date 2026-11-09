//! The input plugin's part in a game session (the game loop calls these
//! through `engine_backend::scene::game_hooks::GameSessionRegistration`,
//! which `crate::registrations` submits).
//!
//! Possession is what routes the local player's keys, mouse and pad to their
//! character: once the controller possesses the pawn, a
//! `PlayerInputComponent` carrying the level's mapping (its World Settings
//! gameplay roles) is attached to the pawn, and the runtime publishes
//! `input::<Action>.triggered` (and its other phases) on the pawn's own
//! event channel, where a `Self`-scoped handler in the character's Blueprint
//! hears its player. The game loop detaches it when the possession ends.

use std::path::Path;

use engine_backend::scene::game_hooks::PossessionContext;
use pulsar_events::EventHub;
use pulsar_scenedb::{Entity, World};
use pulsar_world_registry::pulsar_scene_model::attachments::NewInstance;
use pulsar_world_registry::{attach_component, ComponentPayload};

use super::component::PLAYER_INPUT_CLASS_NAME;

/// Declare the events of every input action of the project on `hub`.
///
/// A mapping registers its actions' events only once it is active, which is
/// after a Blueprint that handles `input::Jump.triggered` has linked and
/// started; declaring them up front lets any class subscribe whichever loads
/// first.
pub fn begin(hub: &EventHub, project_root: &Path) {
    let actions = super::events::declare_project_events(hub, project_root);
    tracing::debug!(actions, "Declared the project's input action events");
}

/// Put the level's input mapping to work on the possessed `pawn`: attach a
/// `PlayerInputComponent` for it. `None` when the level names no mapping:
/// not a failure, a level may possess a pawn nothing steers.
pub fn possess(
    world: &mut World,
    pawn: Entity,
    context: &PossessionContext<'_>,
) -> Result<Option<Entity>, String> {
    let Some(mapping) = context.roles.input_mapping() else {
        tracing::warn!(
            "No Player Input Mapping is set for this level: the player character receives no \
             input (World Settings, Gameplay Roles)"
        );
        return Ok(None);
    };
    // The runtime otherwise finds the project from the environment or the
    // working directory; this session knows better, and it is per world.
    super::set_world_project_root(world, Some(context.project_root.to_path_buf()));
    let instance = attach_component(
        world,
        pawn,
        NewInstance::new(PLAYER_INPUT_CLASS_NAME),
        ComponentPayload::Json(serde_json::json!({
            "enabled": true,
            "mapping": mapping,
            "priority": 0,
        })),
    )
    .map_err(|error| {
        format!("Player input mapping '{mapping}' could not be put to work on the pawn: {error}")
    })?;
    tracing::info!(%mapping, "Player input routed to the possessed pawn");
    Ok(Some(instance))
}
