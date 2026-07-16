use super::{Panel, PanelAction};
use crate::state::EditorState;

pub struct TerminalPreviewPanel;

impl TerminalPreviewPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TerminalPreviewPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Panel for TerminalPreviewPanel {
    fn id(&self) -> &'static str {
        "terminal_preview"
    }
    fn title(&self) -> &'static str {
        "Terminal Preview"
    }
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let mut actions = Vec::new();

        ui.horizontal(|ui| {
            if ui.button("Run").clicked() {
                actions.push(PanelAction::RunScene);
            }
            if ui.button("Pause").clicked() {
                state.engine.pause();
            }
            if ui.button("Stop").clicked() {
                actions.push(PanelAction::StopScene);
            }
            if ui.button("Reload").clicked() {
                actions.push(PanelAction::ReloadScene);
            }
            if ui.button("Step").clicked() {
                actions.push(PanelAction::StepTick);
            }
        });

        if state.engine.scene_path.is_none() {
            ui.vertical_centered(|ui| ui.label("No scene loaded"));
            return actions;
        }

        let grid = state.engine.renderer().grid();
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("terminal_grid")
                .spacing(egui::vec2(0.0, 0.0))
                .num_columns(grid.width as usize)
                .show(ui, |ui| {
                    for y in 0..grid.height as usize {
                        for x in 0..grid.width as usize {
                            let cell = grid.cell(x, y);
                            ui.monospace(format!("{}", cell.ch));
                        }
                        ui.end_row();
                    }
                });
        });

        actions
    }
}
