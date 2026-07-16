use super::{Panel, PanelAction};
use crate::state::EditorState;
use craft_kernel::ComponentValue;

pub struct InspectorPanel;

impl InspectorPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InspectorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for InspectorPanel {
    fn id(&self) -> &'static str {
        "inspector"
    }
    fn title(&self) -> &'static str {
        "Inspector"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let Some(scene_state) = &mut state.scene else {
            ui.vertical_centered(|ui| ui.label("No scene open"));
            return Vec::new();
        };
        let Some(sel) = state.panels.scene_tree.selected_node.clone() else {
            ui.label("Select a node in the Scene Tree");
            return Vec::new();
        };

        let selected_idx = scene_state
            .def
            .nodes
            .iter()
            .position(|n| n.id == sel && !n.destroyed);
        let Some(idx) = selected_idx else {
            ui.label("Selected node not found");
            return Vec::new();
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            let node = &mut scene_state.def.nodes[idx];
            ui.heading(format!("{} ({})", node.type_name, node.id));
            if let Some(lc) = &node.lua_class {
                ui.label(format!("lua_class: {lc}"));
            }
            ui.separator();

            let keys: Vec<String> = node.components.keys().cloned().collect();
            for key in keys {
                ui.horizontal(|ui| {
                    ui.label(&key);
                    let comp = node.components.get_mut(&key).expect("key exists");
                    match &mut comp.value {
                        ComponentValue::Int(v) => {
                            ui.add(egui::DragValue::new(v));
                        }
                        ComponentValue::Float(v) => {
                            ui.add(egui::DragValue::new(v).speed(0.1));
                        }
                        ComponentValue::String(v) => {
                            ui.text_edit_singleline(v);
                        }
                        ComponentValue::Bool(v) => {
                            ui.checkbox(v, "");
                        }
                        ComponentValue::Vec2([x, y]) => {
                            ui.add(egui::DragValue::new(x));
                            ui.add(egui::DragValue::new(y));
                        }
                        ComponentValue::Nil => {
                            ui.label("nil");
                        }
                    }
                });
            }

            if node.components.is_empty() {
                ui.label("(no components)");
            }
        });

        Vec::new()
    }
}
