use std::path::PathBuf;

use super::{Panel, PanelAction};
use crate::json_path::Severity;
use crate::state::{BehaviorEditorState, EditorState, json_path_lsp};

pub struct BehaviorEditorPanel;

impl Panel for BehaviorEditorPanel {
    fn id(&self) -> &'static str {
        "behavior_editor"
    }
    fn title(&self) -> &'static str {
        "Behavior Editor"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        if state.standalone_behavior.path.is_none() {
            ui.vertical_centered(|ui| ui.label("Behavior Editor — open a .behavior.json file"));
            return Vec::new();
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
                if let Err(e) = std::fs::write(path, &state.standalone_behavior.buffer) {
                    state.ui.status_message = format!("write failed: {e}");
                } else {
                    state.standalone_behavior.dirty = false;
                    state.ui.status_message = format!("wrote {}", path.display());
                }
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
