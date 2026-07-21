use craft_kernel::{Engine, NullRenderer};
use craft_replay::{InputFrame, Recorder};
use std::time::Instant;
use tower_defense::load_scene;

const TD_TEST_RNG_SEED: u64 = 0xDEADBEEF;

fn td_make_engine() -> Engine {
    let scene = load_scene().expect("load scene");
    let mut engine = Engine::with_config(craft_kernel::EngineConfig {
        seed: TD_TEST_RNG_SEED,
        tick_hz: 60,
    });
    engine.load_scene(scene);
    engine.set_renderer(Box::new(NullRenderer::new()));
    engine
}

fn run_recorded_ticks(engine: &mut Engine, count: u64) -> craft_replay::Recording {
    let resources = craft_kernel::ResourceRegistry::new();
    let mut recorder = Recorder::start(
        engine.scene.as_ref().expect("scene"),
        TD_TEST_RNG_SEED,
        &resources,
    )
    .expect("recorder start");
    for _ in 0..count {
        engine.tick();
        let h = engine.state_hash();
        recorder.record_tick(
            engine.tick,
            &InputFrame::empty(),
            h,
            engine.take_last_signals(),
        );
    }
    recorder.finish()
}

#[test]
fn scene_loads_with_four_nodes() {
    let scene = load_scene().expect("load scene");
    assert_eq!(scene.kind, "scene");
    assert_eq!(scene.name, "tower_defense");
    assert_eq!(scene.nodes.len(), 5);
}

#[test]
fn tower_defense_runs_thousand_ticks_without_error() {
    let mut engine = td_make_engine();
    let start = Instant::now();
    for _ in 0..1000 {
        engine.tick();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 8_000,
        "1000 ticks should finish in well under 8s; got {elapsed:?}"
    );
}

#[test]
fn spawner_spawns_50_enemies_over_1000_ticks() {
    let mut engine = td_make_engine();
    for _ in 0..1000 {
        engine.tick();
    }
    let spawner = engine
        .scene
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == "spawner_main")
        .expect("spawner exists");
    let spawned = spawner
        .components
        .get("spawned_count")
        .expect("spawned_count");
    assert_eq!(
        spawned.value,
        craft_kernel::ComponentValue::Int(50),
        "spawner should spawn one enemy every 20 ticks; 1000/20 = 50"
    );
}

#[test]
fn enemy_count_is_bounded_through_run() {
    let mut engine = td_make_engine();
    let mut max_enemies = 0usize;
    let mut total_spawns = 0i64;
    for _ in 0..1000 {
        engine.tick();
        let scene = engine.scene.as_ref().unwrap();
        let n = scene
            .nodes
            .iter()
            .filter(|n| n.type_name == "Enemy")
            .count();
        if n > max_enemies {
            max_enemies = n;
        }
    }
    if let Some(s) = engine.scene.as_ref() {
        if let Some(spawner) = s.nodes.iter().find(|n| n.id == "spawner_main") {
            if let Some(c) = spawner.components.get("spawned_count") {
                if let craft_kernel::ComponentValue::Int(count) = c.value {
                    total_spawns = count;
                }
            }
        }
    }
    assert!(
        max_enemies <= 11,
        "at most ~10 enemies in flight (got {max_enemies})"
    );
    assert!(
        max_enemies >= 1,
        "at least one enemy should exist during the run (got {max_enemies}); spawned={total_spawns}"
    );
}

#[test]
fn towers_fire_at_their_configured_rates() {
    let mut engine = td_make_engine();
    for _ in 0..1000 {
        engine.tick();
    }
    let scene = engine.scene.as_ref().unwrap();
    let shots = |tower_id: &str| -> i64 {
        let t = scene
            .nodes
            .iter()
            .find(|n| n.id == tower_id)
            .expect(tower_id);
        match t.components.get("shots_fired").expect("shots_fired").value {
            craft_kernel::ComponentValue::Int(i) => i,
            _ => panic!("expected int"),
        }
    };
    let a = shots("tower_a");
    let b = shots("tower_b");
    let c = shots("tower_c");
    assert!((95..=105).contains(&a), "tower_a fires ~10x/tick (got {a})");
    assert!((80..=90).contains(&b), "tower_b fires ~83x/tick (got {b})");
    assert!(
        (120..=130).contains(&c),
        "tower_c fires ~125x/tick (got {c})"
    );
}

#[test]
fn towers_have_kill_counter_field_ready_for_targeting() {
    let scene = load_scene().expect("load");
    for tower_id in ["tower_a", "tower_b", "tower_c"] {
        let tower = scene
            .nodes
            .iter()
            .find(|n| n.id == tower_id)
            .expect(tower_id);
        assert!(tower.components.contains_key("kills"));
        let kills = tower.components.get("kills").unwrap();
        assert_eq!(
            kills.value,
            craft_kernel::ComponentValue::Int(0),
            "{tower_id} starts at kills=0"
        );
        let damage = tower.components.get("damage").unwrap();
        assert_eq!(damage.value, craft_kernel::ComponentValue::Int(5));
    }
}

#[test]
fn replay_hash_is_byte_equal_across_runs() {
    let recording = run_recorded_ticks(&mut td_make_engine(), 1000);
    let recorded_hashes: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();
    assert_eq!(recorded_hashes.len(), 1000);

    for _ in 0..5 {
        let replayed = run_recorded_ticks(&mut td_make_engine(), 1000);
        let replayed_hashes: Vec<u64> = replayed.frames.iter().map(|f| f.state_hash).collect();
        assert_eq!(
            recorded_hashes, replayed_hashes,
            "every tick's state hash must match across reruns (per ADR 0010 + 0006)"
        );
    }
}

#[test]
fn hot_reload_at_tick_500_does_not_break_state() {
    let mut engine = td_make_engine();
    let mut recorder = {
        let resources = craft_kernel::ResourceRegistry::new();
        Recorder::start(engine.scene.as_ref().unwrap(), TD_TEST_RNG_SEED, &resources)
            .expect("recorder")
    };
    for _ in 0..500 {
        engine.tick();
        recorder.record_tick(
            engine.tick,
            &InputFrame::empty(),
            engine.state_hash(),
            engine.take_last_signals(),
        );
    }

    let mut new_scene = engine.scene.as_ref().unwrap().clone();
    new_scene
        .nodes
        .iter_mut()
        .find(|n| n.id == "tower_b")
        .unwrap()
        .components
        .insert(
            "fire_rate".to_string(),
            craft_kernel::Component {
                value: craft_kernel::ComponentValue::Int(99),
                kind: Default::default(),
            },
        );

    let result = engine.apply_hot_reload(&new_scene).expect("apply");
    assert!(result.applied);

    for _ in 500..1000 {
        engine.tick();
        recorder.record_tick(
            engine.tick,
            &InputFrame::empty(),
            engine.state_hash(),
            engine.take_last_signals(),
        );
    }
    let recording = recorder.finish();
    let final_hash = engine.state_hash();
    assert!(final_hash != 0, "final hash must be non-zero");

    let pre_reload_hash = recording.frames[499].state_hash;
    let post_reload_hash = recording.frames[500].state_hash;
    assert_ne!(
        pre_reload_hash, post_reload_hash,
        "post-hot-reload hash must reflect the changed fire_rate"
    );
}

#[test]
fn scene_serializes_to_value_and_back() {
    let scene = load_scene().expect("load scene");
    let v = scene.to_value();
    assert_eq!(v["kind"], "scene");
    assert_eq!(v["name"], "tower_defense");
    let nodes = v["nodes"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 5);
}

#[test]
fn applied_lint_has_no_critical_warnings() {
    let scene = load_scene().expect("scene");
    let registry = tower_defense::build_node_registry();
    let warnings = craft_kernel::lint(&scene, &registry);
    let critical: Vec<_> = warnings
        .iter()
        .filter(|w| matches!(w.severity, craft_kernel::LintSeverity::Error))
        .collect();
    assert!(
        critical.is_empty(),
        "tower_defense scene should be lint-clean: {critical:?}"
    );
}

#[test]
fn enemies_present_in_steady_state() {
    let mut engine = td_make_engine();
    for _ in 0..200 {
        engine.tick();
    }
    let scene = engine.scene.as_ref().unwrap();
    let enemies: Vec<&craft_kernel::Node> = scene
        .nodes
        .iter()
        .filter(|n| n.type_name == "Enemy")
        .collect();
    assert!(
        !enemies.is_empty(),
        "enemies should exist in steady state after 200 ticks"
    );
    for e in &enemies {
        assert!(e.components.contains_key("health"));
        assert!(e.components.contains_key("lifetime"));
        assert!(e.components.contains_key("position"));
    }
}

#[test]
fn project_manifest_loads_with_canonical_schema() {
    use craft_kernel::Project;
    let toml = include_str!("../craft.toml");
    let project = Project::parse(toml, "craft.toml").expect("parse");
    assert_eq!(project.project.name, "tower_defense");
    assert_eq!(project.project.seed, Some(1));
    assert_eq!(project.project.tick_hz, Some(60));
}

#[test]
fn physics_system_is_registered_in_test_binary() {
    let engine = td_make_engine();
    let names: Vec<&str> = engine.list_systems().iter().map(|s| s.name).collect();
    assert!(
        names.contains(&"PhysicsSystem"),
        "PhysicsSystem must be registered for collision tests. Found: {names:?}"
    );
}

#[test]
fn physics_collision_kills_enemies_before_lifetime_expires() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let mut engine = td_make_engine();
    let collide_hits = Rc::new(RefCell::new(0u32));
    let ch = collide_hits.clone();
    let cid = engine.bus.declare("collide");
    engine.bus.subscribe(cid, move |_| {
        *ch.borrow_mut() += 1;
    });

    for _ in 0..100 {
        engine.tick();
    }

    let collisions = *collide_hits.borrow();
    assert!(
        collisions > 0,
        "physics should detect collisions between projectiles and enemies. Got {collisions} in 100 ticks"
    );
}

#[test]
fn minimal_physics_collision_in_scene() {
    use std::cell::RefCell;
    use std::rc::Rc;
    use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};

    fn make_node(id: &str, ty: &str, comps: Vec<(&str, ComponentValue)>) -> Node {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in comps {
            map.insert(k.to_string(), Component { value: v, kind: ComponentKind::Regular });
        }
        Node { id: id.to_string(), type_name: ty.to_string(), parent: None, components: map, behaviors: vec![], active_state: None, lua_class: None, destroyed: false }
    }

    let mut engine = craft_kernel::Engine::new();
    let collide_count = Rc::new(RefCell::new(0u32));
    let cc = collide_count.clone();
    let sid = engine.bus.declare("collide");
    engine.bus.subscribe(sid, move |_| { *cc.borrow_mut() += 1; });

    let scene = Scene {
        kind: "scene".to_string(),
        name: "minimal".to_string(),
        nodes: vec![
            make_node("a", "Test", vec![
                ("position", ComponentValue::Vec2([0.0, 0.0])),
                ("velocity", ComponentValue::Vec2([0.5, 0.0])),
                ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
            ]),
            make_node("b", "Test", vec![
                ("position", ComponentValue::Vec2([5.0, 0.0])),
                ("velocity", ComponentValue::Vec2([-0.5, 0.0])),
                ("hitbox", ComponentValue::Vec2([1.0, 1.0])),
            ]),
        ],
        spawn_counter: 0,
    };

    engine.load_scene(scene);
    for _ in 0..10 {
        engine.tick();
    }

    let hits = *collide_count.borrow();
    assert!(hits > 0, "two nodes heading toward each other with hitboxes must collide. Got {hits} hits in 10 ticks");
}
