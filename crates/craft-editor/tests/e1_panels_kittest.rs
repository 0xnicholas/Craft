use std::path::PathBuf;

use craft_editor::panels::{FileBrowserPanel, Panel, SceneTreePanel, TerminalPreviewPanel};
use craft_editor::state::{EditorState, ProjectState, SceneState};
use egui_kittest::Harness;
use tower_defense as _;

fn tower_defense_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("games/tower_defense")
}

fn state_with_tower_defense_scene() -> EditorState {
    let path = tower_defense_dir().join("scene.json");
    let mut engine = craft_editor::engine::EditorEngine::new();
    engine.load_scene_file(&path).expect("load scene");

    let mut registry = craft_kernel::NodeRegistry::new();
    registry.instantiate_all();
    let def = craft_editor::io::load_scene(&path, &registry).expect("load def");
    let last_saved_hash = craft_kernel::hash_scene_state(&def);

    let mut state = EditorState {
        engine,
        ..Default::default()
    };
    state.scene = Some(SceneState {
        path,
        def,
        last_saved_hash,
        file_watcher_epoch: 0,
    });
    state
}

fn render_panel<P: Panel>(mut panel: P, mut state: EditorState) {
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn scene_tree_empty_renders() {
    render_panel(SceneTreePanel::new(), EditorState::default());
}

#[test]
fn scene_tree_loaded_renders() {
    render_panel(SceneTreePanel::new(), state_with_tower_defense_scene());
}

#[test]
fn inspector_empty_renders() {
    render_panel(
        craft_editor::panels::InspectorPanel::new(),
        EditorState::default(),
    );
}

#[test]
fn inspector_with_selection_renders() {
    let mut state = state_with_tower_defense_scene();
    let first_id = state.scene.as_ref().unwrap().def.nodes[0].id.clone();
    state.panels.scene_tree.selected_node = Some(first_id);
    render_panel(craft_editor::panels::InspectorPanel::new(), state);
}

#[test]
fn file_browser_no_project_renders() {
    render_panel(FileBrowserPanel::new(), EditorState::default());
}

#[test]
fn file_browser_with_tower_defense_renders() {
    let state = EditorState {
        project: Some(ProjectState {
            root: tower_defense_dir(),
        }),
        ..Default::default()
    };
    render_panel(FileBrowserPanel::new(), state);
}

#[test]
fn terminal_preview_no_scene_renders() {
    render_panel(TerminalPreviewPanel::new(), EditorState::default());
}

#[test]
fn terminal_preview_with_tower_defense_renders() {
    let mut state = state_with_tower_defense_scene();
    state.engine.step();
    state.engine.step();
    render_panel(TerminalPreviewPanel::new(), state);
}
