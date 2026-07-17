use craft_editor::state::{AgentMessage, AgentSuggestion, EditorState, SuggestionStatus};
use craft_kernel::hot_reload::SceneDiff;
use craft_kernel::{NodeRegistry, Scene};

#[test]
fn accept_applies_empty_diff_without_error() {
    let mut state = EditorState::default();

    let mut registry = NodeRegistry::new();
    registry.instantiate_all();
    let def = Scene {
        kind: "scene".into(),
        name: "test".into(),
        nodes: vec![],
        spawn_counter: 0,
    };
    state.engine.engine.load_scene(def.clone());

    state.scene = Some(craft_editor::state::SceneState {
        path: std::path::PathBuf::from("test.json"),
        def,
        last_saved_hash: 0,
        file_watcher_epoch: 0,
    });

    let diff = SceneDiff::default();

    let suggestion = AgentSuggestion {
        id: "s-1".into(),
        description: "test change".into(),
        diff,
        status: SuggestionStatus::Pending,
    };

    state.panels.agent_panel.messages = vec![AgentMessage::Agent {
        text: "I suggest a change".into(),
        suggestions: vec![suggestion],
    }];

    if let AgentMessage::Agent { suggestions, .. } = &mut state.panels.agent_panel.messages[0] {
        for s in suggestions {
            let result = craft_kernel::hot_reload::apply_scene_diff(
                &mut state.scene.as_mut().unwrap().def,
                state.engine.engine.node_registry_mut(),
                &s.diff,
            );
            if result.is_ok() {
                s.status = SuggestionStatus::Accepted;
            }
        }
    }

    if let AgentMessage::Agent { suggestions, .. } = &state.panels.agent_panel.messages[0] {
        assert!(matches!(suggestions[0].status, SuggestionStatus::Accepted));
    }
}
