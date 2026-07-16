use std::path::PathBuf;

use craft_editor::engine::EditorEngine;
use craft_editor::panels::{Panel, PanelAction};
use craft_editor::state::{EditorState, SceneState};
use tower_defense as _;

fn tower_defense_scene_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("games/tower_defense/scene.json")
}

fn state_with_loaded_scene() -> EditorState {
    let path = tower_defense_scene_path();
    let mut engine = EditorEngine::new();
    engine.load_scene_file(&path).expect("load scene");

    let mut registry = craft_kernel::NodeRegistry::new();
    registry.instantiate_all();
    let def = craft_editor::io::load_scene(&path, &registry).expect("load scene def");

    let mut state = EditorState {
        engine,
        ..Default::default()
    };
    let last_saved_hash = craft_kernel::hash_scene_state(&def);
    state.scene = Some(SceneState {
        path: path.clone(),
        def,
        last_saved_hash,
        file_watcher_epoch: 0,
    });
    state
}

#[test]
fn empty_state_has_no_scene_and_engine_is_idle() {
    let state = EditorState::default();
    assert!(state.scene.is_none());
    assert!(!state.engine.is_running);
    assert!(state.engine.scene_path().is_none());
}

#[test]
fn open_scene_populates_state_and_engine() {
    let state = state_with_loaded_scene();
    let scene = state.scene.as_ref().expect("scene loaded");
    assert!(!scene.def.nodes.is_empty(), "tower_defense has nodes");
    assert_eq!(
        scene.path.file_name().and_then(|s| s.to_str()),
        Some("scene.json")
    );
    assert_eq!(
        state
            .engine
            .scene_path()
            .unwrap()
            .file_name()
            .and_then(|s| s.to_str()),
        Some("scene.json")
    );
    assert!(state.engine.is_running, "engine auto-runs after load");
}

#[test]
fn selecting_node_via_scene_tree_state_persists_in_panels() {
    let mut state = state_with_loaded_scene();
    let first_id = state
        .scene
        .as_ref()
        .unwrap()
        .def
        .nodes
        .first()
        .unwrap()
        .id
        .clone();

    state.panels.scene_tree.selected_node = Some(first_id.clone());

    assert_eq!(
        state.panels.scene_tree.selected_node.as_deref(),
        Some(first_id.as_str())
    );
}

#[test]
fn dirty_flag_flips_after_component_edit() {
    let mut state = state_with_loaded_scene();

    let initial_dirty = state.scene.as_ref().unwrap().is_dirty();
    assert!(!initial_dirty);

    let scene = state.scene.as_mut().unwrap();
    let node = scene
        .def
        .nodes
        .iter_mut()
        .find(|n| !n.components.is_empty())
        .expect("at least one node has components");
    let key = node.components.keys().next().cloned().unwrap();
    let original = node.components.get(&key).unwrap().value.clone();
    let new_value = match original {
        craft_kernel::ComponentValue::Int(v) => craft_kernel::ComponentValue::Int(v + 1),
        craft_kernel::ComponentValue::Float(v) => craft_kernel::ComponentValue::Float(v + 1.0),
        other => other,
    };
    node.components.get_mut(&key).unwrap().value = new_value;

    assert!(
        state.scene.as_ref().unwrap().is_dirty(),
        "mutating a component value must flip the dirty flag"
    );

    let tmp = tempfile::tempdir().expect("tempdir");
    let scene_path = tmp.path().join("scene.json");
    std::fs::copy(&state.scene.as_ref().unwrap().path, &scene_path).expect("copy fixture");
    state.scene.as_mut().unwrap().path = scene_path.clone();

    state
        .save_dirty()
        .expect("save_dirty writes to disk and clears the flag");
    assert!(
        !state.scene.as_ref().unwrap().is_dirty(),
        "save_dirty must reset the last_saved_hash"
    );
}

#[test]
fn engine_step_increments_tick_and_renders() {
    let mut engine = EditorEngine::new();
    engine
        .load_scene_file(&tower_defense_scene_path())
        .expect("load scene");
    let before = engine.engine.tick;
    engine.step();
    engine.step();
    assert!(engine.engine.tick > before);
    assert!(engine.renderer().frames_rendered() >= 2);
}

#[test]
fn stop_scene_clears_is_running() {
    let mut engine = EditorEngine::new();
    engine
        .load_scene_file(&tower_defense_scene_path())
        .expect("load scene");
    assert!(engine.is_running);
    engine.stop();
    assert!(!engine.is_running);
}

#[test]
fn panel_action_dispatch_routes_run_step_stop() {
    let mut state = state_with_loaded_scene();

    craft_editor::panels::dispatch(vec![PanelAction::RunScene], &mut state);
    assert!(state.engine.is_running, "RunScene flips is_running");

    let tick_before = state.engine.engine.tick;
    craft_editor::panels::dispatch(vec![PanelAction::StepTick], &mut state);
    assert!(
        state.engine.engine.tick > tick_before,
        "StepTick advances the engine tick"
    );

    craft_editor::panels::dispatch(vec![PanelAction::StopScene], &mut state);
    assert!(!state.engine.is_running, "StopScene clears is_running");
}

#[test]
fn panel_action_set_status_writes_to_ui_state() {
    let mut state = EditorState::default();
    craft_editor::panels::dispatch(
        vec![PanelAction::SetStatus("hello".to_string())],
        &mut state,
    );
    assert_eq!(state.ui.status_message, "hello");
}

#[test]
fn panels_have_expected_ids_and_titles() {
    assert_eq!(
        craft_editor::panels::SceneTreePanel::new().id(),
        "scene_tree"
    );
    assert_eq!(
        craft_editor::panels::InspectorPanel::new().id(),
        "inspector"
    );
    assert_eq!(
        craft_editor::panels::FileBrowserPanel::new().id(),
        "file_browser"
    );
    assert_eq!(
        craft_editor::panels::TerminalPreviewPanel::new().id(),
        "terminal_preview"
    );
    assert_eq!(
        craft_editor::panels::SceneTreePanel::new().title(),
        "Scene Tree"
    );
    assert_eq!(
        craft_editor::panels::TerminalPreviewPanel::new().title(),
        "Terminal Preview"
    );
    assert_eq!(
        craft_editor::panels::InspectorPanel::new().title(),
        "Inspector"
    );
    assert_eq!(
        craft_editor::panels::FileBrowserPanel::new().title(),
        "Files"
    );
}
