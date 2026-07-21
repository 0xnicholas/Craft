use std::path::PathBuf;

use super::{Panel, PanelAction};
use crate::state::EditorState;

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

pub struct LuaEditorPanel;

impl Panel for LuaEditorPanel {
    fn id(&self) -> &'static str {
        "lua_editor"
    }
    fn title(&self) -> &'static str {
        "Lua Editor"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let mut actions = Vec::new();
        check_file_drop(ui, &mut actions);
        if let Some(err) = state.engine.lua_runtime_error() {
            ui.colored_label(
                egui::Color32::RED,
                format!("Lua runtime init failed: {err}"),
            );
        }

        if state.lua_editor.lsp.is_none()
            && !state.lua_editor.fallback_mode
            && state.lua_editor.current_path.is_some()
        {
            let workspace = state
                .lua_editor
                .current_path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));
            match crate::lua_lsp::spawn(&workspace) {
                Ok(client) => {
                    state.lua_editor.lsp = Some(client);
                    state.lua_editor.opened_uris.clear();
                    state.ui.status_message = "LuaLS spawned".to_string();
                }
                Err(_) => {
                    state.lua_editor.fallback_mode = true;
                    state.ui.status_message =
                        "LuaLS not found. Install via brew install lua-language-server."
                            .to_string();
                }
            }
        }

        if state.lua_editor.fallback_mode {
            ui.colored_label(
                egui::Color32::YELLOW,
                "LuaLS not found. Install via brew install lua-language-server.",
            );
        }

        if state.lua_editor.current_path.is_none() {
            ui.vertical_centered(|ui| ui.label("Lua Editor — open a .lua file"));
            return actions;
        }

        let path_display = state
            .lua_editor
            .current_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        ui.label(format!("Editing: {path_display}"));

        let changed;
        {
            let resp = ui.add(
                egui::TextEdit::multiline(&mut state.lua_editor.buffer)
                    .code_editor()
                    .desired_rows(25)
                    .desired_width(f32::INFINITY),
            );
            changed = resp.changed();
        }

        if changed {
            state.lua_editor.dirty = true;
        }

        // LSP: didOpen, didChange, Ctrl+Space completion request
        {
            let uri = state
                .lua_editor
                .current_path
                .as_ref()
                .map(|p| format!("file://{}", p.display()))
                .unwrap_or_default();

            if let Some(client) = state.lua_editor.lsp.as_mut() {
                if !state.lua_editor.opened_uris.contains(&uri) {
                    let _ = client.send_notification(
                        "textDocument/didOpen",
                        serde_json::json!({
                            "textDocument": {
                                "uri": uri.clone(),
                                "languageId": "lua",
                                "version": 1,
                                "text": state.lua_editor.buffer
                            }
                        }),
                    );
                    state.lua_editor.opened_uris.insert(uri.clone());
                }

                if changed {
                    let _ = client.send_notification(
                        "textDocument/didChange",
                        serde_json::json!({
                            "textDocument": { "uri": uri.clone(), "version": 2 },
                            "contentChanges": [{ "text": state.lua_editor.buffer }]
                        }),
                    );
                }

                if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Space)) {
                    let _ = client.send_request(
                        "textDocument/completion",
                        serde_json::json!({
                            "textDocument": { "uri": uri },
                            "position": { "line": 0, "character": 0 },
                            "context": { "triggerKind": 1 }
                        }),
                    );
                    state.lua_editor.completion_requested = true;
                }
            }
        }

        // Drain LSP messages (diagnostics + completions)
        let mut completion_items: Vec<(String, String)> = Vec::new();
        {
            let mut msgs: Vec<crate::lsp::LspMessage> = Vec::new();
            if let Some(client) = state.lua_editor.lsp.as_mut() {
                while let Ok(msg) = client.stdout_rx.try_recv() {
                    msgs.push(msg);
                }
            }
            for msg in msgs {
                if msg.json.get("id").and_then(|i| i.as_i64()).is_some() {
                    if state.lua_editor.completion_requested {
                        if let Some(result) = msg.json.get("result") {
                            let items = parse_completion_items(result);
                            if !items.is_empty() {
                                completion_items = items;
                                state.lua_editor.show_completion_popup = true;
                            }
                        }
                        state.lua_editor.completion_requested = false;
                    }
                } else {
                    let uri = state
                        .lua_editor
                        .current_path
                        .as_ref()
                        .map(|p| format!("file://{}", p.display()))
                        .unwrap_or_default();
                    apply_lsp_message(msg, &uri, state);
                }
            }
        }

        // Render completion popup
        if state.lua_editor.show_completion_popup && !completion_items.is_empty() {
            let mut close = false;
            let mut chosen: Option<String> = None;
            egui::Window::new("Lua completions")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    for (label, insert_text) in &completion_items {
                        if ui.selectable_label(false, label).clicked() {
                            chosen = Some(insert_text.clone());
                            close = true;
                        }
                    }
                    if ui.button("cancel").clicked() {
                        close = true;
                    }
                });
            if close {
                state.lua_editor.show_completion_popup = false;
                if let Some(text) = chosen {
                    state.lua_editor.buffer.push_str(&text);
                }
            }
        }

        // Diagnostic display
        for diag in &state.lua_editor.diagnostics {
            let color = match diag.severity {
                crate::json_path::Severity::Error => egui::Color32::RED,
                crate::json_path::Severity::Warning => egui::Color32::YELLOW,
            };
            ui.colored_label(color, format!("line {}: {}", diag.line, diag.message));
        }

        if ui.button("Apply (save + reload)").clicked() {
            save_and_reload(state);
        }

        actions
    }
}

fn apply_lsp_message(msg: crate::lsp::LspMessage, expected_uri: &str, state: &mut EditorState) {
    let json = msg.json;
    if json.get("method").and_then(|m| m.as_str()) == Some("textDocument/publishDiagnostics") {
        let Some(params) = json.get("params") else {
            return;
        };
        let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
        if !expected_uri.is_empty() && uri != expected_uri {
            return;
        }
        if let Some(diags) = params.get("diagnostics").and_then(|d| d.as_array()) {
            state.lua_editor.diagnostics.clear();
            for d in diags {
                let sev = match d.get("severity").and_then(|s| s.as_i64()).unwrap_or(1) {
                    1 => crate::json_path::Severity::Error,
                    2 => crate::json_path::Severity::Warning,
                    _ => crate::json_path::Severity::Warning,
                };
                let line = d
                    .get("range")
                    .and_then(|r| r.get("start"))
                    .and_then(|s| s.get("line"))
                    .and_then(|l| l.as_u64())
                    .unwrap_or(0) as u32;
                let message = d
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                state
                    .lua_editor
                    .diagnostics
                    .push(crate::lua_lsp::LspDiagnostic {
                        line,
                        col: 0,
                        end_line: line,
                        end_col: 0,
                        severity: sev,
                        message,
                    });
            }
        }
    }
}

fn parse_completion_items(value: &serde_json::Value) -> Vec<(String, String)> {
    let arr = match value.as_array() {
        Some(a) => a,
        None => match value.get("items").and_then(|i| i.as_array()) {
            Some(a) => a,
            None => return Vec::new(),
        },
    };
    arr.iter()
        .filter_map(|item| {
            let label = item
                .get("label")
                .and_then(|l| l.as_str())
                .unwrap_or("")
                .to_string();
            if label.is_empty() {
                return None;
            }
            let insert_text = item
                .get("insertText")
                .and_then(|t| t.as_str())
                .unwrap_or(&label)
                .to_string();
            Some((label, insert_text))
        })
        .collect()
}

pub fn open_file(state: &mut EditorState, path: PathBuf) {
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            state.lua_editor.current_path = Some(path.clone());
            state.lua_editor.buffer = content;
            state.lua_editor.dirty = false;
            state.lua_editor.diagnostics.clear();
            state.lua_editor.fallback_mode = false;
            state.lua_editor.lsp = None;
            state.lua_editor.opened_uris.clear();
            state.lua_editor.completion_requested = false;
            state.lua_editor.show_completion_popup = false;
            state.ui.status_message = format!("opened {}", path.display());
        }
        Err(e) => {
            state.ui.status_message = format!("open failed: {e}");
        }
    }
}

pub fn save_and_reload(state: &mut EditorState) {
    let Some(path) = state.lua_editor.current_path.clone() else {
        return;
    };
    if let Err(e) = std::fs::write(&path, &state.lua_editor.buffer) {
        state.ui.status_message = format!("write failed: {e}");
        return;
    }

    if let Some(client) = state.lua_editor.lsp.as_mut() {
        let uri = format!("file://{}", path.display());
        let _ = client.send_notification(
            "textDocument/didSave",
            serde_json::json!({
                "textDocument": { "uri": uri, "text": state.lua_editor.buffer }
            }),
        );
    }

    let class_name = derive_class_name(&path);
    if let Some(runtime) = state.engine.lua_runtime_mut() {
        match runtime.reload_class(&class_name, &state.lua_editor.buffer) {
            Ok(()) => {
                state.lua_editor.dirty = false;
                state.ui.status_message = format!("hot-reloaded Lua class: {class_name}");
            }
            Err(e) => {
                state.ui.status_message = format!("reload failed: {e}");
            }
        }
    } else {
        state.ui.status_message = format!("saved {} (no Lua runtime)", path.display());
        state.lua_editor.dirty = false;
    }
}

pub fn derive_class_name(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed");
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if parent.is_empty() {
        stem.to_string()
    } else {
        format!("{parent}.{stem}")
    }
}
