use super::{Panel, PanelAction};
use crate::state::EditorState;
use egui::Ui;

pub struct BehaviorEditorPanel;

impl Panel for BehaviorEditorPanel {
    fn id(&self) -> &'static str {
        "behavior_editor"
    }
    fn title(&self) -> &'static str {
        "Behavior Editor"
    }
    fn show(&mut self, ui: &mut Ui, _state: &mut EditorState) -> Vec<PanelAction> {
        ui.vertical_centered(|ui| ui.label("Behavior Editor — coming in E2"));
        Vec::new()
    }
}
