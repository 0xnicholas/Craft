use super::{Panel, PanelAction};
use crate::json_path::CursorCtx;
use crate::state::{BehaviorEditState, EditorState, json_path_lsp};
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

        let actions_to_dispatch = Vec::new();

        let selected_node_id = scene_state.def.nodes[idx].id.clone();
        let old_components: std::collections::BTreeMap<String, craft_kernel::ComponentValue> =
            scene_state.def.nodes[idx]
                .components
                .iter()
                .map(|(k, c)| (k.clone(), c.value.clone()))
                .collect();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let node = &mut scene_state.def.nodes[idx];
            let icon = crate::theme::node_type_icon(&node.type_name);
            let color = crate::theme::node_type_color(&node.type_name);
            ui.colored_label(color, format!("{icon} {} ({})", node.id, node.type_name));
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

            ui.separator();
            ui.heading("Behaviors");

            let behavior_count = node.behaviors.len();
            let node_id = node.id.clone();

            for bi in 0..behavior_count {
                let key = (node_id.clone(), bi);
                let is_expanded = state.panels.inspector.expanded_behaviors.contains(&key);
                let behavior_json = serde_json::to_string(&node.behaviors[bi]).unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.label(format!("behavior {bi}: {behavior_json}"));
                    if ui.selectable_label(is_expanded, "▶ edit JSON").clicked() {
                        if is_expanded {
                            state.panels.inspector.expanded_behaviors.remove(&key);
                            state.panels.inspector.behavior_edits.remove(&key);
                        } else {
                            state
                                .panels
                                .inspector
                                .expanded_behaviors
                                .insert(key.clone());
                            let pretty = serde_json::to_string_pretty(&node.behaviors[bi])
                                .unwrap_or_else(|_| "{}".to_string());
                            state.panels.inspector.behavior_edits.insert(
                                key.clone(),
                                BehaviorEditState {
                                    node_id: node_id.clone(),
                                    behavior_idx: bi,
                                    buffer: pretty.clone(),
                                    parsed: serde_json::from_str(&pretty).ok(),
                                    errors: Vec::new(),
                                    completion: None,
                                    dirty: false,
                                },
                            );
                        }
                    }
                });

                if !is_expanded {
                    continue;
                }

                // --- Phase 1: render editor, collect state ---
                let (errors, completion_items, has_completion, can_apply) = {
                    let Some(edit) = state.panels.inspector.behavior_edits.get_mut(&key) else {
                        continue;
                    };

                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut edit.buffer)
                            .code_editor()
                            .desired_rows(12)
                            .desired_width(f32::INFINITY),
                    );
                    if resp.changed() {
                        edit.dirty = true;
                        edit.parsed = serde_json::from_str(&edit.buffer).ok();
                        edit.errors = json_path_lsp().validate(&edit.buffer);
                    }

                    let errors = edit.errors.clone();

                    if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Space)) {
                        let cursor_byte = edit.buffer.len();
                        let path = json_path_lsp().path_at(&edit.buffer, cursor_byte);
                        let ctx = CursorCtx {
                            in_object_key: false,
                            in_object_value: true,
                            partial_token: String::new(),
                        };
                        let completions = json_path_lsp().complete(&path, &ctx);
                        edit.completion = Some(crate::json_path::CompletionPopup {
                            items: completions,
                            selected: 0,
                        });
                    }

                    let completion_items: Vec<_> = if let Some(popup) = edit.completion.as_ref() {
                        popup
                            .items
                            .iter()
                            .map(|c| (c.label.clone(), c.insert_text.clone()))
                            .collect()
                    } else {
                        Vec::new()
                    };
                    let has_completion = edit.completion.is_some();
                    let can_apply = edit.parsed.is_some()
                        && edit
                            .errors
                            .iter()
                            .all(|e| matches!(e.severity, crate::json_path::Severity::Warning));
                    (errors, completion_items, has_completion, can_apply)
                };

                // --- Phase 2: errors ---
                if !errors.is_empty() {
                    for e in &errors {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!(
                                "{}: {}",
                                if matches!(e.severity, crate::json_path::Severity::Error) {
                                    "error"
                                } else {
                                    "warn"
                                },
                                e.message
                            ),
                        );
                    }
                }

                // --- Phase 3: completion popup ---
                let mut completion_close = false;
                let mut chosen_text: Option<String> = None;
                if has_completion && !completion_items.is_empty() {
                    egui::Window::new("completions")
                        .collapsible(false)
                        .resizable(false)
                        .show(ui.ctx(), |ui| {
                            for (label, insert_text) in completion_items.iter() {
                                if ui.selectable_label(false, label).clicked() {
                                    chosen_text = Some(insert_text.clone());
                                    completion_close = true;
                                }
                            }
                            if ui.button("cancel").clicked() {
                                completion_close = true;
                            }
                        });
                }
                if completion_close {
                    if let Some(edit) = state.panels.inspector.behavior_edits.get_mut(&key) {
                        if let Some(text) = chosen_text {
                            edit.buffer.push_str(&text);
                        }
                        edit.completion = None;
                    }
                }

                // --- Phase 4: apply / cancel buttons ---
                let mut apply_clicked = false;
                let mut cancel_clicked = false;
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Apply"))
                        .clicked()
                    {
                        apply_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel_clicked = true;
                    }
                });

                if apply_clicked || cancel_clicked {
                    if apply_clicked {
                        let old_behavior = node.behaviors[bi].clone();
                        if let Some(edit) = state.panels.inspector.behavior_edits.get_mut(&key) {
                            if let Some(parsed) = edit.parsed.take() {
                                state.undo_redo.begin_action("edit behavior");
                                let nid = node_id.clone();
                                let old = old_behavior.clone();
                                let new = parsed.clone();
                                state.undo_redo.add_undo(move |s| {
                                    if let Some(scene) = &mut s.scene {
                                        if let Some(n) =
                                            scene.def.nodes.iter_mut().find(|n| n.id == nid)
                                        {
                                            n.behaviors[bi] = old.clone();
                                        }
                                    }
                                });
                                let nid2 = node_id.clone();
                                state.undo_redo.add_do(move |s| {
                                    if let Some(scene) = &mut s.scene {
                                        if let Some(n) =
                                            scene.def.nodes.iter_mut().find(|n| n.id == nid2)
                                        {
                                            n.behaviors[bi] = new.clone();
                                        }
                                    }
                                });
                                node.behaviors[bi] = parsed;
                                state.ui.status_message =
                                    format!("applied behavior {node_id}#{bi}");
                                state.undo_redo.commit_action();
                            }
                        }
                    }
                    state.panels.inspector.expanded_behaviors.remove(&key);
                    state.panels.inspector.behavior_edits.remove(&key);
                }
            }

            if node.behaviors.is_empty() {
                ui.label("(no behaviors)");
            }
        });

        let new_components: std::collections::BTreeMap<String, craft_kernel::ComponentValue> =
            scene_state.def.nodes[idx]
                .components
                .iter()
                .map(|(k, c)| (k.clone(), c.value.clone()))
                .collect();

        if old_components != new_components {
            state.undo_redo.begin_action("change component");
            let nid = selected_node_id.clone();
            for (key, new_val) in &new_components {
                if let Some(old_val) = old_components.get(key) {
                    if old_val != new_val {
                        let old_val = old_val.clone();
                        let new_val = new_val.clone();
                        let key_undo = key.clone();
                        let key_redo = key.clone();
                        let nid_undo = nid.clone();
                        let nid_redo = nid.clone();
                        let old_val_clone = old_val.clone();
                        let new_val_clone = new_val.clone();
                        state.undo_redo.add_undo(move |s| {
                            if let Some(scene) = &mut s.scene {
                                if let Some(n) =
                                    scene.def.nodes.iter_mut().find(|n| n.id == nid_undo)
                                {
                                    if let Some(c) = n.components.get_mut(&key_undo) {
                                        c.value = old_val_clone.clone();
                                    }
                                }
                            }
                        });
                        state.undo_redo.add_do(move |s| {
                            if let Some(scene) = &mut s.scene {
                                if let Some(n) =
                                    scene.def.nodes.iter_mut().find(|n| n.id == nid_redo)
                                {
                                    if let Some(c) = n.components.get_mut(&key_redo) {
                                        c.value = new_val_clone.clone();
                                    }
                                }
                            }
                        });
                    }
                }
            }
            state.undo_redo.commit_action();
        }

        actions_to_dispatch
    }
}
