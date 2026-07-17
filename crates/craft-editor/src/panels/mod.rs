use std::path::PathBuf;

use egui::Ui;

use crate::state::{DockKind, EditorError, EditorState};

const SYSTEM_PROMPT: &str = "You are Craft's AI copilot. You help users build game scenes by inspecting the scene, analyzing issues, and proposing structured changes. Use tools to gather information. When proposing changes, respond with a JSON object containing 'reply' (your explanation) and 'diffs' (an array of SceneDiff objects). Do not read files outside the project. Do not modify files directly — all changes must be reviewed by the human.";

#[derive(Debug, Clone)]
pub enum PanelAction {
    OpenScene(PathBuf),
    SaveScene,
    RunScene,
    StopScene,
    StepTick,
    ReloadScene,
    SetStatus(String),
    ReportError(EditorError),
    RequestQuit,
    OpenBehaviorFile(PathBuf),
    OpenLuaFile(PathBuf),
    AgentSendMessage(String),
}

pub trait Panel {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn show(&mut self, ui: &mut Ui, state: &mut EditorState) -> Vec<PanelAction>;
}

pub mod agent_panel;
pub mod behavior_editor;
pub mod file_browser;
pub mod inspector;
pub mod lua_editor;
pub mod scene_tree;
pub mod terminal_preview;

pub use agent_panel::AgentPanel;
pub use behavior_editor::BehaviorEditorPanel;
pub use file_browser::FileBrowserPanel;
pub use inspector::InspectorPanel;
pub use lua_editor::LuaEditorPanel;
pub use scene_tree::SceneTreePanel;
pub use terminal_preview::TerminalPreviewPanel;

pub fn dispatch(actions: Vec<PanelAction>, state: &mut EditorState) {
    for action in actions {
        match action {
            PanelAction::SaveScene => {
                let _ = state.save_dirty();
            }
            PanelAction::SetStatus(msg) => state.ui.status_message = msg,
            PanelAction::ReportError(err) => state.errors.push(err),
            PanelAction::RequestQuit => {}
            PanelAction::OpenScene(p) => {
                let _ = state.open_scene(&p);
            }
            PanelAction::RunScene => state.engine.is_running = true,
            PanelAction::StopScene => state.engine.stop(),
            PanelAction::StepTick => state.engine.step(),
            PanelAction::ReloadScene => {
                let _ = state.engine.reload();
            }
            PanelAction::OpenBehaviorFile(path) => {
                crate::panels::behavior_editor::open_file(state, path);
            }
            PanelAction::OpenLuaFile(path) => {
                crate::panels::lua_editor::open_file(state, path);
            }
            PanelAction::AgentSendMessage(text) => {
                if let Some(ref client) = state.agent_client {
                    let context = crate::agent::context::ContextBuilder::build_from_state(state);
                    let context_msg =
                        crate::agent::context::ContextBuilder::build_system_message(&context);
                    let tools = state.agent_tool_registry.all_defs();

                    let messages = vec![
                        crate::agent::ChatMessage {
                            role: "system".into(),
                            content: SYSTEM_PROMPT.into(),
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        context_msg,
                        crate::agent::ChatMessage {
                            role: "user".into(),
                            content: text,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                    ];

                    let (tx, rx) = std::sync::mpsc::channel();
                    state.agent_rx = Some(rx);
                    state.panels.agent_panel.is_streaming = true;
                    state.panels.agent_panel.streaming_text.clear();
                    state.panels.agent_panel.tool_round = 0;

                    if let Some(handle) = client.chat(messages, &tools, false, tx) {
                        state.agent_handle = Some(handle);
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
fn _kind_check(kind: DockKind) -> DockKind {
    kind
}
