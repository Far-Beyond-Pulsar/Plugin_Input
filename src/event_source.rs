//! The events a player input component offers scripts: the actions of the
//! mapping it names, as one `On Input Action` node.
//!
//! A `PlayerInputComponent` names an `.inputmap`; the mapping's source
//! (`engine_backend::AssetEventSource`, registered for the `inputmap` kind)
//! answers with one **family**, `input.action`, instead of five events per
//! action. The node's parameters pick an action (by its asset id, so a
//! rename of the action's identifier never breaks it) and the phases to hear;
//! the value pin is typed from the action: `bool` for a digital action, `f64`
//! for an axis, `Vec2` and `Vec3` for the vectors, built by the compiler from
//! the runtime's `x`, `y`, `z` fields (the wire format is unchanged).
//!
//! The runtime raises `input::<Action>.<phase>` events as it always did
//! (`crate::runtime::events`); this module only says which of them a node
//! hears and how its pins are made.

use std::collections::BTreeMap;

use engine_backend::{
    AssetEventSource, AssetPathSlot, ComponentEventCatalog, EventFamily, FamilyOption, FamilyParam,
    ParamKind, ParamValues, ResolvedFamily, ResolvedOutput, SourceCx,
};
use pulsar_reflection::InputMapAssetKind;

use crate::library::{relative_path, Library};
use crate::runtime::component::PLAYER_INPUT_CLASS_NAME;
use crate::runtime::events::action_descriptor;
use crate::runtime::system::Phase;

/// The family's stable id; its node type is `event::family::input.action`.
pub const FAMILY_ID: &str = "input.action";

/// The `inputmap` kind's source.
pub(crate) fn source(origin: &'static str) -> AssetEventSource {
    AssetEventSource::for_kind::<InputMapAssetKind>(origin, declare)
}

/// `PlayerInputComponent.mapping` is a project-relative path, not a typed
/// reference, so reflection cannot see that it names a mapping: say so.
pub(crate) fn mapping_slot(origin: &'static str) -> AssetPathSlot {
    AssetPathSlot::for_kind::<InputMapAssetKind>(origin, PLAYER_INPUT_CLASS_NAME, "mapping")
}

fn declare(cx: &mut SourceCx<'_>, reference: &str) -> ComponentEventCatalog {
    // The mapping names its actions by id, and the actions are other files:
    // watch both kinds rather than track each file.
    cx.watch_extension("inputmap");
    cx.watch_extension("inputaction");
    let mut catalog = ComponentEventCatalog::default();
    let Some(path) = cx.resolve("inputmap", reference) else {
        // A path that is not there yet, or an id the index does not know:
        // the component names no mapping the project has.
        catalog.diagnostics.push((
            reference.into(),
            "the input mapping is not in this project".into(),
        ));
        return catalog;
    };
    let library = Library::scan(cx.root());
    let relative = relative_path(cx.root(), &path);
    let Some(mapping) = library.mapping_at_relative(&relative) else {
        catalog
            .diagnostics
            .push((path, "the file is not a readable input mapping".into()));
        return catalog;
    };

    // The actions the mapping binds, once each, by name.
    let mut bound: BTreeMap<String, (String, &'static str)> = BTreeMap::new();
    for (_, entry) in mapping.asset.entries() {
        let Some(action) = entry.action.as_ref() else {
            continue;
        };
        if let Some(found) = library.action_entry(&action.id) {
            bound.insert(
                found.event_name(),
                (
                    action.id.as_str().to_owned(),
                    found.asset.value_type.short(),
                ),
            );
        }
    }
    if bound.is_empty() {
        return catalog;
    }
    let options: Vec<FamilyOption> = bound
        .iter()
        .map(|(name, (id, shape))| FamilyOption {
            id: id.clone(),
            label: name.clone(),
            detail: (*shape).to_owned(),
        })
        .collect();
    let first = options.first().map(|o| o.id.clone()).unwrap_or_default();
    let events = bound
        .keys()
        .flat_map(|name| Phase::ALL.map(|phase| action_descriptor(name, phase)))
        .filter_map(Result::ok)
        .collect();
    catalog.families.push(EventFamily {
        id: FAMILY_ID.to_owned(),
        label: "Input Action".to_owned(),
        group: "Input Action".to_owned(),
        description: "Runs when an input action of this component's mapping changes phase."
            .to_owned(),
        documentation: "Pick an action of the mapping and the phases to hear; each phase is \
                        an execution output. The value is typed from the action: a bool for a \
                        digital action, a number for an axis, a Vec2 or Vec3 for a vector. By \
                        default only this object's own player is heard (scope `self`)."
            .to_owned(),
        params: vec![
            FamilyParam {
                name: "action".to_owned(),
                label: "Action".to_owned(),
                kind: ParamKind::Choice,
                options,
                default: first,
            },
            FamilyParam {
                name: "phases".to_owned(),
                label: "Fires on".to_owned(),
                kind: ParamKind::Multi,
                options: Phase::ALL
                    .iter()
                    .map(|phase| FamilyOption {
                        id: phase.word().to_owned(),
                        label: title(phase.word()),
                        detail: String::new(),
                    })
                    .collect(),
                // Every phase is offered as an exec output; the compiler
                // subscribes only the ones something is wired to.
                default: Phase::ALL.map(|phase| phase.word()).join(","),
            },
        ],
        events,
        resolve,
    });
    catalog
}

fn title(word: &str) -> String {
    let mut chars = word.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// What a node's parameters mean: an execution output per chosen phase and the
/// typed value pins of the chosen action.
pub fn resolve(family: &EventFamily, values: &ParamValues) -> Result<ResolvedFamily, String> {
    let action = values.get("action").map(String::as_str).unwrap_or_default();
    let option = family
        .param("action")
        .and_then(|param| param.options.iter().find(|option| option.id == action))
        .ok_or_else(|| "the action is no longer bound by this component's mapping".to_owned())?;

    let chosen: Vec<&str> = values
        .get("phases")
        .map(|phases| phases.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let arms: Vec<(String, String)> = Phase::ALL
        .iter()
        .filter(|phase| chosen.contains(&phase.word()))
        .map(|phase| {
            (
                title(phase.word()),
                crate::runtime::events::qualified(&option.label, *phase),
            )
        })
        .collect();
    if arms.is_empty() {
        return Err("no phase is selected".to_owned());
    }

    let value = match option.detail.as_str() {
        "Bool" => ("Pressed", "bool", "bool:value"),
        "2D" => ("Value", "Vec2", "vec2:x,y"),
        "3D" => ("Value", "Vec3", "vec3:x,y,z"),
        _ => ("Value", "f64", "f64:value"),
    };
    let output = |pin: &str, type_name: &str, lowering: &str| ResolvedOutput {
        pin: pin.to_owned(),
        type_name: type_name.to_owned(),
        lowering: lowering.to_owned(),
    };
    Ok(ResolvedFamily {
        arms,
        outputs: vec![
            output(value.0, value.1, value.2),
            output("Elapsed", "f64", "f64:elapsed"),
            output("TriggeredFor", "f64", "f64:triggered_for"),
            output("Entity", "Entity", "entity:entity"),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ActionAsset, Asset, MappingAsset, ValueType};
    use crate::model::{ActionRef, Entry};
    use engine_backend::EventCache;

    fn project(tag: &str) -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("input_event_source_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("input")).unwrap();
        root
    }

    fn write_action(root: &std::path::Path, name: &str, shape: ValueType) -> ActionRef {
        let mut action = ActionAsset::new();
        action.identifier = name.to_owned();
        action.value_type = shape;
        let path = format!("input/{name}.inputaction");
        std::fs::write(root.join(&path), Asset::to_text(&action)).unwrap();
        ActionRef::new(action.id.clone(), path)
    }

    fn write_mapping(root: &std::path::Path, name: &str, actions: &[ActionRef]) -> String {
        let mut mapping = MappingAsset::new();
        for (index, action) in actions.iter().enumerate() {
            mapping.categories[0].entries.push(Entry {
                id: index as u32 + 1,
                key: "Keyboard.KeyW".into(),
                action: Some(action.clone()),
                enabled: true,
                ..Entry::default()
            });
        }
        let path = format!("input/{name}.inputmap");
        std::fs::write(root.join(&path), Asset::to_text(&mapping)).unwrap();
        path
    }

    fn catalog(root: &std::path::Path, mapping: &str) -> std::sync::Arc<ComponentEventCatalog> {
        let cache = EventCache::new();
        cache.catalog(&source("test"), root, mapping)
    }

    #[test]
    fn a_mapping_answers_with_one_family_of_the_actions_it_binds() {
        let root = project("family");
        let jump = write_action(&root, "Jump", ValueType::Bool);
        let moving = write_action(&root, "Move", ValueType::Axis2D);
        write_action(&root, "Unbound", ValueType::Bool);
        let mapping = write_mapping(&root, "Game", &[jump, moving]);

        let found = catalog(&root, &mapping);
        assert_eq!(found.families.len(), 1, "one node, however many actions");
        let family = &found.families[0];
        assert_eq!(family.id, FAMILY_ID);
        let actions: Vec<_> = family
            .param("action")
            .unwrap()
            .options
            .iter()
            .map(|o| (o.label.as_str(), o.detail.as_str()))
            .collect();
        assert_eq!(
            actions,
            [("Jump", "Bool"), ("Move", "2D")],
            "only bound actions"
        );
        assert_eq!(family.events.len(), 10, "five phases of each");
        assert!(found.diagnostics.is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_value_pin_is_typed_from_the_action() {
        let root = project("types");
        let actions = [
            write_action(&root, "Jump", ValueType::Bool),
            write_action(&root, "Throttle", ValueType::Axis1D),
            write_action(&root, "Look", ValueType::Axis2D),
            write_action(&root, "Fly", ValueType::Axis3D),
        ];
        let mapping = write_mapping(&root, "Game", &actions);
        let found = catalog(&root, &mapping);
        let family = &found.families[0];
        let shape = |name: &str| {
            let mut values = family.defaults();
            let id = family
                .param("action")
                .unwrap()
                .options
                .iter()
                .find(|o| o.label == name)
                .unwrap()
                .id
                .clone();
            values.insert("action".into(), id);
            let resolved = resolve(family, &values).unwrap();
            let first = &resolved.outputs[0];
            (
                first.pin.clone(),
                first.type_name.clone(),
                first.lowering.clone(),
            )
        };
        assert_eq!(
            shape("Jump"),
            ("Pressed".into(), "bool".into(), "bool:value".into())
        );
        assert_eq!(
            shape("Throttle"),
            ("Value".into(), "f64".into(), "f64:value".into())
        );
        assert_eq!(
            shape("Look"),
            ("Value".into(), "Vec2".into(), "vec2:x,y".into())
        );
        assert_eq!(
            shape("Fly"),
            ("Value".into(), "Vec3".into(), "vec3:x,y,z".into())
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn each_chosen_phase_is_an_execution_output_with_its_own_event() {
        let root = project("phases");
        let jump = write_action(&root, "Jump", ValueType::Bool);
        let mapping = write_mapping(&root, "Game", &[jump]);
        let family = catalog(&root, &mapping).families[0].clone();
        let mut values = family.defaults();
        let names: Vec<_> = resolve(&family, &values)
            .unwrap()
            .arms
            .into_iter()
            .map(|(pin, _)| pin)
            .collect();
        assert_eq!(
            names,
            ["Started", "Ongoing", "Triggered", "Completed", "Canceled"],
            "every phase is an output by default; the compiler listens only to wired ones"
        );
        values.insert("phases".into(), "started, completed".into());
        assert_eq!(
            resolve(&family, &values).unwrap().arms,
            [
                ("Started".to_owned(), "input::Jump.started".to_owned()),
                ("Completed".to_owned(), "input::Jump.completed".to_owned())
            ],
            "in phase order, whatever order they were chosen in"
        );
        values.insert("phases".into(), String::new());
        assert!(resolve(&family, &values).unwrap_err().contains("no phase"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_action_the_mapping_no_longer_binds_is_an_error_not_a_guess() {
        let root = project("gone");
        let jump = write_action(&root, "Jump", ValueType::Bool);
        let mapping = write_mapping(&root, "Game", &[jump]);
        let family = catalog(&root, &mapping).families[0].clone();
        let mut values = family.defaults();
        values.insert(
            "action".into(),
            "00000000-0000-4000-8000-000000000000".into(),
        );
        assert!(resolve(&family, &values)
            .unwrap_err()
            .contains("no longer bound"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_mapping_that_is_not_there_is_a_diagnostic_not_a_panic() {
        let root = project("missing");
        let found = catalog(&root, "input/Nope.inputmap");
        assert!(found.families.is_empty());
        assert_eq!(found.diagnostics.len(), 1);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn the_component_property_is_a_mapping_path_slot() {
        let slot = mapping_slot("test");
        assert_eq!(
            (slot.class_name, slot.property, slot.kind),
            ("PlayerInputComponent", "mapping", "inputmap")
        );
    }
}
