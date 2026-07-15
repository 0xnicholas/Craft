use std::collections::BTreeMap;

use craft_kernel::{Component, ComponentKind, ComponentValue, Engine, Node, SCENE_KIND, Scene};
use craft_lua::LuaRuntime;

fn make_engine_with_node(
    id: &str,
    type_name: &str,
    components: &[(&str, ComponentValue)],
) -> Engine {
    let mut map = BTreeMap::new();
    for (k, v) in components {
        map.insert(
            (*k).to_string(),
            Component {
                value: v.clone(),
                kind: ComponentKind::Regular,
            },
        );
    }
    let scene = Scene {
        kind: SCENE_KIND.to_string(),
        name: "t".to_string(),
        nodes: vec![Node {
            id: id.to_string(),
            type_name: type_name.to_string(),
            parent: None,
            components: map,
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        }],
        spawn_counter: 0,
    };
    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine
}

const ENEMY_SOURCE: &str = r#"
Enemy = {}
Enemy.__index = Enemy

function Enemy.new(node)
    return setmetatable({ node = node, hp = 100 }, Enemy)
end

function Enemy:on_tick()
    self.hp = self.hp - 1
    self.node.hp = self.hp
end

function Enemy:on_spawn()
    self.spawn_tick = 999
end

function Enemy:on_signal(name, args)
    self.last_signal = name
    self.last_args = args
end
"#;

#[test]
fn load_class_detects_hooks() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let def = runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    assert!(def.hooks.on_tick);
    assert!(def.hooks.on_signal);
    assert!(def.hooks.on_spawn);
    assert_eq!(runtime.class_count(), 1);
}

#[test]
fn load_class_without_hooks_reports_none() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let def = runtime
        .load_class(
            "Static",
            r#"
Static = {}
function Static.new(node)
    return { node = node }
end
"#,
        )
        .unwrap();
    assert!(!def.hooks.on_tick);
    assert!(!def.hooks.on_signal);
    assert!(!def.hooks.on_spawn);
}

#[test]
fn load_class_fails_when_global_missing() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.load_class("NoSuch", r#"print("hello")"#);
    assert!(result.is_err(), "expected error for missing global");
}

#[test]
fn load_class_fails_when_new_missing() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.load_class(
        "NoNew",
        r#"
NoNew = {}
NoNew.__index = NoNew
"#,
    );
    assert!(result.is_err(), "expected error for missing `new`");
}

#[test]
fn bind_node_calls_on_spawn_immediately() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    assert_eq!(runtime.binding_count(), 1);
    let node = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(100)),
        "on_spawn must not pre-mutate component values"
    );
}

#[test]
fn tick_pre_pass_fires_on_tick_per_node() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let mut engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    runtime.tick_pre_pass(&mut engine).unwrap();
    runtime.tick_pre_pass(&mut engine).unwrap();
    let node = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(98)),
        "two on_tick calls should decrement hp by 2"
    );
}

#[test]
fn dispatch_signal_fires_on_signal_with_args() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let mut engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    runtime
        .dispatch_signal(&mut engine, "damage", &serde_json::json!({"amount": 25}))
        .unwrap();
}

#[test]
fn dispatch_spawn_fires_on_spawn_for_individual_node() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let mut engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    runtime.dispatch_spawn(&mut engine, "e1").unwrap();
    runtime.dispatch_spawn(&mut engine, "e1").unwrap();
}

#[test]
fn tick_pre_pass_skips_nodes_without_on_tick() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .load_class(
            "SpawnOnly",
            r#"
SpawnOnly = {}
SpawnOnly.__index = SpawnOnly

function SpawnOnly.new(node)
    return setmetatable({ node = node }, SpawnOnly)
end

function SpawnOnly:on_spawn()
    self.spawned = true
end
"#,
        )
        .unwrap();
    let mut engine = make_engine_with_node("e1", "SpawnOnly", &[("hp", ComponentValue::Int(50))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "SpawnOnly")
        .unwrap();
    runtime.tick_pre_pass(&mut engine).unwrap();
    let node = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(50)),
        "tick_pre_pass must not mutate node when class lacks on_tick"
    );
}

#[test]
fn unbind_node_removes_binding() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let mut engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    assert_eq!(runtime.binding_count(), 1);
    assert!(runtime.unbind_node("e1"));
    assert_eq!(runtime.binding_count(), 0);
    runtime.tick_pre_pass(&mut engine).unwrap();
    let node = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(100)),
        "post-unbind tick must not mutate"
    );
}

#[test]
fn reload_class_updates_methods_and_preserves_self_state() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let mut engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    runtime
        .bind_node(engine.scene().unwrap(), "e1", "Enemy")
        .unwrap();
    runtime.tick_pre_pass(&mut engine).unwrap();
    let node_before = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node_before.get_component_value("hp"),
        Some(&ComponentValue::Int(99))
    );

    let new_source = r#"
Enemy = Enemy or {}
Enemy.__index = Enemy

function Enemy.new(node)
    return setmetatable({ node = node, hp = 100 }, Enemy)
end

function Enemy:on_tick()
    self.hp = self.hp - 5
    self.node.hp = self.hp
end
"#;
    runtime.reload_class("Enemy", new_source).unwrap();
    runtime.tick_pre_pass(&mut engine).unwrap();
    let node_after = engine.scene().unwrap().find_node("e1").unwrap();
    assert_eq!(
        node_after.get_component_value("hp"),
        Some(&ComponentValue::Int(94)),
        "reloaded on_tick should decrement by 5 from the persisted self.hp=99"
    );
}

#[test]
fn bind_node_errors_on_missing_node() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.load_class("Enemy", ENEMY_SOURCE).unwrap();
    let engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    let result = runtime.bind_node(engine.scene().unwrap(), "ghost", "Enemy");
    assert!(result.is_err(), "expected error for missing node");
}

#[test]
fn bind_node_errors_on_unloaded_class() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let engine = make_engine_with_node("e1", "Enemy", &[("hp", ComponentValue::Int(100))]);
    let result = runtime.bind_node(engine.scene().unwrap(), "e1", "Unknown");
    assert!(result.is_err(), "expected error for unloaded class");
}

#[test]
fn reload_class_without_prior_load_errors() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.reload_class("Nope", "Nope = {}; function Nope.new(n) return {} end");
    assert!(result.is_err());
}

#[test]
fn engine_tick_fires_lua_pre_pass_before_json_behaviors() {
    
    use craft_lua::{LuaEngineHook, LuaRuntime};

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .load_class(
            "Tracker",
            r#"
Tracker = {}
Tracker.__index = Tracker

function Tracker.new(node)
    return setmetatable({ node = node, ticks = 0 }, Tracker)
end

function Tracker:on_tick()
    self.ticks = self.ticks + 1
    self.node.tick_count = self.ticks
end
"#,
        )
        .unwrap();

    let mut engine =
        make_engine_with_node("t1", "Tracker", &[("tick_count", ComponentValue::Int(0))]);
    let mut hook = LuaEngineHook::new(runtime);
    hook.runtime_mut()
        .bind_node(engine.scene().unwrap(), "t1", "Tracker")
        .unwrap();
    engine.set_hook(Some(Box::new(hook)));

    engine.tick();
    engine.tick();
    engine.tick();

    let node = engine.scene().unwrap().find_node("t1").unwrap();
    assert_eq!(
        node.get_component_value("tick_count"),
        Some(&ComponentValue::Int(3)),
        "Lua on_tick must fire exactly 3 times across 3 engine ticks"
    );
}

#[test]
fn engine_dispatch_signal_calls_lua_on_signal_with_args() {
    
    use craft_lua::{LuaEngineHook, LuaRuntime};

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .load_class(
            "Damage",
            r#"
Damage = {}
Damage.__index = Damage

function Damage.new(node)
    return setmetatable({ node = node }, Damage)
end

function Damage:on_signal(name, args)
    if name == "damage" and args.amount then
        self.node.hp = self.node.hp - args.amount
    end
end
"#,
        )
        .unwrap();

    let mut engine = make_engine_with_node("target", "Damage", &[("hp", ComponentValue::Int(100))]);
    let mut hook = LuaEngineHook::new(runtime);
    hook.runtime_mut()
        .bind_node(engine.scene().unwrap(), "target", "Damage")
        .unwrap();
    engine.set_hook(Some(Box::new(hook)));

    engine.signal_bus_mut().declare("damage");
    engine.queue_signal("damage", serde_json::json!({"amount": 30}));
    engine.tick();
    engine.tick();

    let node = engine.scene().unwrap().find_node("target").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(70)),
        "on_signal should subtract 30 from hp once when delivered in tick 1"
    );
}

#[test]
fn lua_pre_pass_runs_before_json_behaviors_in_same_tick() {
    use craft_kernel::{Behavior, Target};
    use craft_lua::{LuaEngineHook, LuaRuntime};

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .load_class(
            "Adder",
            r#"
Adder = {}
Adder.__index = Adder
function Adder.new(node)
    return setmetatable({ node = node, tag = "lua" }, Adder)
end
function Adder:on_tick()
    self.node.tick_tag = "lua"
end
"#,
        )
        .unwrap();

    let mut components = BTreeMap::new();
    components.insert(
        "tick_tag".to_string(),
        Component {
            value: ComponentValue::String("unset".to_string()),
            kind: ComponentKind::Regular,
        },
    );
    let scene = Scene {
        kind: SCENE_KIND.to_string(),
        name: "ordering".to_string(),
        nodes: vec![Node {
            id: "n1".to_string(),
            type_name: "Adder".to_string(),
            parent: None,
            components,
            behaviors: vec![Behavior::OnTick {
                actions: vec![craft_kernel::Action::SetState {
                    target: Target::This,
                    key: "tick_tag".to_string(),
                    value: serde_json::json!("json"),
                }],
            }],
            active_state: None,
            lua_class: None,
            destroyed: false,
        }],
        spawn_counter: 0,
    };
    let mut engine = Engine::new();
    engine.load_scene(scene);

    let mut hook = LuaEngineHook::new(runtime);
    hook.runtime_mut()
        .bind_node(engine.scene().unwrap(), "n1", "Adder")
        .unwrap();
    engine.set_hook(Some(Box::new(hook)));

    engine.tick();

    let node = engine.scene().unwrap().find_node("n1").unwrap();
    let observed = node.get_component_value("tick_tag").cloned();
    assert_eq!(
        observed,
        Some(ComponentValue::String("json".to_string())),
        "JSON behavior fires after Lua pre-pass and overwrites the value"
    );
}

#[test]
fn lua_pre_pass_skipped_when_no_hook_set() {
    let mut engine = make_engine_with_node("n1", "Anything", &[("hp", ComponentValue::Int(10))]);
    engine.tick();
    engine.tick();
    let node = engine.scene().unwrap().find_node("n1").unwrap();
    assert_eq!(
        node.get_component_value("hp"),
        Some(&ComponentValue::Int(10)),
        "no Lua hook means node is untouched"
    );
}
