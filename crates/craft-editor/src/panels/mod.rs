use std::path::PathBuf;

use egui::Ui;

use crate::state::{DockKind, EditorError, EditorState};

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
        }
    }
}

#[allow(dead_code)]
fn _kind_check(kind: DockKind) -> DockKind {
    kind
}
