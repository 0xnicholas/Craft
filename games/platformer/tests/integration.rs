use std::path::PathBuf;

use craft_kernel::{ComponentValue, Engine, EngineConfig};
use craft_lua::{LuaEngineHook, LuaRuntime};
use craft_replay::{InputFrame, Recorder, ReplayRunner};
use platformer::load_scene;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn make_engine(seed: u64) -> Engine {
    let scene = load_scene().expect("scene");
    let mut engine = Engine::with_config(EngineConfig { seed, tick_hz: 60 });
    engine.resources.add_base_dir(manifest_dir().join("assets"));
    engine
        .resources
        .load("res://levels/level1.json")
        .expect("level resource");
    engine.load_scene(scene);
    let mut runtime = LuaRuntime::new(seed).expect("lua runtime");
    let source = std::fs::read_to_string(manifest_dir().join("scripts/player_controller.lua"))
        .expect("script source");
    runtime
        .load_class("player_controller", &source)
        .expect("load class");
    let mut hook = LuaEngineHook::new(runtime);
    hook.runtime_mut()
        .bind_node(engine.scene().unwrap(), "player", "player_controller")
        .expect("bind");
    engine.set_hook(Some(Box::new(hook)));
    engine
}

fn player_y(engine: &Engine) -> f64 {
    match engine
        .scene()
        .unwrap()
        .find_node("player")
        .unwrap()
        .components
        .get("position")
        .unwrap()
        .value
        .clone()
    {
        ComponentValue::Vec2([_, y]) => y,
        other => panic!("unexpected position {other:?}"),
    }
}

fn player_won(engine: &Engine) -> bool {
    matches!(
        engine
            .scene()
            .unwrap()
            .find_node("player")
            .unwrap()
            .components
            .get("won")
            .map(|c| &c.value),
        Some(ComponentValue::Bool(true))
    )
}

#[test]
fn scene_loads_with_all_node_types() {
    let scene = load_scene().expect("load");
    assert!(scene.nodes.iter().any(|n| n.id == "__input"));
    assert!(scene.nodes.iter().any(|n| n.id == "player"));
    assert!(scene.nodes.iter().any(|n| n.id == "level"));
    assert!(scene.nodes.iter().any(|n| n.id == "goal"));
    assert_eq!(
        scene
            .nodes
            .iter()
            .find(|n| n.id == "player")
            .unwrap()
            .lua_class
            .as_deref(),
        Some("player_controller")
    );
}

#[test]
fn player_falls_to_ground_and_rests() {
    let mut engine = make_engine(7);
    for _ in 0..120 {
        engine.tick();
    }
    let y = player_y(&engine);
    assert!(
        (y - 10.6).abs() < 1e-9,
        "player must rest on top of ground row 11 (11.0 - half 0.4); got y={y}"
    );
}

#[test]
fn jump_lifts_player_off_ground() {
    let mut engine = make_engine(7);
    for _ in 0..120 {
        engine.tick();
    }
    let rest_y = player_y(&engine);
    engine.set_input_action(true);
    for _ in 0..3 {
        engine.tick();
    }
    engine.set_input_action(false);
    let apex_y = player_y(&engine);
    assert!(
        apex_y < rest_y - 1.0,
        "jump must lift the player at least one tile; rest={rest_y}, apex={apex_y}"
    );
}

#[test]
fn scripted_playthrough_reaches_goal() {
    let mut engine = make_engine(7);
    engine.set_input_direction(1.0, 0.0);
    let mut won = false;
    for _ in 0..400 {
        engine.tick();
        if player_won(&engine) {
            won = true;
            break;
        }
    }
    assert!(
        won,
        "holding right must reach the goal within 400 ticks; final x-driven run did not win"
    );
}

#[test]
fn replay_with_same_input_is_byte_equal() {
    let scene = load_scene().expect("scene");
    let mut resources = craft_kernel::ResourceRegistry::new();
    resources.add_base_dir(manifest_dir().join("assets"));
    resources
        .load("res://levels/level1.json")
        .expect("level resource");
    let mut recorder = Recorder::start(&scene, 7, &resources).expect("recorder");
    let mut engine = make_engine(7);

    // Script: hold right the whole run; one jump at tick 30.
    let script: Vec<(f64, bool)> = (0..160).map(|t| (1.0, (30..36).contains(&t))).collect();

    for (t, (dx, action)) in script.iter().enumerate() {
        engine.set_input_direction(*dx, 0.0);
        engine.set_input_action(*action);
        engine.tick();
        let signals = engine.take_last_signals();
        let hash = engine.state_hash();
        recorder.record_tick(t as u64, &InputFrame::empty(), hash, signals);
    }
    let recording = recorder.finish();
    let recorded: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();

    let script_source =
        std::fs::read_to_string(manifest_dir().join("scripts/player_controller.lua"))
            .expect("script source");
    for run in 0..5 {
        let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
        let mut runtime = LuaRuntime::new(7).expect("lua runtime");
        runtime
            .load_class("player_controller", &script_source)
            .expect("load class");
        let mut hook = LuaEngineHook::new(runtime);
        let scene = runner.engine().scene().expect("scene").clone();
        hook.runtime_mut()
            .bind_node(&scene, "player", "player_controller")
            .expect("bind");
        runner.engine_mut().set_hook(Some(Box::new(hook)));

        let mut replayed = Vec::new();
        for (dx, action) in &script {
            runner.engine_mut().set_input_direction(*dx, 0.0);
            runner.engine_mut().set_input_action(*action);
            let ev = runner.step().expect("step");
            replayed.push(ev.post_hash);
        }
        assert_eq!(
            replayed, recorded,
            "replay run #{run} diverged from the recorded physics+input run (ADR 0006)"
        );
    }
}
