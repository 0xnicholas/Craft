use egui::Context;

use crate::app::EditorApp;
use crate::panels::PanelAction;

pub fn draw(ctx: &Context, app: &mut EditorApp) {
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    app.pending_actions.push(PanelAction::SaveScene);
                    ui.close_menu();
                }
                if ui.button("Quit").clicked() {
                    app.pending_actions.push(PanelAction::RequestQuit);
                    ui.close_menu();
                }
            });
            ui.menu_button("Scene", |ui| {
                if ui.button("Run (F5)").clicked() {
                    app.pending_actions.push(PanelAction::RunScene);
                    ui.close_menu();
                }
                if ui.button("Stop (F8)").clicked() {
                    app.pending_actions.push(PanelAction::StopScene);
                    ui.close_menu();
                }
                if ui.button("Step (F10)").clicked() {
                    app.pending_actions.push(PanelAction::StepTick);
                    ui.close_menu();
                }
                if ui.button("Reload").clicked() {
                    app.pending_actions.push(PanelAction::ReloadScene);
                    ui.close_menu();
                }
            });
            ui.menu_button("View", |_ui| {});
            ui.menu_button("Help", |ui| {
                ui.label("craft-editor v0.1.0");
            });
        });
    });
}
