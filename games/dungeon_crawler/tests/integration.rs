use std::cell::RefCell;
use std::rc::Rc;

use craft_kernel::Engine;
use dungeon_crawler::load_scene;

fn make_engine() -> Engine {
    let scene = load_scene().expect("load");
    let mut engine = Engine::with_config(craft_kernel::EngineConfig {
        seed: 42,
        tick_hz: 10,
    });
    engine.load_scene(scene);
    engine
}

#[test]
fn scene_loads_with_all_node_types() {
    let scene = load_scene().expect("load");
    assert!(scene.nodes.iter().any(|n| n.id == "player"));
    assert!(scene.nodes.iter().any(|n| n.id == "enemy_a"));
    assert!(scene.nodes.iter().any(|n| n.id == "potion_a"));
    assert!(scene.nodes.iter().any(|n| n.id == "exit_stairs"));
}

#[test]
fn player_starts_with_full_health() {
    let scene = load_scene().expect("load");
    let player = scene.nodes.iter().find(|n| n.id == "player").unwrap();
    let hp = player
        .components
        .get("health")
        .and_then(|c| match &c.value {
            craft_kernel::ComponentValue::Int(i) => Some(*i),
            _ => None,
        })
        .unwrap();
    assert_eq!(hp, 100);
}

#[test]
fn enemies_have_health_and_damage() {
    let scene = load_scene().expect("load");
    for eid in ["enemy_a", "enemy_b", "enemy_c"] {
        let enemy = scene.nodes.iter().find(|n| n.id == eid).unwrap();
        let hp = enemy.components.get("health").unwrap();
        let dmg = enemy.components.get("damage").unwrap();
        assert!(matches!(hp.value, craft_kernel::ComponentValue::Int(30)));
        assert!(matches!(dmg.value, craft_kernel::ComponentValue::Int(5)));
        assert_eq!(enemy.lua_class.as_deref(), Some("scripts.enemy"));
    }
}

#[test]
fn player_moves_toward_exit_over_time() {
    let mut engine = make_engine();
    let start_pos = engine
        .scene()
        .unwrap()
        .find_node("player")
        .and_then(|n| n.components.get("position"))
        .and_then(|c| match &c.value {
            craft_kernel::ComponentValue::Vec2(v) => Some(*v),
            _ => None,
        })
        .unwrap();

    for _ in 0..10 {
        engine.tick();
    }

    let end_pos = engine
        .scene()
        .unwrap()
        .find_node("player")
        .and_then(|n| n.components.get("position"))
        .and_then(|c| match &c.value {
            craft_kernel::ComponentValue::Vec2(v) => Some(*v),
            _ => None,
        })
        .unwrap();
    assert!(end_pos[0] > start_pos[0], "player should move right");
}

#[test]
fn collision_detected_during_gameplay() {
    let mut engine = make_engine();
    let collide_count = Rc::new(RefCell::new(0u32));
    let cc = collide_count.clone();
    let cid = engine.bus.declare("collide");
    engine.bus.subscribe(cid, move |_| { *cc.borrow_mut() += 1; });

    for _ in 0..50 {
        engine.tick();
    }

    assert!(
        *collide_count.borrow() > 0,
        "at least one collision should occur during 50 ticks"
    );
}

#[test]
fn dungeon_runs_50_ticks_without_error() {
    let mut engine = make_engine();
    for _ in 0..50 {
        engine.tick();
    }
    let player = engine.scene().unwrap().find_node("player").unwrap();
    let hp = player
        .components
        .get("health")
        .and_then(|c| match &c.value {
            craft_kernel::ComponentValue::Int(i) => Some(*i),
            _ => None,
        })
        .unwrap();
    assert!(hp > 0, "player HP should be positive after 50 ticks");
}
