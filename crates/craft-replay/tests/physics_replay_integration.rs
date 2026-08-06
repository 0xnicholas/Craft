use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use craft_kernel::Engine;
use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};
pub use craft_physics;
use craft_replay::{InputFrame, Recorder, Recording, ReplayRunner};

fn physics_node(id: &str, pos: [f64; 2], vel: [f64; 2]) -> Node {
    let mut components = BTreeMap::new();
    components.insert(
        "position".to_string(),
        Component {
            value: ComponentValue::Vec2(pos),
            kind: ComponentKind::Regular,
        },
    );
    components.insert(
        "velocity".to_string(),
        Component {
            value: ComponentValue::Vec2(vel),
            kind: ComponentKind::Regular,
        },
    );
    components.insert(
        "hitbox".to_string(),
        Component {
            value: ComponentValue::Vec2([1.0, 1.0]),
            kind: ComponentKind::Regular,
        },
    );
    Node {
        id: id.to_string(),
        type_name: "PhysicsProbe".to_string(),
        parent: None,
        components,
        behaviors: vec![],
        active_state: None,
        lua_class: None,
        destroyed: false,
    }
}

/// Two boxes moving toward each other at 1.0/tick: they overlap from
/// tick 4 to tick 6, so PhysicsSystem must integrate velocity and emit
/// `collide` signals — if it isn't in the loop, nothing here moves.
fn physics_scene() -> Scene {
    Scene {
        kind: "scene".to_string(),
        name: "physics_replay".to_string(),
        nodes: vec![
            physics_node("a", [0.0, 0.0], [0.5, 0.0]),
            physics_node("b", [5.0, 0.0], [-0.5, 0.0]),
        ],
        spawn_counter: 0,
    }
}

fn record_physics_run(scene: &Scene, ticks: u64) -> (Recording, u32) {
    let resources = craft_kernel::ResourceRegistry::new();
    let mut engine = Engine::new();
    engine.load_scene(scene.clone());

    let collide_count = Rc::new(RefCell::new(0u32));
    let cc = collide_count.clone();
    let cid = engine.bus.declare("collide");
    engine.bus.subscribe(cid, move |_| {
        *cc.borrow_mut() += 1;
    });

    let mut recorder = Recorder::start(scene, 0, &resources).expect("start");
    for _ in 0..ticks {
        engine.tick();
        let signals = engine.take_last_signals();
        let hash = engine.state_hash();
        recorder.record_tick(engine.tick, &InputFrame::empty(), hash, signals);
    }
    (recorder.finish(), *collide_count.borrow())
}

/// The re-export above forces the linker to keep craft-physics's object
/// file so its `inventory` registration survives dead-code elimination
/// (dev-dependency crates are otherwise dropped as unreferenced).
#[test]
fn physics_system_is_registered_via_linkage() {
    let _ = craft_physics::PhysicsSystem;
    let names: Vec<&str> = Engine::new()
        .list_systems()
        .iter()
        .map(|s| s.name)
        .collect();
    assert!(
        names.contains(&"PhysicsSystem"),
        "PhysicsSystem must be registered once craft-physics is linked. Found: {names:?}"
    );
}

#[test]
fn physics_in_loop_replay_hashes_are_byte_equal_across_reruns() {
    let scene = physics_scene();
    let (recording, collisions) = record_physics_run(&scene, 30);

    assert!(
        collisions > 0,
        "physics must detect collisions during the recorded run; got {collisions} — \
         PhysicsSystem is not in the loop (is craft-physics linked?)"
    );

    let recorded: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();
    for i in 0..10 {
        let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
        let hashes: Vec<u64> = runner.run_all().iter().map(|ev| ev.post_hash).collect();
        assert_eq!(
            hashes, recorded,
            "rerun #{i} diverged from the recorded per-tick hashes — physics-in-loop \
             replay is not deterministic (ADR 0006)"
        );
    }
}

#[test]
fn physics_recordings_are_repeatable_between_independent_runs() {
    let scene = physics_scene();
    let (r1, _) = record_physics_run(&scene, 30);
    let (r2, _) = record_physics_run(&scene, 30);

    let h1: Vec<u64> = r1.frames.iter().map(|f| f.state_hash).collect();
    let h2: Vec<u64> = r2.frames.iter().map(|f| f.state_hash).collect();
    assert_eq!(
        h1, h2,
        "two independent recordings of the same physics scene must produce identical \
         per-tick hashes"
    );
}
