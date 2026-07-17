use craft_editor::panels::Panel;
use craft_editor::panels::file_browser::FileBrowserPanel;
use craft_editor::panels::scene_tree::SceneTreePanel;
use craft_editor::state::EditorState;
use egui_kittest::Harness;

#[test]
fn scene_tree_panel_renders_no_scene() {
    let mut state = EditorState::default();
    let mut panel = SceneTreePanel::new();
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn scene_tree_panel_renders_filter_field() {
    let mut state = EditorState::default();
    state.panels.scene_tree.filter_text = "test".into();
    let mut panel = SceneTreePanel::new();
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn file_browser_panel_renders_no_project() {
    let mut state = EditorState::default();
    let mut panel = FileBrowserPanel::new();
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn file_browser_panel_renders_with_filter() {
    let mut state = EditorState::default();
    state.panels.file_browser.filter = "lua".into();
    let mut panel = FileBrowserPanel::new();
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn file_browser_panel_renders_project_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("test.lua"), "").unwrap();
    std::fs::write(dir.path().join("scene.json"), "{}").unwrap();
    std::fs::create_dir(dir.path().join("subdir")).unwrap();

    let mut state = EditorState::default();
    state.project = Some(craft_editor::state::ProjectState {
        root: dir.path().to_path_buf(),
    });
    let mut panel = FileBrowserPanel::new();
    let mut harness = Harness::new_ui(|ui| {
        panel.show(ui, &mut state);
    });
    harness.run();
}

#[test]
fn scene_tree_and_file_browser_together() {
    let mut state = EditorState::default();
    let mut scene_panel = SceneTreePanel::new();
    let mut file_panel = FileBrowserPanel::new();
    let mut harness = Harness::new_ui(|ui| {
        scene_panel.show(ui, &mut state);
        file_panel.show(ui, &mut state);
    });
    harness.run();
}
