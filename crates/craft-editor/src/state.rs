use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;

use craft_kernel::Scene;
use craft_kernel::hot_reload::SceneDiff;

use crate::engine::EditorEngine;
pub use crate::error::EditorError;
use crate::json_path::{CompletionPopup, SchemaError};

pub struct EditorState {
    pub project: Option<ProjectState>,
    pub scene: Option<SceneState>,
    pub engine: EditorEngine,
    pub panels: PanelsState,
    pub ui: UiState,
    pub errors: Vec<EditorError>,
    pub lsp: LspManager,
    pub dock_kind: DockKind,
    pub lua_editor: LuaEditorState,
    pub standalone_behavior: BehaviorEditorState,
    pub agent_client: Option<crate::agent::AgentClient>,
    pub agent_tool_registry: crate::agent::tools::ToolRegistry,
    pub agent_rx: Option<std::sync::mpsc::Receiver<crate::agent::AgentStreamEvent>>,
    pub agent_handle: Option<std::thread::JoinHandle<()>>,
    pub undo_redo: crate::undo::UndoRedo,
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
            lua_editor: LuaEditorState::default(),
            standalone_behavior: BehaviorEditorState::default(),
            agent_client: None,
            agent_tool_registry: crate::agent::tools::ToolRegistry::new(),
            agent_rx: None,
            agent_handle: None,
            undo_redo: crate::undo::UndoRedo::new(100),
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
        let def = self
            .engine
            .engine
            .scene()
            .cloned()
            .ok_or_else(|| EditorError::Other {
                message: "engine has no scene after load".into(),
            })?;
        let last_saved_hash = craft_kernel::hash_scene_state(&def);
        self.scene = Some(SceneState {
            path: path.to_path_buf(),
            def,
            last_saved_hash,
            file_watcher_epoch: 0,
        });
        if self.agent_client.is_none() {
            if let Some(ref project) = self.project {
                let config = crate::agent::config::AgentConfig::load(&project.root);
                self.agent_client = Some(crate::agent::AgentClient::new(config));
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct UiState {
    pub status_message: String,
    pub file_change_pending: Option<FileChangePending>,
}

#[derive(Debug, Clone)]
pub struct FileChangePending {
    pub path: PathBuf,
    pub kind: FileChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileChangeKind {
    SceneJson,
    Lua,
    Behavior,
}

pub struct PanelsState {
    pub scene_tree: SceneTreeState,
    pub inspector: InspectorState,
    pub file_browser: FileBrowserState,
    pub terminal_preview: TerminalPreviewState,
    pub behavior_editor: BehaviorEditorStub,
    pub lua_editor: LuaEditorStub,
    pub agent_panel: AgentPanelState,
}

impl Default for PanelsState {
    fn default() -> Self {
        Self {
            scene_tree: SceneTreeState {
                selected_node: None,
                expanded_nodes: HashSet::new(),
                filter_text: String::new(),
                context_menu: None,
            },
            inspector: InspectorState::default(),
            file_browser: FileBrowserState::default(),
            terminal_preview: TerminalPreviewState::default(),
            behavior_editor: BehaviorEditorStub,
            lua_editor: LuaEditorStub,
            agent_panel: AgentPanelState::default(),
        }
    }
}

pub struct SceneTreeState {
    pub selected_node: Option<String>,
    pub expanded_nodes: HashSet<String>,
    pub filter_text: String,
    pub context_menu: Option<(String, egui::Pos2)>,
}

#[derive(Default)]
pub struct InspectorState {
    pub search_text: String,
    pub add_component_menu_open: bool,
    pub expanded_behaviors: HashSet<(String, usize)>,
    pub behavior_edits: HashMap<(String, usize), BehaviorEditState>,
    pub last_validation_ms: u64,
}

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: BTreeMap<PathBuf, FileEntry>,
    pub filter: String,
    pub context_menu: Option<(PathBuf, bool, egui::Pos2)>,
}

impl Default for FileBrowserState {
    fn default() -> Self {
        Self {
            current_dir: PathBuf::new(),
            entries: BTreeMap::new(),
            filter: String::new(),
            context_menu: None,
        }
    }
}

#[derive(Default)]
pub struct TerminalPreviewState {
    pub is_running: bool,
}

#[derive(Default)]
pub struct AgentPanelState {
    pub messages: Vec<AgentMessage>,
    pub input: String,
    pub is_streaming: bool,
    pub streaming_text: String,
    pub suggestion_counter: u32,
    pub diff_preview: Option<usize>,
    pub tool_round: u32,
}

pub enum AgentMessage {
    User {
        text: String,
    },
    Agent {
        text: String,
        suggestions: Vec<AgentSuggestion>,
    },
    System {
        text: String,
    },
}

pub struct AgentSuggestion {
    pub id: String,
    pub description: String,
    pub diff: SceneDiff,
    pub status: SuggestionStatus,
}

#[derive(Clone, Debug)]
pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Failed { reason: String },
}

#[derive(Default)]
pub struct BehaviorEditorStub;
#[derive(Default)]
pub struct LuaEditorStub;

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

pub struct LuaEditorState {
    pub current_path: Option<PathBuf>,
    pub buffer: String,
    pub dirty: bool,
    pub lsp: Option<crate::lua_lsp::LspClient>,
    pub fallback_mode: bool,
    pub completion: Option<CompletionPopup>,
    pub last_validation_ms: u64,
    pub opened_uris: HashSet<String>,
    pub pending_completions: HashMap<i64, serde_json::Value>,
    pub next_completion_id: i64,
    pub diagnostics: Vec<crate::lua_lsp::LspDiagnostic>,
    pub completion_requested: bool,
    pub show_completion_popup: bool,
}

impl Default for LuaEditorState {
    fn default() -> Self {
        Self {
            current_path: None,
            buffer: String::new(),
            dirty: false,
            lsp: None,
            fallback_mode: false,
            completion: None,
            last_validation_ms: 0,
            opened_uris: HashSet::new(),
            pending_completions: HashMap::new(),
            next_completion_id: 1,
            diagnostics: Vec::new(),
            completion_requested: false,
            show_completion_popup: false,
        }
    }
}

#[derive(Default)]
pub struct BehaviorEditorState {
    pub path: Option<PathBuf>,
    pub buffer: String,
    pub errors: Vec<SchemaError>,
    pub completion: Option<CompletionPopup>,
    pub dirty: bool,
}

pub struct BehaviorEditState {
    pub node_id: String,
    pub behavior_idx: usize,
    pub buffer: String,
    pub parsed: Option<craft_kernel::Behavior>,
    pub errors: Vec<SchemaError>,
    pub completion: Option<CompletionPopup>,
    pub dirty: bool,
}

pub fn json_path_lsp() -> crate::json_path::JsonPathLsp {
    crate::json_path::JsonPathLsp::new(craft_schema::get_full_schema())
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
