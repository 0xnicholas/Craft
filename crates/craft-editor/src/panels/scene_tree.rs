use std::collections::HashSet;

use super::{Panel, PanelAction};
use crate::state::EditorState;

pub struct SceneTreePanel;

impl SceneTreePanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SceneTreePanel {
    fn default() -> Self {
        Self::new()
    }
}

fn matches_filter(text: &str, id: &str, type_name: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    let needle = text.to_lowercase();
    id.to_lowercase().contains(&needle) || type_name.to_lowercase().contains(&needle)
}

fn children_of<'a>(scene: &'a craft_kernel::Scene, parent_id: &str) -> Vec<&'a String> {
    let mut out: Vec<&String> = scene
        .nodes
        .iter()
        .filter(|n| n.parent.as_deref() == Some(parent_id))
        .map(|n| &n.id)
        .collect();
    out.sort();
    out
}

fn find_node<'a>(scene: &'a craft_kernel::Scene, id: &str) -> Option<&'a craft_kernel::Node> {
    scene.nodes.iter().find(|n| n.id == id)
}

fn draw_subtree(
    ui: &mut egui::Ui,
    node: &craft_kernel::Node,
    scene: &craft_kernel::Scene,
    state: &mut EditorState,
    depth: usize,
    visible: &HashSet<String>,
    actions: &mut Vec<PanelAction>,
) {
    if !visible.contains(&node.id) {
        return;
    }
    let selected = state.panels.scene_tree.selected_node.as_deref() == Some(node.id.as_str());
    let indent = depth as f32 * 12.0;

    let _ = ui.next_widget_position();
    let hovered = ui.rect_contains_pointer(ui.max_rect());
    if hovered {
        let dragged_id = ui
            .ctx()
            .data_mut(|d| d.get_temp::<String>(egui::Id::new("drag_node_id")));
        if let Some(dragged_id) = dragged_id {
            if dragged_id != node.id && ui.input(|i| i.pointer.any_released()) {
                actions.push(PanelAction::ReparentNode(dragged_id, node.id.clone()));
                ui.ctx().data_mut(|d| {
                    d.remove::<String>(egui::Id::new("drag_node_id"));
                });
            }
        }
        let lua_dragged = ui
            .ctx()
            .data_mut(|d| d.get_temp::<String>(egui::Id::new("drag_lua_path")));
        if let Some(lua_path) = lua_dragged {
            if ui.input(|i| i.pointer.any_released()) {
                actions.push(PanelAction::SetLuaClass(node.id.clone(), lua_path.clone()));
                ui.ctx().data_mut(|d| {
                    d.remove::<String>(egui::Id::new("drag_lua_path"));
                });
            }
        }
    }

    ui.horizontal(|ui| {
        ui.add_space(indent);
        let icon = crate::theme::node_type_icon(&node.type_name);
        let label = format!("{icon} [{}] {}", node.type_name, node.id);
        let response = ui.selectable_label(selected, label);
        if response.clicked() {
            state.panels.scene_tree.selected_node = Some(node.id.clone());
        }
        if response.secondary_clicked() {
            state.panels.scene_tree.context_menu = Some((
                node.id.clone(),
                response.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)),
            ));
        }
        if response.drag_started() {
            ui.ctx().data_mut(|d| {
                d.insert_temp(egui::Id::new("drag_node_id"), node.id.clone());
            });
        }
    });
    for child_id in children_of(scene, &node.id) {
        if let Some(child) = find_node(scene, child_id) {
            draw_subtree(ui, child, scene, state, depth + 1, visible, actions);
        }
    }
}

fn collect_visible(scene: &craft_kernel::Scene, filter: &str) -> HashSet<String> {
    let mut visible = HashSet::new();
    if filter.is_empty() {
        for n in &scene.nodes {
            visible.insert(n.id.clone());
        }
        return visible;
    }
    for n in &scene.nodes {
        if matches_filter(filter, &n.id, &n.type_name) {
            visible.insert(n.id.clone());
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for n in &scene.nodes {
            if visible.contains(&n.id) {
                continue;
            }
            if let Some(p) = &n.parent {
                if visible.contains(p) {
                    visible.insert(n.id.clone());
                    changed = true;
                }
            }
        }
    }
    visible
}

impl Panel for SceneTreePanel {
    fn id(&self) -> &'static str {
        "scene_tree"
    }
    fn title(&self) -> &'static str {
        "Scene Tree"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        ui.text_edit_singleline(&mut state.panels.scene_tree.filter_text);

        let filter_text = state.panels.scene_tree.filter_text.clone();
        let def_snapshot = state.scene.as_ref().map(|s| s.def.clone());

        let Some(def) = def_snapshot.as_ref() else {
            ui.vertical_centered(|ui| ui.label("No scene open"));
            return Vec::new();
        };

        let visible = collect_visible(def, &filter_text);
        let mut actions: Vec<PanelAction> = Vec::new();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in &def.nodes {
                if node.parent.is_none() {
                    draw_subtree(ui, node, def, state, 0, &visible, &mut actions);
                }
            }
        });

        if let Some((ref node_id, pos)) = state.panels.scene_tree.context_menu.take() {
            egui::Area::new("node_context_menu".into())
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style().as_ref()).show(ui, |ui| {
                        if ui.button("Add Child Node").clicked() {
                            actions.push(PanelAction::AddChildNodeAt(node_id.clone()));
                        }
                        if ui.button("Duplicate").clicked() {
                            actions.push(PanelAction::DuplicateNode(node_id.clone()));
                        }
                        if ui.button("Delete").clicked() {
                            actions.push(PanelAction::DeleteNode(node_id.clone()));
                        }
                    });
                });
        }

        actions
    }
}
