//! The plugin's editor provider: its file types and editors, offered to
//! the editor while the plugin is enabled (`plugin_manager::PluginEditorProvider`).

use gpui::{App, Window};
use plugin_editor_api::*;
use plugin_manager::{BuiltinEditorProvider, EditorContext};
use std::path::PathBuf;
use std::sync::Arc;
use ui::dock::PanelView;

/// Wraps the input_plugin crate as a built-in provider: `.inputaction` and
/// `.inputmap` with their editors, and the Player Input component. Linking the
/// crate is what registers the component and the per-frame input runtime that
/// publishes `input::` events.
pub struct InputBuiltinProvider;

impl BuiltinEditorProvider for InputBuiltinProvider {
    fn provider_id(&self) -> &str {
        "com.pulsar.input"
    }

    fn file_types(&self) -> Vec<FileTypeDefinition> {
        crate::InputPlugin.file_types()
    }

    fn editors(&self) -> Vec<EditorMetadata> {
        crate::InputPlugin.editors()
    }

    fn can_handle(&self, editor_id: &EditorId) -> bool {
        matches!(
            editor_id.as_str(),
            crate::ACTION_EDITOR_ID | crate::MAPPING_EDITOR_ID
        )
    }

    fn component_definitions(&self) -> Vec<ComponentDefinition> {
        crate::InputPlugin.component_definitions()
    }

    fn create_editor(
        &self,
        file_path: PathBuf,
        editor_context: &EditorContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Arc<dyn PanelView>, PluginError> {
        // Without an editor id, the file's type says which editor it opens in.
        let Some(editor_id) = crate::InputPlugin::editor_for_path(&file_path) else {
            return Err(PluginError::Other {
                message: format!("the input plugin has no editor for {}", file_path.display()),
            });
        };
        self.create_editor_for(&editor_id, file_path, editor_context, window, cx)
    }

    fn create_editor_for(
        &self,
        editor_id: &EditorId,
        file_path: PathBuf,
        editor_context: &EditorContext,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Arc<dyn PanelView>, PluginError> {
        crate::InputPlugin::create_editor(
            editor_id,
            file_path,
            Some(&editor_context.services.undo),
            window,
            cx,
        )
    }
}

plugin_manager::inventory::submit! {
    plugin_manager::PluginEditorProvider {
        origin: pulsar_plugins::origin!(),
        create: || Arc::new(InputBuiltinProvider),
    }
}
