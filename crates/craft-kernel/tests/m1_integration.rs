use craft_kernel::{NodeRegistry, Scene, craft_node};

craft_node!(Player, {
    components: {
        position: Vec2 = [0.0, 0.0],
        health: Int = 100,
    },
});

craft_node!(Enemy, {
    components: {
        position: Vec2 = [0.0, 0.0],
        speed: Float = 1.0,
    },
});

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    r.register::<Player>();
    r.register::<Enemy>();
    r
}

#[test]
fn craft_node_macro_registers_node_type() {
    let r = registry();
    let player = r.get("Player").expect("Player registered");
    assert_eq!(player.type_name, "Player");
    let names: Vec<&str> = player
        .component_specs()
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["position", "health"]);
}

#[test]
fn end_to_end_load_scene_using_macro_generated_types() {
    let json = r#"{
        "kind": "scene",
        "name": "main",
        "nodes": [
            {
                "id": "player_1",
                "type": "Player",
                "components": { "position": [10.0, 20.0], "health": 100 }
            },
            {
                "id": "enemy_1",
                "type": "Enemy",
                "components": { "position": [5.0, 5.0], "speed": 2.5 }
            }
        ]
    }"#;

    let scene = Scene::parse(json, "scene.json", &registry()).expect("parse");
    assert_eq!(scene.kind, "scene");
    assert_eq!(scene.name, "main");
    assert_eq!(scene.nodes.len(), 2);
    assert_eq!(scene.nodes[0].type_name, "Player");
    assert_eq!(scene.nodes[1].type_name, "Enemy");
}

#[test]
fn end_to_end_rejects_wrong_component_type() {
    let json = r#"{
        "kind": "scene",
        "name": "main",
        "nodes": [
            {
                "id": "player_1",
                "type": "Player",
                "components": { "health": "fast" }
            }
        ]
    }"#;
    let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
    let craft_kernel::EngineError::Validation { errors, .. } = err else {
        panic!("expected validation error");
    };
    let e = errors
        .iter()
        .find(|e| e.json_path == "$.nodes[0].components.health")
        .expect("health error");
    assert_eq!(e.expected_type, "int");
    assert!(e.suggestion.is_some());
}

#[test]
fn end_to_end_rejects_unknown_node_type_with_suggestion() {
    let json = r#"{
        "kind": "scene",
        "name": "main",
        "nodes": [
            { "id": "e1", "type": "Enemmy", "components": {} }
        ]
    }"#;
    let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
    let craft_kernel::EngineError::Validation { errors, .. } = err else {
        panic!("expected validation error");
    };
    let e = errors
        .iter()
        .find(|e| e.json_path == "$.nodes[0].type")
        .expect("type error");
    assert!(e.suggestion.as_deref().unwrap().contains("Enemy"));
}
