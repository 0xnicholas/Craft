use craft_editor::io::load_scene;

#[test]
fn loads_tower_defense_scene() {
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let path = workspace_root.join("games/tower_defense/scene.json");
    let registry = tower_defense::build_node_registry();
    let scene = load_scene(&path, &registry).expect("tower_defense scene parses");
    assert!(!scene.nodes.is_empty(), "scene should have nodes");
}
