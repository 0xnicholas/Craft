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
}

pub trait Panel {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn show(&mut self, ui: &mut Ui, state: &mut EditorState) -> Vec<PanelAction>;
}

pub mod agent_panel;
pub mod behavior_editor;
pub mod lua_editor;

pub use agent_panel::AgentPanel;
pub use behavior_editor::BehaviorEditorPanel;
pub use lua_editor::LuaEditorPanel;

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
        }
    }
}

#[allow(dead_code)]
fn _kind_check(kind: DockKind) -> DockKind {
    kind
}
