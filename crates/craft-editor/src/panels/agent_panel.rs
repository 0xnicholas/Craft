use super::{Panel, PanelAction};
use crate::state::EditorState;
use egui::Ui;

pub struct AgentPanel;

impl Panel for AgentPanel {
    fn id(&self) -> &'static str {
        "agent_panel"
    }
    fn title(&self) -> &'static str {
        "Agent Copilot"
    }
    fn show(&mut self, ui: &mut Ui, _state: &mut EditorState) -> Vec<PanelAction> {
        ui.vertical_centered(|ui| ui.label("Agent Copilot — coming in E3"));
        Vec::new()
    }
}
