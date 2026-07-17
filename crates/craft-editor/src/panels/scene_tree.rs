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
) {
    if !visible.contains(&node.id) {
        return;
    }
    let selected = state.panels.scene_tree.selected_node.as_deref() == Some(node.id.as_str());
    let indent = depth as f32 * 12.0;
    ui.horizontal(|ui| {
        ui.add_space(indent);
        let icon = crate::theme::node_type_icon(&node.type_name);
        let label = format!("{icon} [{}] {}", node.type_name, node.id);
        if ui.selectable_label(selected, label).clicked() {
            state.panels.scene_tree.selected_node = Some(node.id.clone());
        }
    });
    for child_id in children_of(scene, &node.id) {
        if let Some(child) = find_node(scene, child_id) {
            draw_subtree(ui, child, scene, state, depth + 1, visible);
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

        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in &def.nodes {
                if node.parent.is_none() {
                    draw_subtree(ui, node, def, state, 0, &visible);
                }
            }
        });

        Vec::new()
    }
}
