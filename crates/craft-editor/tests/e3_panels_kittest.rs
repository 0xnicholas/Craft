use craft_editor::panels::Panel;
use craft_editor::panels::agent_panel::AgentPanel;
use craft_editor::state::{AgentMessage, EditorState};
use egui_kittest::Harness;

#[test]
fn agent_panel_renders_empty_state() {
    let mut state = EditorState::default();
    let mut panel = AgentPanel;
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn agent_panel_renders_with_messages() {
    let mut state = EditorState::default();
    state.panels.agent_panel.messages = vec![
        AgentMessage::User {
            text: "Hello".into(),
        },
        AgentMessage::Agent {
            text: "Hi there!".into(),
            suggestions: vec![],
        },
    ];
    let mut panel = AgentPanel;
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn agent_panel_renders_streaming_text() {
    let mut state = EditorState::default();
    state.panels.agent_panel.is_streaming = true;
    state.panels.agent_panel.streaming_text = "Generating response...".into();
    let mut panel = AgentPanel;
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}
