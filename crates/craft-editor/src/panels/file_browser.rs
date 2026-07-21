use std::path::Path;

use super::{Panel, PanelAction};
use crate::state::{EditorState, FileKind};

pub struct FileBrowserPanel;

impl FileBrowserPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FileBrowserPanel {
    fn default() -> Self {
        Self::new()
    }
}

fn classify(path: &Path) -> FileKind {
    if path.is_dir() {
        return FileKind::Directory;
    }
    let name = path.to_string_lossy();
    if name.ends_with(".behavior.json") {
        return FileKind::Behavior;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("lua") => FileKind::Lua,
        Some("json") => FileKind::Scene,
        Some("toml") => FileKind::Other,
        Some("png") | Some("jpg") | Some("jpeg") | Some("svg") => FileKind::Resource,
        _ => FileKind::Other,
    }
}

fn draw_dir(
    ui: &mut egui::Ui,
    dir: &Path,
    filter_lower: &str,
    actions: &mut Vec<PanelAction>,
    depth: usize,
    state: &mut EditorState,
) {
    let entries: Vec<std::fs::DirEntry> = match std::fs::read_dir(dir) {
        Ok(it) => it.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    let mut dirs: Vec<std::fs::DirEntry> = Vec::new();
    let mut files: Vec<std::fs::DirEntry> = Vec::new();
    for entry in entries {
        let kind = classify(&entry.path());
        if matches!(kind, FileKind::Directory) {
            dirs.push(entry);
        } else {
            files.push(entry);
        }
    }
    dirs.sort_by_key(|d| d.file_name());
    files.sort_by_key(|d| d.file_name());

    for entry in files {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !filter_lower.is_empty() && !name.to_lowercase().contains(filter_lower) {
            continue;
        }
        let kind = classify(&path);
        let indent = depth as f32 * 12.0;
        ui.horizontal(|ui| {
            ui.add_space(indent);
            let label = name.clone();
            let resp = ui.selectable_label(false, label);
            if resp.double_clicked() {
                match kind {
                    FileKind::Scene => actions.push(PanelAction::OpenScene(path.clone())),
                    FileKind::Lua => actions.push(PanelAction::OpenLuaFile(path.clone())),
                    FileKind::Behavior => actions.push(PanelAction::OpenBehaviorFile(path.clone())),
                    _ => {}
                }
            }
            if resp.secondary_clicked() {
                state.panels.file_browser.context_menu = Some((
                    path.clone(),
                    false,
                    resp.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)),
                ));
            }
            if resp.drag_started() {
                let path_str = path.to_string_lossy().to_string();
                if matches!(kind, FileKind::Lua) {
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(egui::Id::new("drag_lua_path"), path_str.clone());
                    });
                }
                ui.ctx().data_mut(|d| {
                    d.insert_temp(egui::Id::new("drag_file_path"), path_str);
                });
            }
        });
    }

    for entry in dirs {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with('.') {
            let indent = depth as f32 * 12.0;
            ui.horizontal(|ui| {
                ui.add_space(indent);
                let resp = ui.label(format!("[{name}]"));
                if resp.secondary_clicked() {
                    state.panels.file_browser.context_menu = Some((
                        path.clone(),
                        true,
                        resp.hover_pos().unwrap_or(egui::pos2(0.0, 0.0)),
                    ));
                }
            });
            draw_dir(ui, &path, filter_lower, actions, depth + 1, state);
        }
    }
}

impl Panel for FileBrowserPanel {
    fn id(&self) -> &'static str {
        "file_browser"
    }
    fn title(&self) -> &'static str {
        "Files"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        ui.text_edit_singleline(&mut state.panels.file_browser.filter);

        let project_root = match &state.project {
            Some(p) => p.root.clone(),
            None => {
                ui.vertical_centered(|ui| ui.label("Open a project"));
                return Vec::new();
            }
        };

        let filter_lower = state.panels.file_browser.filter.to_lowercase();
        let mut actions = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            draw_dir(ui, &project_root, &filter_lower, &mut actions, 0, state);
        });

        if let Some((ref path, is_dir, pos)) = state.panels.file_browser.context_menu.take() {
            let path = path.clone();
            let show_new_types = state
                .panels
                .file_browser
                .new_file_parent
                .as_ref()
                == Some(&path);
            egui::Area::new("file_context_menu".into())
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style().as_ref()).show(ui, |ui| {
                        if is_dir {
                            if show_new_types {
                                ui.label("File type:");
                                if ui.button("Scene (.json)").clicked() {
                                    let stub = r#"{"kind": "scene", "name": "", "nodes": []}"#;
                                    let file_path = path.join("untitled_scene.json");
                                    let _ = std::fs::write(&file_path, stub);
                                    actions.push(PanelAction::SetStatus(format!(
                                        "created {}",
                                        file_path.display()
                                    )));
                                    state.panels.file_browser.new_file_parent = None;
                                }
                                if ui.button("Lua (.lua)").clicked() {
                                    let stub = "local M = {}\nreturn M\n";
                                    let file_path = path.join("untitled.lua");
                                    let _ = std::fs::write(&file_path, stub);
                                    actions.push(PanelAction::SetStatus(format!(
                                        "created {}",
                                        file_path.display()
                                    )));
                                    state.panels.file_browser.new_file_parent = None;
                                }
                                if ui.button("Behavior (.behavior.json)").clicked() {
                                    let stub = r#"{"kind": "on_tick", "actions": []}"#;
                                    let file_path = path.join("untitled.behavior.json");
                                    let _ = std::fs::write(&file_path, stub);
                                    actions.push(PanelAction::SetStatus(format!(
                                        "created {}",
                                        file_path.display()
                                    )));
                                    state.panels.file_browser.new_file_parent = None;
                                }
                                if ui.button("Back").clicked() {
                                    state.panels.file_browser.new_file_parent = None;
                                }
                            } else {
                                if ui.button("New File").clicked() {
                                    state.panels.file_browser.new_file_parent =
                                        Some(path.clone());
                                }
                                if ui.button("New Folder").clicked() {
                                    actions.push(PanelAction::NewFolder(path.clone()));
                                }
                                if ui.button("Delete").clicked() {
                                    actions.push(PanelAction::DeleteFile(path.clone()));
                                }
                            }
                        } else {
                            if ui.button("Open").clicked() {
                                match classify(&path) {
                                    FileKind::Scene => {
                                        actions.push(PanelAction::OpenScene(path.clone()))
                                    }
                                    FileKind::Lua => {
                                        actions.push(PanelAction::OpenLuaFile(path.clone()))
                                    }
                                    FileKind::Behavior => {
                                        actions.push(PanelAction::OpenBehaviorFile(path.clone()))
                                    }
                                    _ => {}
                                }
                            }
                            if ui.button("Delete").clicked() {
                                actions.push(PanelAction::DeleteFile(path.clone()));
                            }
                        }
                    });
                });
        }

        actions
    }
}
