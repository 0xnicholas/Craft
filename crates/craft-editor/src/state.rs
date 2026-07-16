use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use craft_kernel::Scene;

use crate::engine::EditorEngine;
pub use crate::error::EditorError;

pub struct EditorState {
    pub project: Option<ProjectState>,
    pub scene: Option<SceneState>,
    pub engine: EditorEngine,
    pub panels: PanelsState,
    pub ui: UiState,
    pub errors: Vec<EditorError>,
    pub lsp: LspManager,
    pub dock_kind: DockKind,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            project: None,
            scene: None,
            engine: EditorEngine::default(),
            panels: PanelsState::default(),
            ui: UiState::default(),
            errors: Vec::new(),
            lsp: LspManager,
            dock_kind: DockKind::default(),
        }
    }
}

pub struct ProjectState {
    pub root: PathBuf,
}

pub struct SceneState {
    pub path: PathBuf,
    pub def: Scene,
    pub last_saved_hash: u64,
    pub file_watcher_epoch: u64,
}

impl SceneState {
    pub fn is_dirty(&self) -> bool {
        craft_kernel::hash_scene_state(&self.def) != self.last_saved_hash
    }
}

impl EditorState {
    pub fn save_dirty(&mut self) -> Result<(), EditorError> {
        if let Some(scene) = &mut self.scene {
            if scene.is_dirty() {
                let path = scene.path.clone();
                crate::io::save_scene(&path, &scene.def)?;
                scene.last_saved_hash = craft_kernel::hash_scene_state(&scene.def);
            }
        }
        Ok(())
    }

    pub fn open_scene(&mut self, path: &std::path::Path) -> Result<(), EditorError> {
        self.engine
            .load_scene_file(path)
            .map_err(|e| EditorError::Other {
                message: e.to_string(),
            })?;
        let registry = craft_kernel::NodeRegistry::new();
        let def = crate::io::load_scene(path, &registry)?;
        let last_saved_hash = craft_kernel::hash_scene_state(&def);
        self.scene = Some(SceneState {
            path: path.to_path_buf(),
            def,
            last_saved_hash,
            file_watcher_epoch: 0,
        });
        Ok(())
    }
}

#[derive(Default)]
pub struct UiState {
    pub status_message: String,
    pub file_change_pending: Option<PathBuf>,
}

pub struct PanelsState {
    pub scene_tree: SceneTreeState,
    pub inspector: InspectorState,
    pub file_browser: FileBrowserState,
    pub terminal_preview: TerminalPreviewState,
    pub behavior_editor: BehaviorEditorStub,
    pub lua_editor: LuaEditorStub,
    pub agent_panel: AgentPanelStub,
}

impl Default for PanelsState {
    fn default() -> Self {
        Self {
            scene_tree: SceneTreeState {
                selected_node: None,
                expanded_nodes: HashSet::new(),
                filter_text: String::new(),
            },
            inspector: InspectorState::default(),
            file_browser: FileBrowserState::default(),
            terminal_preview: TerminalPreviewState::default(),
            behavior_editor: BehaviorEditorStub,
            lua_editor: LuaEditorStub,
            agent_panel: AgentPanelStub,
        }
    }
}

pub struct SceneTreeState {
    pub selected_node: Option<String>,
    pub expanded_nodes: HashSet<String>,
    pub filter_text: String,
}

#[derive(Default)]
pub struct InspectorState {
    pub search_text: String,
    pub add_component_menu_open: bool,
}

#[derive(Default)]
pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: BTreeMap<PathBuf, FileEntry>,
    pub filter: String,
}

#[derive(Default)]
pub struct TerminalPreviewState {
    pub is_running: bool,
}

#[derive(Default)]
pub struct BehaviorEditorStub;
#[derive(Default)]
pub struct LuaEditorStub;
#[derive(Default)]
pub struct AgentPanelStub;

#[derive(Default)]
pub struct LspManager;

#[derive(Default, Clone, Copy)]
pub enum DockKind {
    #[default]
    DefaultFourPanel,
}

pub struct FileEntry {
    pub name: String,
    pub kind: FileKind,
}

#[derive(Clone, Copy)]
pub enum FileKind {
    Directory,
    Scene,
    Behavior,
    Lua,
    Resource,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_kernel::{ComponentKind, ComponentValue, Node, SCENE_KIND};

    #[test]
    fn dirty_flag_flips_on_def_change() {
        let mut scene_state = make_blank_scene_state();
        let initial_dirty = scene_state.is_dirty();
        assert!(!initial_dirty);

        scene_state.def.nodes.push(Node {
            id: "x".to_string(),
            type_name: "Player".to_string(),
            parent: None,
            components: BTreeMap::new(),
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        });
        assert!(scene_state.is_dirty());

        scene_state.last_saved_hash = craft_kernel::hash_scene_state(&scene_state.def);
        assert!(!scene_state.is_dirty());
    }

    #[test]
    fn hash_ignores_destroyed_flag() {
        let mut alive = make_blank_scene_state();
        alive.def.nodes.push(Node {
            id: "x".to_string(),
            type_name: "Player".to_string(),
            parent: None,
            components: BTreeMap::new(),
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        });
        let mut dead_def = alive.def.clone();
        dead_def.nodes[0].destroyed = true;
        assert_eq!(
            craft_kernel::hash_scene_state(&alive.def),
            craft_kernel::hash_scene_state(&dead_def),
            "destroyed flag is #[serde(skip)] and must not affect the saved hash"
        );
    }

    #[test]
    fn component_value_change_marks_dirty() {
        let mut scene_state = make_blank_scene_state();
        scene_state.def.nodes.push(Node {
            id: "x".to_string(),
            type_name: "Player".to_string(),
            parent: None,
            components: BTreeMap::from([(
                "hp".to_string(),
                craft_kernel::Component {
                    value: ComponentValue::Int(1),
                    kind: ComponentKind::Regular,
                },
            )]),
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        });
        scene_state.last_saved_hash = craft_kernel::hash_scene_state(&scene_state.def);
        assert!(!scene_state.is_dirty());
        if let Some(n) = scene_state.def.nodes.first_mut() {
            if let Some(c) = n.components.get_mut("hp") {
                c.value = ComponentValue::Int(2);
            }
        }
        assert!(scene_state.is_dirty());
    }

    fn make_blank_scene_state() -> SceneState {
        let def = Scene {
            kind: SCENE_KIND.to_string(),
            name: "blank".to_string(),
            nodes: Vec::new(),
            spawn_counter: 0,
        };
        let last_saved_hash = craft_kernel::hash_scene_state(&def);
        SceneState {
            path: PathBuf::from("/tmp/scene.json"),
            def,
            last_saved_hash,
            file_watcher_epoch: 0,
        }
    }
}
