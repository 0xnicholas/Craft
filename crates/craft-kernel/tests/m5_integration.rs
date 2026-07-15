use craft_kernel::craft_node;
use craft_kernel::serde_json::json;
use craft_kernel::{Engine, NodeRegistry, ResourceRef, ResourceRegistry, Scene, SignalId};

craft_node!(HRPlayer, {
    components: {
        health: Int = 100,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(HREnemy, {
    components: {
        health: Int = 50,
        position: Vec2 = [0.0, 0.0],
    },
});

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    r.register::<HRPlayer>();
    r.register::<HREnemy>();
    r
}

fn engine_with(scene_json: &str) -> Engine {
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine
}

#[test]
fn file_change_applies_via_diff() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 50, "position": [10.0, 20.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);
    let result = engine.apply_hot_reload(&new_scene).expect("hot reload");
    assert!(result.applied, "diff must be applied when scene changed");
    assert_eq!(result.affected_node_ids, vec!["p1".to_string()]);

    let node = &engine.scene.as_ref().unwrap().nodes[0];
    let health = node.components.get("health").expect("health").value.clone();
    assert_eq!(health, craft_kernel::ComponentValue::Int(50));
    let pos = node
        .components
        .get("position")
        .expect("position")
        .value
        .clone();
    assert_eq!(pos, craft_kernel::ComponentValue::Vec2([10.0, 20.0]));
}

#[test]
fn hot_reload_preserves_node_ids() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            },
            {
                "id": "e1",
                "type": "HREnemy",
                "components": { "health": 50, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 25, "position": [1.0, 1.0] }
            },
            {
                "id": "e1",
                "type": "HREnemy",
                "components": { "health": 10, "position": [5.0, 5.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);
    engine.apply_hot_reload(&new_scene).expect("hot reload");

    let scene = engine.scene.as_ref().unwrap();
    let ids: Vec<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["p1", "e1"],
        "Node IDs must remain stable across reloads"
    );
}

#[test]
fn hot_reload_adds_new_nodes() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            },
            {
                "id": "e1",
                "type": "HREnemy",
                "components": { "health": 50, "position": [5.0, 5.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);
    assert_eq!(engine.scene.as_ref().unwrap().nodes.len(), 1);
    engine.apply_hot_reload(&new_scene).expect("hot reload");
    assert_eq!(engine.scene.as_ref().unwrap().nodes.len(), 2);
    let ids: Vec<&str> = engine
        .scene
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .collect();
    assert!(ids.contains(&"e1"));
    assert!(ids.contains(&"p1"));
}

#[test]
fn hot_reload_removes_nodes() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            },
            {
                "id": "e1",
                "type": "HREnemy",
                "components": { "health": 50, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);
    assert_eq!(engine.scene.as_ref().unwrap().nodes.len(), 2);
    engine.apply_hot_reload(&new_scene).expect("hot reload");
    assert_eq!(engine.scene.as_ref().unwrap().nodes.len(), 1);
    let ids: Vec<&str> = engine
        .scene
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .collect();
    assert_eq!(ids, vec!["p1"]);
}

#[test]
fn hot_reload_preserves_signal_subscriptions() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 50, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);

    let hit: SignalId = engine.bus.declare("hit");
    let count = std::cell::RefCell::new(0u32);
    {
        let count = std::rc::Rc::new(count);
        let count_inner = count.clone();
        engine.bus.subscribe(hit, move |_| {
            *count_inner.borrow_mut() += 1;
        });
    }

    engine.apply_hot_reload(&new_scene).expect("hot reload");
    engine.emit(hit, json!({}));
    engine.tick();
    let count = engine.bus.subscriber_count(hit);
    assert!(count >= 1, "subscriber count must be preserved");
}

#[test]
fn reregistering_resource_does_not_change_loaded_instances() {
    let mut resources = ResourceRegistry::new();
    let id = resources.register("res://enemy_stats.json", json!({"hp": 100}));
    let v1: u32 = resources.version(id).expect("version 1");

    let _id_again = resources.register("res://enemy_stats.json", json!({"hp": 150}));

    let v_after: u32 = resources.version(id).expect("version after re-register");
    assert_eq!(
        v1, v_after,
        "ADR 0009: re-registering an existing URI must NOT bump the version"
    );

    let fresh_id = resources.register("res://other.json", json!({"hp": 200}));
    let v_fresh = resources.version(fresh_id).expect("fresh version");
    assert!(
        v_fresh > v_after,
        "a brand-new URI gets a higher version than existing ones"
    );
}

#[test]
fn resource_ref_snapshot_preserved_on_reregister() {
    let mut resources = ResourceRegistry::new();
    let _id = resources.register("res://data.json", json!({"hp": 100}));
    let ref1: ResourceRef = resources.resolve_ref("res://data.json").expect("ref1");
    assert_eq!(ref1.snapshot_version, 0);
    let _id2 = resources.register("res://data.json", json!({"hp": 150}));
    let ref2 = resources.resolve_ref("res://data.json").expect("ref2");
    assert_eq!(
        ref2.snapshot_version, 0,
        "existing ref keeps snapshot version"
    );
}

#[test]
fn hot_reload_emits_hot_reload_signal() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let updated = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "HRPlayer",
                "components": { "health": 99, "position": [0.0, 0.0] }
            }
        ]
    }"#;
    let new_scene = Scene::parse(updated, "scene.json", &registry()).expect("parse updated");
    let mut engine = engine_with(initial);

    let hot_reload_sig = engine.bus.declare("hot_reload");
    let received = std::rc::Rc::new(std::cell::RefCell::new(0u32));
    let received_inner = received.clone();
    engine
        .bus
        .subscribe(hot_reload_sig, move |_| *received_inner.borrow_mut() += 1);

    engine.apply_hot_reload(&new_scene).expect("hot reload");
    engine.tick();
    assert_eq!(
        *received.borrow(),
        1,
        "hot_reload signal should be delivered at start of next tick"
    );
}

#[test]
fn hot_reload_at_tick_500_does_not_break_state() {
    let initial = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "HRPlayer",
                "components": { "health": 0, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "health", "by": 1 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let mut engine = engine_with(initial);
    for _ in 0..500 {
        engine.tick();
    }
    let pre_hash = engine.state_hash();
    let pre_health = engine.scene.as_ref().unwrap().nodes[0]
        .components
        .get("health")
        .unwrap()
        .value
        .clone();
    assert_eq!(pre_health, craft_kernel::ComponentValue::Int(500));

    let mut new_scene = engine.scene.as_ref().unwrap().clone();
    new_scene.nodes[0].components.insert(
        "health".to_string(),
        craft_kernel::Component {
            value: craft_kernel::ComponentValue::Int(999),
            kind: Default::default(),
        },
    );
    let result = engine.apply_hot_reload(&new_scene).expect("hot reload");
    assert!(result.applied);

    let post_health = engine.scene.as_ref().unwrap().nodes[0]
        .components
        .get("health")
        .unwrap()
        .value
        .clone();
    assert_eq!(post_health, craft_kernel::ComponentValue::Int(999));

    let post_hash = engine.state_hash();
    assert_ne!(pre_hash, post_hash, "state must change after hot reload");
}
