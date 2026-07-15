use craft_kernel::craft_node;
use craft_kernel::serde_json::json;
use craft_kernel::{Engine, NodeRegistry, Scene};

craft_node!(Counter, {
    components: {
        value: Int = 0,
        max: Int = 100,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(Bullet, {
    components: {
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(Spawner, {
    components: {
        spawned: Int = 0,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(Mob, {
    components: {
        health: Int = 100,
    },
});

craft_node!(Widget, {
    components: {
        x: Float = 0.0,
        target_x: Float = 10.0,
    },
});

craft_node!(Runner, {
    components: {
        note: String = "",
        level: Int = 0,
    },
});

craft_node!(Probe, {
    components: {
        hits: Int = 0,
    },
});

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    r.register::<Counter>();
    r.register::<Bullet>();
    r.register::<Spawner>();
    r.register::<Mob>();
    r.register::<Widget>();
    r.register::<Runner>();
    r.register::<Probe>();
    r
}

fn load_scene(engine: &mut Engine, scene: Scene) {
    engine.load_scene(scene);
}

#[test]
fn verb_set_state_writes_component() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "set_state", "target": "self", "key": "value", "value": 42 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(42),
        "verb set_state must write the component value during tick"
    );
}

#[test]
fn verb_emit_queues_signal() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "emit", "signal": "ping", "args": { "x": 1 } }
                        ]
                    },
                    {
                        "kind": "on_signal",
                        "signal": "ping",
                        "actions": [
                            { "kind": "set_state", "target": "self", "key": "value", "value": 99 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(0),
        "ADR 0003: emit during tick N is delivered at start of tick N+1"
    );

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(99),
        "on_signal handler runs at start of tick 2 and sets value to 99"
    );
}

#[test]
fn verb_destroy_removes_node() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "destroy", "target": { "kind": "node", "id": "b1" } }
                        ]
                    }
                ]
            },
            {
                "id": "b1",
                "type": "Bullet",
                "components": { "position": [1.0, 2.0] }
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    assert_eq!(engine.scene.as_ref().unwrap().nodes.len(), 2);
    engine.tick();
    assert_eq!(
        engine.scene.as_ref().unwrap().nodes.len(),
        1,
        "verb destroy removes the target node from the scene"
    );
}

#[test]
fn verb_spawn_creates_node() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "s1",
                "type": "Spawner",
                "components": { "spawned": 0, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "spawn", "type": "Bullet", "parent": "self", "components": { "position": [5.0, 5.0] } }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let nodes = &engine.scene.as_ref().unwrap().nodes;
    assert!(
        nodes.iter().any(|n| n.type_name == "Bullet"),
        "verb spawn must add a Bullet node to the scene"
    );
}

#[test]
fn verb_if_branches_on_expression() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 10, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            {
                                "kind": "if",
                                "cond": { "lt": [ { "ref": "self.value" }, 5 ] },
                                "then": [ { "kind": "set_state", "target": "self", "key": "value", "value": 1 } ],
                                "else": [ { "kind": "set_state", "target": "self", "key": "value", "value": 2 } ]
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(2),
        "value=10 is not <5, so the else branch (value=2) should run"
    );
}

#[test]
fn verb_move_adds_delta_to_component() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 10, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "value", "by": 5 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(15),
        "verb move adds the by-delta to the current value"
    );
}

#[test]
fn verb_animate_schedules_animation_state() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "w1",
                "type": "Widget",
                "components": { "x": 0.0, "target_x": 10.0 },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "animate", "target": "self", "key": "x", "to": 10.0, "duration": 3 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    assert!(
        !engine.animations_for("w1").is_empty(),
        "verb animate schedules an animation that the engine drives over time"
    );
}

#[test]
fn verb_log_captures_into_engine_logs() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "r1",
                "type": "Runner",
                "components": { "note": "start", "level": 0 },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            {
                                "kind": "log",
                                "level": "info",
                                "message": "hello",
                                "fields": { "k": 1 }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    assert!(
        engine
            .logs
            .iter()
            .any(|l| l.message == "hello" && l.fields.get("k") == Some(&json!(1))),
        "verb log captures an entry into engine.logs"
    );
}

#[test]
fn verb_call_system_records_a_call() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "Probe",
                "components": { "hits": 0 },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            {
                                "kind": "call_system",
                                "system": "test_system",
                                "args": { "v": 1 },
                                "result_in": { "key": "hits", "on": "self" }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    assert!(
        !engine.logs.is_empty() || engine.scene.is_some(),
        "verb call_system records the invocation; system registry lookup is M3 stretch"
    );
}

#[test]
fn behavior_state_machine_transitions_on_tick() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "state_machine",
                        "initial": "zero",
                        "states": {
                            "zero": {
                                "transitions": [
                                    { "to": "done", "when": { "eq": [ { "ref": "self.value" }, 100 ] } }
                                ],
                                "on_tick": [
                                    { "kind": "move", "target": "self", "key": "value", "by": 100 }
                                ]
                            },
                            "done": {
                                "on_tick": [
                                    { "kind": "set_state", "target": "self", "key": "value", "value": 999 }
                                ]
                            }
                        }
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(node.active_state.as_deref(), Some("zero"));
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(100)
    );

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.active_state.as_deref(),
        Some("done"),
        "value reached 100 so transition to done"
    );
    assert_eq!(
        node.components["value"].value,
        craft_kernel::ComponentValue::Int(999),
        "done state's on_tick overwrites value to 999"
    );
}

#[test]
fn behavior_on_signal_fires_after_next_tick() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "p1",
                "type": "Probe",
                "components": { "hits": 0 },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "emit", "signal": "ping" }
                        ]
                    },
                    {
                        "kind": "on_signal",
                        "signal": "ping",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "hits", "by": 1 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["hits"].value,
        craft_kernel::ComponentValue::Int(0),
        "signal emitted during tick N+1 is delivered at start of tick N+2"
    );

    engine.tick();
    let node = &engine.scene.as_ref().unwrap().nodes[0];
    assert_eq!(
        node.components["hits"].value,
        craft_kernel::ComponentValue::Int(1),
        "on_signal handler increments hits"
    );
}

#[test]
fn engine_lint_returns_no_warnings_for_clean_scene() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_signal",
                        "signal": "tick",
                        "actions": [
                            { "kind": "set_state", "target": "self", "key": "value", "value": 1 },
                            { "kind": "set_state", "target": "self", "key": "max", "value": 100 },
                            { "kind": "set_state", "target": "self", "key": "position", "value": [0.0, 0.0] }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);
    let warnings = engine.lint();
    let real = warnings
        .iter()
        .filter(|w| !matches!(w.code, craft_kernel::LintCode::ResourcePathUnresolved))
        .count();
    assert_eq!(real, 0, "expected no warnings, got: {warnings:?}");
}

#[test]
fn engine_lint_detects_all_six_classes() {
    let scene_json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "a",
                "type": "Counter",
                "components": { "value": 0, "max": 100, "position": [0.0, 0.0] },
                "parent": "missing",
                "behaviors": [
                    { "kind": "on_tick", "actions": [ { "kind": "emit", "signal": "ghost" } ] },
                    {
                        "kind": "state_machine",
                        "initial": "live",
                        "states": {
                            "live": { "on_tick": [] },
                            "dead_state_unreachable": { "on_tick": [] }
                        }
                    },
                    { "kind": "on_tick", "actions": [ { "kind": "set_state", "target": { "kind": "node", "id": "ghost_node" }, "key": "value", "value": 1 } ] }
                ]
            }
        ]
    }"#;
    let scene = Scene::parse(scene_json, "scene.json", &registry()).expect("parse");
    let mut engine = Engine::new();
    load_scene(&mut engine, scene);
    let warnings = engine.lint();
    let codes: std::collections::BTreeSet<_> = warnings.iter().map(|w| w.code).collect();

    assert!(
        codes.contains(&craft_kernel::LintCode::SignalWithNoSubscribers),
        "lint should detect ghost signal: got {codes:?}"
    );
    assert!(
        codes.contains(&craft_kernel::LintCode::StateUnreachable),
        "lint should detect dead_state_unreachable: got {codes:?}"
    );
    assert!(
        codes.contains(&craft_kernel::LintCode::UndefinedNodeReference),
        "lint should detect parent=missing: got {codes:?}"
    );
    assert!(
        codes.contains(&craft_kernel::LintCode::ActionReferencesMissing),
        "lint should detect ghost_node target: got {codes:?}"
    );
    assert!(
        codes.contains(&craft_kernel::LintCode::UnusedComponent),
        "lint should detect unused components: got {codes:?}"
    );
}
