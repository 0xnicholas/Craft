use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use craft_gpu::build_lua_hook;
use craft_kernel::Engine;
use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};

fn lua_source() -> &'static str {
    r#"
marker = marker or {}
marker.__index = marker

function marker.new(node)
    return setmetatable({ node = node }, marker)
end

function marker:on_tick()
    local me = self.node
    local t = me.ticks
    me.ticks = t + 1
end
"#
}

fn tmp_scripts_dir(src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "craft_gpu_lua_test_{}_{}",
        std::process::id(),
        src.len()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("marker.lua"), src).unwrap();
    dir
}

fn node(id: &str, lua_class: Option<&str>) -> Node {
    let mut components = BTreeMap::new();
    components.insert(
        "ticks".to_string(),
        Component {
            value: ComponentValue::Int(0),
            kind: ComponentKind::Regular,
        },
    );
    Node {
        id: id.to_string(),
        type_name: "Probe".to_string(),
        parent: None,
        components,
        behaviors: vec![],
        active_state: None,
        lua_class: lua_class.map(|s| s.to_string()),
        destroyed: false,
    }
}

fn scene_with(nodes: Vec<Node>) -> Scene {
    Scene {
        kind: "scene".to_string(),
        name: "lua_wiring".to_string(),
        nodes,
        spawn_counter: 0,
    }
}

fn ticks_of(engine: &Engine) -> i64 {
    match engine
        .scene()
        .unwrap()
        .find_node("p1")
        .unwrap()
        .components
        .get("ticks")
        .unwrap()
        .value
        .clone()
    {
        ComponentValue::Int(i) => i,
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn lua_hook_binds_class_and_runs_on_tick() {
    let scene = scene_with(vec![node("p1", Some("marker"))]);
    let scripts = tmp_scripts_dir(lua_source());
    let hook = build_lua_hook(&scene, &scripts, 7).expect("hook");

    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine.set_hook(Some(Box::new(hook)));
    engine.tick();
    engine.tick();
    engine.tick();
    assert_eq!(
        ticks_of(&engine),
        3,
        "Lua on_tick must run once per engine tick"
    );
}

#[test]
fn lua_hook_without_lua_class_nodes_does_nothing() {
    let scene = scene_with(vec![node("p1", None)]);
    let scripts = tmp_scripts_dir(lua_source());
    let hook = build_lua_hook(&scene, &scripts, 7).expect("hook");

    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine.set_hook(Some(Box::new(hook)));
    engine.tick();
    engine.tick();
    assert_eq!(ticks_of(&engine), 0, "no binding, no hook effect");
}

#[test]
fn missing_scripts_dir_yields_empty_hook() {
    let scene = scene_with(vec![node("p1", None)]);
    let missing = std::env::temp_dir().join("craft_gpu_no_such_scripts_dir_xyz");
    let hook =
        build_lua_hook(&scene, &missing, 7).expect("no lua_class nodes, missing dir is fine");
    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine.set_hook(Some(Box::new(hook)));
    engine.tick();
    assert_eq!(ticks_of(&engine), 0);
}

#[test]
fn lua_class_with_missing_script_fails_loudly() {
    let scene = scene_with(vec![node("p1", Some("marker"))]);
    let missing = std::env::temp_dir().join("craft_gpu_no_such_scripts_dir_xyz");
    let err = match build_lua_hook(&scene, &missing, 7) {
        Ok(_) => panic!("class not loaded must error"),
        Err(e) => e,
    };
    assert!(err.contains("marker"), "error names the class: {err}");
}

#[test]
fn physics_system_is_registered_via_linkage() {
    let _ = craft_physics::PhysicsSystem;
    let engine = Engine::new();
    let names: Vec<&str> = engine.list_systems().iter().map(|s| s.name).collect();
    assert!(
        names.contains(&"PhysicsSystem"),
        "craft-gpu must link physics. Found: {names:?}"
    );
}
