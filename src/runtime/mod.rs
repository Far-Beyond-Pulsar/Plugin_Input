//! The input runtime: devices in, mappings evaluated, actions out on the
//! Gamma event bus.
//!
//! | Module | What |
//! |---|---|
//! | [`raw`] | what every key reads right now |
//! | [`modify`], [`trigger`] | modifiers and trigger state machines |
//! | [`system`] | the active mappings and the per-frame evaluation |
//! | [`events`] | the `input` event namespace, and listening to the devices' events |
//! | [`gamepad`] | controllers, read on their own thread |
//! | [`assets`] | finding the project's mappings and noticing edits |
//! | [`driver`] | one session: the above, wired to an event hub |
//! | [`component`] | `PlayerInputComponent` |
//! | [`session`] | the plugin's part in a game session and a possession |
//!
//! Only `run` here knows about the world (`crate::registrations` registers
//! it as the Player Input class's system); everything else is testable without one.

pub mod assets;
pub mod component;
pub mod driver;
pub mod events;
pub mod gamepad;
pub mod modify;
pub mod raw;
pub mod session;
pub mod system;
pub mod trigger;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use pulsar_events::EventHub;
use pulsar_scenedb::World;
use pulsar_world_registry::pulsar_scene_model::ComponentOwner;
use pulsar_world_registry::SystemContext;

use component::PlayerInputComponent;
use driver::{Driver, Wanted};

/// A session nobody ticked for this long is shut down.
const IDLE_SESSION: Duration = Duration::from_secs(10);

/// One driver per `World`, keyed by the world's address: tick registrations
/// are plain function pointers and carry no session state.
static SESSIONS: LazyLock<Mutex<HashMap<usize, Session>>> = LazyLock::new(Mutex::default);

struct Session {
    driver: Driver,
    last_tick: Instant,
}

/// The project each world belongs to, by the world's address like `SESSIONS`.
static ROOTS: LazyLock<Mutex<HashMap<usize, PathBuf>>> = LazyLock::new(Mutex::default);

/// Tell the runtime which project `world` belongs to, for a host that knows:
/// its mappings are then found there, whatever the environment or working
/// directory say, and two worlds in one process may be in two projects.
/// `None` goes back to finding the project. Takes effect at the next frame.
pub fn set_world_project_root(world: &World, root: Option<PathBuf>) {
    let key = world as *const World as usize;
    let mut roots = ROOTS.lock().unwrap_or_else(|p| p.into_inner());
    match root {
        Some(root) => roots.insert(key, root),
        None => roots.remove(&key),
    };
}

/// End the input session of `world`: its devices, its mappings and its
/// listeners on the event hub are released. A host calls it when the game
/// over `world` ends, so a stopped session leaves nothing on its hub (the
/// next frame of a world that plays again starts a fresh session).
pub fn end_session(world: &World) {
    let key = world as *const World as usize;
    let session = SESSIONS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(&key);
    // Stopping the controller thread waits for it: not under the lock.
    drop(session);
    ROOTS.lock().unwrap_or_else(|p| p.into_inner()).remove(&key);
}

/// One frame of input over the whole `World`: every enabled
/// `PlayerInputComponent`'s mapping evaluated and its actions published. The
/// `pulsar.input.devices` system of the Player Input class.
pub(crate) fn run(ctx: &mut SystemContext<'_>) {
    profiling::profile_scope!("Input::Tick");
    let (events, delta_seconds) = (ctx.events, ctx.delta_seconds);
    let wanted: Vec<Wanted> = ctx
        .world
        .query::<(&PlayerInputComponent, &ComponentOwner)>()
        .filter(|(_, (component, link))| {
            component.enabled && link.is_enabled() && !component.mapping.trim().is_empty()
        })
        .map(|(instance, (component, link))| Wanted {
            holder: instance.bits(),
            owner: link.entity().bits(),
            mapping: component.mapping.trim().to_string(),
            priority: component.priority,
        })
        .collect();
    let key = &*ctx.world as *const World as usize;
    let root = ROOTS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(&key)
        .cloned();
    let mut sessions = SESSIONS.lock().unwrap_or_else(|p| p.into_inner());
    let now = Instant::now();
    sessions.retain(|&other, session| other == key || now - session.last_tick < IDLE_SESSION);
    let session = sessions.entry(key).or_insert_with(|| Session {
        driver: Driver::new(None, cfg!(feature = "gamepad")),
        last_tick: now,
    });
    session.last_tick = now;
    if let Some(root) = root {
        session.driver.use_project_root(root);
    }
    session.driver.frame(events, delta_seconds, &wanted);
}
