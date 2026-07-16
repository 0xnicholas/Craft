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
                    FileKind::Lua => {
                        actions.push(PanelAction::SetStatus(format!(
                            "Lua editor coming in E2 — {name}"
                        )));
                    }
                    _ => {}
                }
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
                ui.label(format!("[{name}]"));
            });
            draw_dir(ui, &path, filter_lower, actions, depth + 1);
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

        let Some(project) = &state.project else {
            ui.vertical_centered(|ui| ui.label("Open a project"));
            return Vec::new();
        };

        let filter_lower = state.panels.file_browser.filter.to_lowercase();
        let mut actions = Vec::new();
        egui::ScrollArea::vertical().show(ui, |ui| {
            draw_dir(ui, &project.root, &filter_lower, &mut actions, 0);
        });
        actions
    }
}
