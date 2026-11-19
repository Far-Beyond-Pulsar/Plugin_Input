//! The component that puts a mapping to work on an object.

use engine_class_derive::{engine_class, world_component};

/// The class name scripts and level files use.
pub const PLAYER_INPUT_CLASS_NAME: &str = "PlayerInputComponent";

/// The project-relative path of an `.inputmap` asset: a transparent newtype
/// over the string saved scenes already hold, so the property gets the shared
/// asset picker (restricted to `.inputmap` files) instead of a text box, as a
/// mesh component's `MeshAssetPath` does.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct InputMapPath(pub String);

impl InputMapPath {
    pub fn new(path: impl Into<String>) -> Self {
        Self(path.into())
    }
}

impl std::ops::Deref for InputMapPath {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

fn serialize_input_map_path_json(value: &InputMapPath) -> pulsar_reflection::ReflectResult<serde_json::Value> {
    Ok(serde_json::Value::String(value.0.clone()))
}

fn deserialize_input_map_path_json(
    value: serde_json::Value,
) -> pulsar_reflection::ReflectResult<InputMapPath> {
    match value {
        serde_json::Value::String(text) => Ok(InputMapPath(text)),
        serde_json::Value::Null => Ok(InputMapPath::default()),
        other => Err(pulsar_reflection::ReflectError::TypeMismatch {
            expected: "InputMapPath",
            found: format!("{other:?}"),
        }),
    }
}

#[cfg(feature = "editor")]
impl ui_common::asset_path_editor::AssetPathValue for InputMapPath {
    fn from_path(path: String) -> Self {
        Self(path)
    }

    fn path(&self) -> &str {
        &self.0
    }
}

#[cfg(feature = "editor")]
fn input_map_path_editor(
    args: &pulsar_reflection::PropertyEditorArgs<'_>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> pulsar_reflection::BoundPropertyEditor {
    ui_common::asset_path_editor::asset_path_editor::<InputMapPath>(
        args,
        "No mapping",
        &["inputmap"],
        window,
        cx,
    )
}

#[cfg_attr(
    feature = "editor",
    pulsar_reflection::pulsar_type(
        serialize_json_with = serialize_input_map_path_json,
        deserialize_json_with = deserialize_input_map_path_json,
        editor = input_map_path_editor
    )
)]
#[cfg_attr(
    not(feature = "editor"),
    pulsar_reflection::pulsar_type(
        serialize_json_with = serialize_input_map_path_json,
        deserialize_json_with = deserialize_input_map_path_json
    )
)]
#[allow(dead_code)]
type RegisteredInputMapPath = InputMapPath;

/// Makes an input mapping active while this object exists. The actions the
/// mapping defines are published on this object's event channel
/// (`input::Jump.triggered`, ...), so a script attached to the same object
/// hears its own player's input and nobody else's.
#[engine_class(category = "Input", clone, debug, serialize, deserialize)]
#[category("Input", category_color = "#4ECDC4")]
#[serde(default)]
pub struct PlayerInputComponent {
    /// Turn the mapping off without removing the component.
    #[property(category = "Input")]
    pub enabled: bool,
    /// Project-relative path of an `.inputmap` asset.
    #[property(category = "Input")]
    pub mapping: InputMapPath,
    /// Added to the mapping's own priority; higher sees input first.
    #[property(min = -1000.0, max = 1000.0, step = 1.0, category = "Input")]
    pub priority: i32,
}

impl Default for PlayerInputComponent {
    fn default() -> Self {
        Self {
            enabled: true,
            mapping: InputMapPath::default(),
            priority: 0,
        }
    }
}

/// A `World` class; `crate::registrations` registers it.
#[world_component]
impl PlayerInputComponent {}
