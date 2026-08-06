pub use craft_animation;
use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};
use craft_kernel::{Engine, ResourceRegistry};
pub use craft_physics;
use craft_replay::{InputFrame, Recorder, ReplayRunner};

fn anim_node(id: &str, with_animation: bool) -> Node {
    let mut components = std::collections::BTreeMap::new();
    components.insert(
        "position".to_string(),
        Component {
            value: ComponentValue::Vec2([0.0, 0.0]),
            kind: ComponentKind::Regular,
        },
    );
    if with_animation {
        components.insert(
            "animation".to_string(),
            Component {
                value: ComponentValue::String("res://anims/walk.json".into()),
                kind: ComponentKind::Regular,
            },
        );
        components.insert(
            "frame".to_string(),
            Component {
                value: ComponentValue::Int(0),
                kind: ComponentKind::Regular,
            },
        );
    }
    Node {
        id: id.to_string(),
        type_name: "Animated".to_string(),
        parent: None,
        components,
        behaviors: vec![],
        active_state: None,
        lua_class: None,
        destroyed: false,
    }
}

fn scene() -> Scene {
    Scene {
        kind: "scene".to_string(),
        name: "anim_test".to_string(),
        nodes: vec![anim_node("a", true), anim_node("b", false)],
        spawn_counter: 0,
    }
}

fn register_animation(engine: &mut Engine, frames: u64, fps: u64, loop_: bool) {
    engine.resources.register(
        "res://anims/walk.json",
        craft_kernel::serde_json::json!({
            "atlas": "res://sprites/walk.png",
            "frames": (0..frames)
                .map(|i| [i as f64 * 16.0, 0.0, 16.0, 16.0])
                .collect::<Vec<[f64; 4]>>(),
            "fps": fps,
            "loop": loop_,
        }),
    );
}

fn make_engine(frames: u64, fps: u64, loop_: bool) -> Engine {
    let mut engine = Engine::new();
    register_animation(&mut engine, frames, fps, loop_);
    engine.load_scene(scene());
    engine
}

fn frame_of(engine: &Engine, id: &str) -> Option<i64> {
    engine
        .scene()
        .unwrap()
        .find_node(id)
        .and_then(|n| match n.components.get("frame")?.value {
            ComponentValue::Int(i) => Some(i),
            _ => None,
        })
}

fn sprite_rect_of(engine: &Engine, id: &str) -> Option<[f64; 4]> {
    engine.scene().unwrap().find_node(id).and_then(|n| {
        match n.components.get("sprite_rect")?.value {
            ComponentValue::Rect(r) => Some(r),
            _ => None,
        }
    })
}

#[test]
fn animation_system_is_registered_via_linkage() {
    let _ = craft_animation::AnimationSystem;
    let engine = Engine::new();
    let names: Vec<&str> = engine.list_systems().iter().map(|s| s.name).collect();
    assert!(
        names.contains(&"AnimationSystem"),
        "AnimationSystem must be registered. Found: {names:?}"
    );
}

#[test]
fn frames_advance_every_tick_at_60fps_and_loop() {
    let mut engine = make_engine(3, 60, true);
    engine.tick();
    assert_eq!(frame_of(&engine, "a"), Some(1));
    assert_eq!(sprite_rect_of(&engine, "a"), Some([16.0, 0.0, 16.0, 16.0]));
    engine.tick();
    assert_eq!(frame_of(&engine, "a"), Some(2));
    engine.tick();
    assert_eq!(frame_of(&engine, "a"), Some(0), "3-frame loop wraps around");
    assert_eq!(sprite_rect_of(&engine, "a"), Some([0.0, 0.0, 16.0, 16.0]));
}

#[test]
fn fps_30_advances_every_second_tick() {
    let mut engine = make_engine(4, 30, true);
    engine.tick();
    assert_eq!(
        frame_of(&engine, "a"),
        Some(0),
        "first tick still on frame 0"
    );
    engine.tick();
    assert_eq!(frame_of(&engine, "a"), Some(1));
    engine.tick();
    engine.tick();
    assert_eq!(frame_of(&engine, "a"), Some(2));
}

#[test]
fn non_looping_animation_stops_on_last_frame() {
    let mut engine = make_engine(3, 60, false);
    for _ in 0..10 {
        engine.tick();
    }
    assert_eq!(frame_of(&engine, "a"), Some(2), "stays on last frame");
    assert_eq!(sprite_rect_of(&engine, "a"), Some([32.0, 0.0, 16.0, 16.0]));
}

#[test]
fn nodes_without_animation_are_untouched() {
    let mut engine = make_engine(3, 60, true);
    for _ in 0..5 {
        engine.tick();
    }
    assert_eq!(frame_of(&engine, "b"), None, "no frame component written");
    assert_eq!(sprite_rect_of(&engine, "b"), None);
}

#[test]
fn animation_replay_is_byte_equal_across_reruns() {
    let scene = scene();
    let mut resources = ResourceRegistry::new();
    resources.register(
        "res://anims/walk.json",
        craft_kernel::serde_json::json!({
            "atlas": "res://sprites/walk.png",
            "frames": [[0.0, 0.0, 16.0, 16.0], [16.0, 0.0, 16.0, 16.0], [32.0, 0.0, 16.0, 16.0]],
            "fps": 30,
            "loop": true,
        }),
    );

    let mut recorder = Recorder::start(&scene, 0, &resources).expect("recorder");
    let mut engine = Engine::new();
    register_animation(&mut engine, 3, 30, true);
    engine.load_scene(scene.clone());
    for t in 0..20u64 {
        engine.tick();
        let signals = engine.take_last_signals();
        let hash = engine.state_hash();
        recorder.record_tick(t, &InputFrame::empty(), hash, signals);
    }
    let recording = recorder.finish();
    let recorded: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();

    for run in 0..5 {
        let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
        let hashes: Vec<u64> = runner.run_all().iter().map(|ev| ev.post_hash).collect();
        assert_eq!(
            hashes, recorded,
            "rerun #{run} diverged — animation advance must be deterministic (ADR 0006)"
        );
    }
}

#[test]
fn animation_and_physics_share_the_tick_deterministically() {
    use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};

    fn moving_node(id: &str) -> Node {
        let mut components = std::collections::BTreeMap::new();
        for (k, v) in [
            ("position", ComponentValue::Vec2([0.0, 0.0])),
            ("velocity", ComponentValue::Vec2([0.5, 0.0])),
            ("hitbox", ComponentValue::Vec2([0.4, 0.4])),
        ] {
            components.insert(
                k.to_string(),
                Component {
                    value: v,
                    kind: ComponentKind::Regular,
                },
            );
        }
        Node {
            id: id.to_string(),
            type_name: "Ball".to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }
    }

    let scene = Scene {
        kind: "scene".to_string(),
        name: "order_test".to_string(),
        nodes: vec![anim_node("a", true), moving_node("m")],
        spawn_counter: 0,
    };

    let run = || {
        let mut engine = Engine::new();
        register_animation(&mut engine, 3, 30, true);
        engine.load_scene(scene.clone());
        for _ in 0..10 {
            engine.tick();
        }
        let a = engine.scene().unwrap().find_node("a").unwrap().clone();
        let m = engine.scene().unwrap().find_node("m").unwrap().clone();
        (
            match &a.components.get("frame").unwrap().value {
                ComponentValue::Int(i) => *i,
                other => panic!("{other:?}"),
            },
            match &m.components.get("position").unwrap().value {
                ComponentValue::Vec2(v) => *v,
                other => panic!("{other:?}"),
            },
        )
    };

    let (f1, p1) = run();
    let (f2, p2) = run();
    assert_eq!(
        (f1, p1),
        (f2, p2),
        "animation+physics ordering must be stable"
    );
    assert!(f1 > 0, "animation must have advanced");
    assert!(p1[0] > 0.0, "physics must have integrated");
}
