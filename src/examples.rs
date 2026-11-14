//! The plugin's `examples/` folder: varied, valid example files of each file
//! type it registers (`.inputaction`, `.inputmap`), served read-only in the
//! plugin's virtual folder and embedded from `examples_assets/`.

use std::sync::Arc;

use engine_fs::{FolderSource, InGame, MemoryFolder, PluginFolder};

/// The example files, embedded; each path is where it sits in the plugin's
/// folder.
pub const FILES: &[engine_fs::EmbeddedFile] = engine_fs::embedded_files!("examples_assets":
    "examples/README.md",
    "examples/actions/jump.inputaction",
    "examples/actions/move.inputaction",
    "examples/actions/look.inputaction",
    "examples/actions/sprint.inputaction",
    "examples/actions/charged_shot.inputaction",
    "examples/actions/dash.inputaction",
    "examples/actions/throttle.inputaction",
    "examples/actions/rapid_fire.inputaction",
    "examples/actions/fly.inputaction",
    "examples/actions/crouch.inputaction",
    "examples/actions/modifier_ctrl.inputaction",
    "examples/actions/quick_save.inputaction",
    "examples/actions/menu_navigate.inputaction",
    "examples/actions/menu_confirm.inputaction",
    "examples/actions/aim.inputaction",
    "examples/actions/throw.inputaction",
    "examples/mappings/gameplay.inputmap",
    "examples/mappings/menu.inputmap",
    "examples/mappings/vehicle.inputmap",
    "examples/mappings/free_camera.inputmap",
);

fn open(folder: PluginFolder) -> Arc<dyn FolderSource> {
    Arc::new(MemoryFolder::with_files(folder, FILES))
}

/// The plugin's one file provider.
pub(crate) fn provider(origin: &'static str) -> engine_fs::FileProvider {
    engine_fs::FileProvider {
        origin,
        open,
        // A game does not ask the plugin: it serves the packager's snapshot.
        in_game: InGame::Snapshot,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::model::behavior::TriggerKind;
    use crate::model::validate::{ActionInfo, ActionSource};
    use crate::model::{ActionAsset, Asset, AssetId, MappingAsset, Severity};

    fn text(file: &engine_fs::EmbeddedFile) -> &'static str {
        std::str::from_utf8(file.bytes).expect("example is UTF-8")
    }

    fn with_ext(ext: &'static str) -> impl Iterator<Item = &'static engine_fs::EmbeddedFile> {
        FILES.iter().filter(move |f| f.path.ends_with(ext))
    }

    fn stem(file: &engine_fs::EmbeddedFile) -> &'static str {
        let name = file.path.rsplit('/').next().unwrap();
        name.split('.').next().unwrap()
    }

    #[test]
    fn every_action_example_parses_cleanly() {
        let mut count = 0;
        for file in with_ext(".inputaction") {
            let (action, notes) =
                ActionAsset::parse(text(file)).unwrap_or_else(|e| panic!("{}: {e}", file.path));
            assert!(notes.is_empty(), "{}: {notes:?}", file.path);
            assert!(action.id.is_assigned(), "{} needs an id", file.path);
            assert!(!action.description.is_empty(), "{}", file.path);
            assert!(action.triggers.iter().all(|t| t.known().is_some()), "{}", file.path);
            assert!(action.modifiers.iter().all(|m| m.known().is_some()), "{}", file.path);
            count += 1;
        }
        assert!(count >= 10);
    }

    /// The example actions, by id, as the project library would list them
    /// (the path is the `hint` the mappings carry).
    fn actions() -> HashMap<AssetId, ActionInfo> {
        with_ext(".inputaction")
            .map(|file| {
                let action = ActionAsset::parse(text(file)).unwrap().0;
                (
                    action.id.clone(),
                    ActionInfo {
                        name: action.event_name(stem(file)),
                        value_type: action.value_type,
                        path: format!("examples/actions/{}.inputaction", stem(file)),
                        consume_input: action.consume_input,
                    },
                )
            })
            .collect()
    }

    struct Source<'a>(&'a HashMap<AssetId, ActionInfo>);

    impl ActionSource for Source<'_> {
        fn action(&self, id: &AssetId) -> Option<ActionInfo> {
            self.0.get(id).cloned()
        }
        fn name_clash(&self, name: &str, me: &AssetId) -> Option<String> {
            self.0
                .iter()
                .find(|(id, info)| *id != me && info.name == name)
                .map(|(_, info)| info.path.clone())
        }
    }

    #[test]
    fn actions_validate_without_warnings_and_are_unique() {
        let known = actions();
        assert_eq!(known.len(), with_ext(".inputaction").count(), "ids are unique");
        for file in with_ext(".inputaction") {
            let action = ActionAsset::parse(text(file)).unwrap().0;
            let issues = action.validate(stem(file), &Source(&known));
            assert!(
                issues.iter().all(|i| i.severity < Severity::Warning),
                "{}: {issues:?}",
                file.path
            );
        }
    }

    #[test]
    fn every_mapping_example_parses_and_binds_real_actions_and_keys() {
        let known = actions();
        let mut count = 0;
        for file in with_ext(".inputmap") {
            let (mapping, notes) =
                MappingAsset::parse(text(file)).unwrap_or_else(|e| panic!("{}: {e}", file.path));
            assert!(notes.is_empty(), "{}: {notes:?}", file.path);
            assert!(mapping.id.is_assigned() && mapping.entry_count() > 0, "{}", file.path);
            let issues = mapping.validate(&known);
            assert!(
                issues.iter().all(|i| i.severity < Severity::Warning),
                "{}: {issues:?}",
                file.path
            );
            count += 1;
        }
        assert!(count >= 3);
    }

    #[test]
    fn the_examples_cover_every_trigger_kind() {
        let mut kinds = std::collections::HashSet::new();
        for file in with_ext(".inputaction") {
            let action = ActionAsset::parse(text(file)).unwrap().0;
            kinds.extend(action.known_triggers().map(|t| t.kind()));
        }
        for kind in TriggerKind::ALL {
            assert!(kinds.contains(&kind), "no example uses {kind:?}");
        }
    }

    #[test]
    fn the_folder_has_a_readme() {
        assert!(FILES.iter().any(|f| f.path == "examples/README.md"));
    }
}
