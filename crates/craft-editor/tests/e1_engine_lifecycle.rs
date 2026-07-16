use craft_editor::engine::EditorEngine;
use std::path::PathBuf;
use tower_defense as _;

#[test]
fn loads_and_runs_tower_defense() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let path = workspace_root.join("games/tower_defense/scene.json");

    let mut engine = EditorEngine::new();
    engine.load_scene_file(&path).expect("load scene");
    assert!(engine.is_running);

    for _ in 0..5 {
        engine.step();
    }
    assert!(engine.renderer().frames_rendered() >= 5);
}

#[test]
fn stop_freezes_renderer() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let path = workspace_root.join("games/tower_defense/scene.json");

    let mut engine = EditorEngine::new();
    engine.load_scene_file(&path).unwrap();
    engine.step();
    let before = engine.renderer().frames_rendered();

    engine.stop();
    for _ in 0..3 {
        let ticked = engine.tick_if_due();
        assert!(!ticked);
    }
    assert_eq!(engine.renderer().frames_rendered(), before);
}
