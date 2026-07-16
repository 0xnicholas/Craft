use super::{Panel, PanelAction};
use crate::state::EditorState;
use egui::Ui;

pub struct LuaEditorPanel;

impl Panel for LuaEditorPanel {
    fn id(&self) -> &'static str {
        "lua_editor"
    }
    fn title(&self) -> &'static str {
        "Lua Editor"
    }
    fn show(&mut self, ui: &mut Ui, _state: &mut EditorState) -> Vec<PanelAction> {
        ui.vertical_centered(|ui| ui.label("Lua Editor — coming in E2"));
        Vec::new()
    }
}
