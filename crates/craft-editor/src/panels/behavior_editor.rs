use std::path::PathBuf;

use super::{Panel, PanelAction};
use crate::json_path::Severity;
use crate::state::{BehaviorEditorState, EditorState, json_path_lsp};

fn check_file_drop(ui: &egui::Ui, actions: &mut Vec<PanelAction>) {
    if ui.input(|i| i.pointer.any_released()) {
        if let Some(path) = ui
            .ctx()
            .data_mut(|d| d.get_temp::<String>(egui::Id::new("drag_file_path")))
        {
            if ui.rect_contains_pointer(ui.max_rect()) {
                let p = PathBuf::from(&path);
                let file_str = path;
                ui.ctx().data_mut(|d| {
                    d.remove::<String>(egui::Id::new("drag_file_path"));
                });
                if file_str.ends_with(".behavior.json") {
                    actions.push(PanelAction::OpenBehaviorFile(p));
                } else if file_str.ends_with(".lua") {
                    actions.push(PanelAction::OpenLuaFile(p));
                } else if file_str.ends_with(".json") {
                    actions.push(PanelAction::OpenScene(p));
                }
            }
        }
    }
}

pub struct BehaviorEditorPanel;

impl Panel for BehaviorEditorPanel {
    fn id(&self) -> &'static str {
        "behavior_editor"
    }
    fn title(&self) -> &'static str {
        "Behavior Editor"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let mut actions = Vec::new();
        check_file_drop(ui, &mut actions);
        if state.standalone_behavior.path.is_none() {
            ui.vertical_centered(|ui| ui.label("Behavior Editor — open a .behavior.json file"));
            return actions;
        }

        let path_display = state
            .standalone_behavior
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        ui.label(format!("Editing: {path_display}"));

        let resp = ui.add(
            egui::TextEdit::multiline(&mut state.standalone_behavior.buffer)
                .code_editor()
                .desired_rows(20)
                .desired_width(f32::INFINITY),
        );

        if resp.changed() {
            state.standalone_behavior.dirty = true;
            let lsp = json_path_lsp();
            state.standalone_behavior.errors = lsp.validate(&state.standalone_behavior.buffer);
        }

        let errors = state.standalone_behavior.errors.clone();
        if !errors.is_empty() {
            ui.label(format!("{} error(s):", errors.len()));
            for e in &errors {
                let kind = match e.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                };
                ui.label(format!(
                    "[{kind}] line {} col {}: {}",
                    e.line, e.col, e.message
                ));
            }
        }

        if ui
            .add_enabled(
                errors
                    .iter()
                    .all(|e| matches!(e.severity, Severity::Warning)),
                egui::Button::new("Apply"),
            )
            .clicked()
        {
            if let Some(path) = &state.standalone_behavior.path {
                let old_content = std::fs::read_to_string(path).unwrap_or_default();
                let new_content = state.standalone_behavior.buffer.clone();
                state.undo_redo.begin_action("edit behavior file");
                let path_clone = path.clone();
                let old_clone = old_content.clone();
                state.undo_redo.add_undo(move |_s| {
                    let _ = std::fs::write(&path_clone, &old_clone);
                });
                let path_clone2 = path.clone();
                let new_clone = new_content.clone();
                state.undo_redo.add_do(move |_s| {
                    let _ = std::fs::write(&path_clone2, &new_clone);
                });
                if let Err(e) = std::fs::write(path, &new_content) {
                    state.ui.status_message = format!("write failed: {e}");
                } else {
                    state.standalone_behavior.dirty = false;
                    state.ui.status_message = format!("wrote {}", path.display());
                }
                state.undo_redo.commit_action();
            }
        }

        Vec::new()
    }
}

pub fn open_file(state: &mut EditorState, path: PathBuf) {
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let errors = json_path_lsp().validate(&content);
            state.standalone_behavior = BehaviorEditorState {
                path: Some(path.clone()),
                buffer: content,
                errors,
                completion: None,
                dirty: false,
            };
            state.ui.status_message = format!("opened {}", path.display());
        }
        Err(e) => {
            state.ui.status_message = format!("open failed: {e}");
        }
    }
}
