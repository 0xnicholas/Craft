use super::{Panel, PanelAction};
use crate::state::{AgentMessage, EditorState, SuggestionStatus};
use craft_kernel::hot_reload;
use egui::{ScrollArea, TextEdit, Ui};

pub struct AgentPanel;

impl Panel for AgentPanel {
    fn id(&self) -> &'static str {
        "agent_panel"
    }
    fn title(&self) -> &'static str {
        "Agent Copilot"
    }
    fn show(&mut self, ui: &mut Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let mut actions = Vec::new();
        let panel = &mut state.panels.agent_panel;

        egui::CollapsingHeader::new("Context")
            .default_open(true)
            .show(ui, |ui| {
                if let Some(ref path) = state.engine.scene_path {
                    ui.label(format!("Active: {}", path.display()));
                }
                if let Some(ref selected) = state.panels.scene_tree.selected_node {
                    ui.label(format!("Selected: {}", selected));
                }
            });

        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (msg_idx, msg) in panel.messages.iter_mut().enumerate() {
                    match msg {
                        AgentMessage::User { text } => {
                            ui.label(format!("You: {}", text));
                        }
                        AgentMessage::Agent { text, suggestions } => {
                            ui.label(format!("Agent: {}", text));
                            for s in suggestions {
                                ui.horizontal(|ui| {
                                    ui.add_space(20.0);
                                    match s.status {
                                        SuggestionStatus::Pending => {
                                            if ui.button("Preview Diff").clicked() {
                                                panel.diff_preview = Some(msg_idx);
                                            }
                                            if ui.button("Accept").clicked() {
                                                let diff = s.diff.clone();
                                                let is_running = state.engine.is_running;
                                                if let Some(ref mut scene_state) = state.scene {
                                                    if is_running {
                                                        let mut cloned = scene_state.def.clone();
                                                        match hot_reload::apply_scene_diff(
                                                            &mut cloned,
                                                            state.engine.engine.node_registry_mut(),
                                                            &diff,
                                                        ) {
                                                            Ok(()) => {
                                                                if state
                                                                    .engine
                                                                    .engine
                                                                    .apply_hot_reload(&cloned)
                                                                    .is_ok()
                                                                {
                                                                    if let Some(updated) =
                                                                        state.engine.engine.scene()
                                                                    {
                                                                        scene_state.def =
                                                                            updated.clone();
                                                                    }
                                                                    actions.push(
                                                                        PanelAction::SetStatus(
                                                                            format!(
                                                                                "Hot-reloaded: {}",
                                                                                s.description
                                                                            ),
                                                                        ),
                                                                    );
                                                                    s.status =
                                                                        SuggestionStatus::Accepted;
                                                                } else {
                                                                    s.status =
                                                                        SuggestionStatus::Failed {
                                                                            reason: "hot reload returned error"
                                                                                .into(),
                                                                        };
                                                                }
                                                            }
                                                            Err(e) => {
                                                                actions.push(
                                                                    PanelAction::SetStatus(
                                                                        format!(
                                                                            "Hot reload failed: {e}"
                                                                        ),
                                                                    ),
                                                                );
                                                                s.status =
                                                                    SuggestionStatus::Failed {
                                                                        reason: e.to_string(),
                                                                    };
                                                            }
                                                        }
                                                    } else {
                                                        match hot_reload::apply_scene_diff(
                                                            &mut scene_state.def,
                                                            state.engine.engine.node_registry_mut(),
                                                            &diff,
                                                        ) {
                                                            Ok(()) => {
                                                                actions.push(
                                                                    PanelAction::SetStatus(
                                                                        format!(
                                                                            "Applied: {}",
                                                                            s.description
                                                                        ),
                                                                    ),
                                                                );
                                                                s.status =
                                                                    SuggestionStatus::Accepted;
                                                            }
                                                            Err(e) => {
                                                                actions.push(
                                                                    PanelAction::SetStatus(
                                                                        format!(
                                                                            "Apply failed: {e}"
                                                                        ),
                                                                    ),
                                                                );
                                                                s.status =
                                                                    SuggestionStatus::Failed {
                                                                        reason: e.to_string(),
                                                                    };
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    actions.push(PanelAction::SetStatus(
                                                        "No scene open".into(),
                                                    ));
                                                }
                                            }
                                            if ui.button("✕").clicked() {
                                                s.status = SuggestionStatus::Rejected;
                                            }
                                        }
                                        SuggestionStatus::Accepted => {
                                            ui.colored_label(
                                                egui::Color32::GREEN,
                                                "✓ Accepted",
                                            );
                                        }
                                        SuggestionStatus::Rejected => {
                                            ui.colored_label(
                                                egui::Color32::GRAY,
                                                "✕ Rejected",
                                            );
                                        }
                                        SuggestionStatus::Failed { ref reason } => {
                                            ui.colored_label(
                                                egui::Color32::RED,
                                                format!("Failed: {}", reason),
                                            );
                                        }
                                    }
                                });
                            }
                        }
                        AgentMessage::System { text } => {
                            ui.colored_label(egui::Color32::GRAY, text);
                        }
                    }
                }
                if panel.is_streaming {
                    ui.colored_label(egui::Color32::GRAY, &panel.streaming_text);
                }
            });

        ui.separator();

        let send_clicked = ui.horizontal(|ui| {
            let resp = ui.add_sized(
                [ui.available_width() - 40.0, 20.0],
                TextEdit::singleline(&mut panel.input).hint_text("Ask about your scene..."),
            );
            let clicked = ui.button("Send").clicked();
            let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            clicked || enter_pressed
        });

        if send_clicked.inner {
            let text = panel.input.trim().to_string();
            if !text.is_empty() {
                panel
                    .messages
                    .push(AgentMessage::User { text: text.clone() });
                panel.input.clear();
                actions.push(PanelAction::AgentSendMessage(text));
            }
        }

        if let Some(preview_idx) = panel.diff_preview {
            let mut close = false;
            egui::Window::new("Diff Preview")
                .collapsible(false)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    if let Some(AgentMessage::Agent { suggestions, .. }) =
                        panel.messages.get(preview_idx)
                    {
                        for s in suggestions {
                            if matches!(s.status, SuggestionStatus::Pending) {
                                ui.label(&s.description);
                                ui.separator();
                            }
                        }
                    }
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                });
            if close {
                panel.diff_preview = None;
            }
        }

        actions
    }
}
