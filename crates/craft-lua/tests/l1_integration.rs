use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use craft_kernel::{Component, ComponentKind, ComponentValue, Engine, Node, SCENE_KIND, Scene};
use craft_lua::{LuaRuntime, QueryRegistry};

fn make_scene_with_player() -> Scene {
    let mut components = BTreeMap::new();
    components.insert(
        "hp".to_string(),
        Component {
            value: ComponentValue::Int(100),
            kind: ComponentKind::Regular,
        },
    );
    components.insert(
        "position".to_string(),
        Component {
            value: ComponentValue::Vec2([10.0, 20.0]),
            kind: ComponentKind::Regular,
        },
    );
    Scene {
        kind: SCENE_KIND.to_string(),
        name: "test_scene".to_string(),
        nodes: vec![Node {
            id: "player".to_string(),
            type_name: "Player".to_string(),
            parent: None,
            components,
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        }],
        spawn_counter: 0,
    }
}

fn fresh_engine() -> Engine {
    let mut engine = Engine::new();
    engine.load_scene(make_scene_with_player());
    engine
}

#[test]
fn node_userdata_exposes_id_field() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            assert(n ~= nil, "expected to find node")
            assert(n.id == "player", "id mismatch: " .. tostring(n.id))
            "#,
        )
        .unwrap();
}

#[test]
fn node_index_reads_existing_component() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            assert(n.hp == 100, "expected hp=100, got " .. tostring(n.hp))
            assert(n.position[1] == 10.0, "expected x=10, got " .. tostring(n.position[1]))
            assert(n.position[2] == 20.0, "expected y=20, got " .. tostring(n.position[2]))
            "#,
        )
        .unwrap();
}

#[test]
fn node_index_returns_nil_for_missing_component() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            assert(n.mana == nil, "expected nil for missing component")
            "#,
        )
        .unwrap();
}

#[test]
fn node_newindex_writes_existing_component() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            n.hp = 42
            assert(n.hp == 42, "expected hp=42 after write")
            "#,
        )
        .unwrap();
    let scene = engine.scene().unwrap();
    let player = scene.find_node("player").unwrap();
    assert_eq!(
        player.get_component_value("hp"),
        Some(&ComponentValue::Int(42))
    );
}

#[test]
fn node_newindex_adds_new_component_when_key_missing() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            n.mana = 7
            assert(n.mana == 7, "expected mana=7")
            assert(n:has_component("mana"), "expected mana to be present")
            "#,
        )
        .unwrap();
    let scene = engine.scene().unwrap();
    let player = scene.find_node("player").unwrap();
    assert_eq!(
        player.get_component_value("mana"),
        Some(&ComponentValue::Int(7))
    );
}

#[test]
fn emit_queues_signal_on_signal_bus() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            engine.emit("player_hit", { amount = 10, source = "trap" })
            "#,
        )
        .unwrap();
    let pending: Vec<_> = engine
        .signal_bus_mut()
        .drain()
        .into_iter()
        .map(|s| (s.id, s.payload))
        .collect();
    assert_eq!(pending.len(), 1);
    let id = engine.signal_bus().resolve("player_hit").unwrap();
    assert_eq!(pending[0].0, id);
    assert_eq!(pending[0].1["amount"], serde_json::json!(10));
    assert_eq!(pending[0].1["source"], serde_json::json!("trap"));
}

#[test]
fn spawn_creates_new_node_with_components() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local enemy = engine.spawn("Enemy", { hp = 50, position = { 3.0, 4.0 } })
            assert(enemy ~= nil, "spawn returned nil")
            assert(enemy.id == "__spawn_Enemy_0", "expected id '__spawn_Enemy_0', got " .. tostring(enemy.id))
            assert(enemy.hp == 50, "expected hp=50")
            assert(enemy.position[1] == 3.0, "expected x=3")
            assert(enemy.position[2] == 4.0, "expected y=4")
            "#,
        )
        .unwrap();
    let scene = engine.scene().unwrap();
    let enemy = scene.find_node("__spawn_Enemy_0").unwrap();
    assert_eq!(enemy.type_name, "Enemy");
    assert_eq!(
        enemy.get_component_value("hp"),
        Some(&ComponentValue::Int(50))
    );
}

#[test]
fn node_destroy_marks_destroyed_then_purge_removes_it() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            n:destroy()
            "#,
        )
        .unwrap();
    assert!(engine.scene().unwrap().find_node("player").is_none());
    let removed = engine.scene_mut().unwrap().purge_destroyed();
    assert_eq!(removed, 1);
}

#[test]
fn call_system_invokes_registered_query() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.register_query("math::add", |args| {
        let a = args.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
        let b = args.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(serde_json::json!(a + b))
    });
    runtime.register_query("math::vec2_dist_sq", |args| {
        let from = args
            .get("from")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let to = args
            .get("to")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let fx = from.first().and_then(|x| x.as_f64()).unwrap_or(0.0);
        let fy = from.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0);
        let tx = to.first().and_then(|x| x.as_f64()).unwrap_or(0.0);
        let ty = to.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0);
        let dx = tx - fx;
        let dy = ty - fy;
        Ok(serde_json::json!(dx * dx + dy * dy))
    });
    runtime
        .run(
            &mut engine,
            r#"
            local sum = engine.call_system("math::add", { a = 2, b = 40 })
            assert(sum == 42, "expected 42, got " .. tostring(sum))
            local d2 = engine.call_system("math::vec2_dist_sq", {
                from = { 0.0, 0.0 },
                to = { 3.0, 4.0 },
            })
            assert(d2 == 25.0, "expected 25.0, got " .. tostring(d2))
            "#,
        )
        .unwrap();
}

#[test]
fn call_system_errors_on_unknown_query() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"engine.call_system("nope", {})"#);
    assert!(result.is_err(), "expected error for unknown query");
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("nope"),
        "error should mention query name: {msg}"
    );
}

#[test]
fn rng_returns_values_in_range() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            for i = 1, 50 do
                local v = engine.rng(5, 10)
                assert(v >= 5 and v <= 10, "rng out of range: " .. tostring(v))
            end
            "#,
        )
        .unwrap();
}

#[test]
fn rng_is_deterministic_for_same_seed() {
    let run_with_seed = |seed: u64| -> Vec<i64> {
        let mut engine = fresh_engine();
        let mut runtime = LuaRuntime::new(seed).unwrap();
        let out: Rc<RefCell<Vec<i64>>> = Rc::new(RefCell::new(Vec::new()));
        let out_for_closure = out.clone();
        runtime.register_query("collect", move |args| {
            let v = args.get("v").and_then(|x| x.as_i64()).unwrap_or(0);
            out_for_closure.borrow_mut().push(v);
            Ok(serde_json::Value::Null)
        });
        runtime
            .run(
                &mut engine,
                r#"
                for i = 1, 20 do
                    engine.call_system("collect", { v = engine.rng(0, 1000000) })
                end
                "#,
            )
            .unwrap();
        out.borrow().clone()
    };

    let a = run_with_seed(42);
    let b = run_with_seed(42);
    assert_eq!(a, b, "rng must be deterministic for same seed");
    assert!(a.iter().all(|&v| (0..=1000000).contains(&v)));
    let c = run_with_seed(43);
    assert_ne!(a, c, "different seed should yield different stream");
}

#[test]
fn sandbox_blocks_io_os_debug() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    for script in [
        r#"io.open("/tmp/x", "w")"#,
        r#"os.execute("ls")"#,
        r#"debug.getinfo(1)"#,
        r#"dofile("/tmp/x.lua")"#,
        r#"loadfile("/tmp/x.lua")"#,
    ] {
        let result = runtime.run(&mut engine, script);
        assert!(result.is_err(), "sandbox leaked for: {script}");
    }
}

#[test]
fn sandbox_blocks_package_loadlib() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"package.loadlib("/tmp/x.so", "luaopen_x")"#);
    assert!(result.is_err(), "package.loadlib should be blocked");
}

#[test]
fn sandbox_blocks_math_random() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"math.random()"#);
    assert!(result.is_err(), "math.random should be blocked");
}

#[test]
fn script_without_engine_errors_when_no_scene_loaded() {
    let mut engine = Engine::new();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"engine.emit("x", {})"#);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("no scene loaded"));
}

#[test]
fn query_registry_exposes_registered_names() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.register_query("a::b", |_| Ok(serde_json::Value::Null));
    runtime.register_query("c::d", |_| Ok(serde_json::Value::Null));
    let registry = runtime.query_registry();
    let names: Vec<&str> = registry.names().collect();
    assert!(names.contains(&"a::b"));
    assert!(names.contains(&"c::d"));
}

#[test]
fn query_registry_default_is_empty() {
    let registry = QueryRegistry::new();
    assert!(!registry.contains("anything"));
    assert_eq!(registry.names().count(), 0);
}

#[test]
fn sandbox_preserves_safe_standard_library() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            assert(math.floor(3.7) == 3, "math.floor broken")
            assert(math.pi > 3.14 and math.pi < 3.15, "math.pi broken")
            assert(string.format("%d", 42) == "42", "string.format broken")
            assert(string.upper("abc") == "ABC", "string.upper broken")
            local t = {1, 2, 3}
            table.insert(t, 4)
            assert(#t == 4 and t[4] == 4, "table.insert broken")
            local ok, err = pcall(function() error("boom") end)
            assert(not ok, "pcall should catch errors")
            assert(string.find(err, "boom", 1, true) ~= nil, "pcall error message lost")
            "#,
        )
        .unwrap();
}

#[test]
fn rng_rejects_lo_greater_than_hi() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"engine.rng(10, 5)"#);
    assert!(result.is_err(), "rng should error when lo > hi");
    assert!(result.unwrap_err().to_string().contains("lo <= hi"));
}

#[test]
fn rng_rejects_range_exceeding_u64() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut engine, r#"engine.rng(0, 18446744073709551615)"#);
    assert!(
        result.is_err(),
        "rng should error when range exceeds u64 span"
    );
}

#[test]
fn node_id_is_read_only() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(
        &mut engine,
        r#"
        local n = engine.get_node("player")
        n.id = "evil"
        "#,
    );
    assert!(result.is_err(), "node.id should be read-only");
    assert!(result.unwrap_err().to_string().contains("read-only"));
    let scene = engine.scene().unwrap();
    let player = scene.find_node("player").unwrap();
    assert_eq!(
        player.id, "player",
        "id must not change after attempted write"
    );
    assert!(
        !player.components.contains_key("id"),
        "id assignment must not silently create an 'id' component"
    );
}

#[test]
fn vec2_rejects_mixed_table_keys() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(
        &mut engine,
        r#"
        local n = engine.get_node("player")
        local ok, err = pcall(function() n.position = { 1.0, 2.0, foo = "bar" } end)
        assert(not ok, "vec2 with extra keys should be rejected")
        assert(string.find(tostring(err), "non-integer key", 1, true) ~= nil,
            "error should mention non-integer key, got: " .. tostring(err))
        "#,
    );
    assert!(
        result.is_ok(),
        "script-level pcall assertion failed: {result:?}"
    );
}

#[test]
fn emit_propagates_deep_nesting_errors() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(
        &mut engine,
        r#"
        local deep = { { { { { { { { { { 1 } } } } } } } } } } }
        engine.emit("deep", deep)
        "#,
    );
    assert!(
        result.is_err(),
        "over-deep payload should error, not silently null"
    );
}

#[test]
fn cross_run_node_ref_detects_stale_generation() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            global_cached = engine.get_node("player")
            "#,
        )
        .unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            -- On a fresh run, the cached NodeRef is from a prior generation.
            -- Touching it must refuse instead of dereferencing freed memory.
            global_cached.hp = 99
            "#,
        )
        .expect_err("stale NodeRef must not silently succeed");
}

#[test]
fn two_node_refs_to_same_node_share_state() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local a = engine.get_node("player")
            local b = engine.get_node("player")
            assert(a ~= b, "two get_node calls should return distinct userdata handles")
            a.hp = 7
            assert(b.hp == 7, "second handle must observe first handle's write")
            "#,
        )
        .unwrap();
}

#[test]
fn node_ref_after_destroy_errors_on_access() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("player")
            n:destroy()
            local ok, err = pcall(function() local _ = n.hp end)
            assert(not ok, "accessing destroyed node must error")
            assert(string.find(tostring(err), "no longer exists", 1, true) ~= nil,
                "expected 'no longer exists' in error: " .. tostring(err))
            "#,
        )
        .unwrap();
}

#[test]
fn spawn_id_does_not_collide_with_user_ids() {
    let mut engine = fresh_engine();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local e = engine.spawn("Enemy", { hp = 1 })
            assert(e.id:sub(1, 8) == "__spawn_", "spawn id must use reserved prefix")
            assert(e.id ~= "Enemy_0", "spawn id must not collide with user-authored Enemy_0")
            -- Even though a user node later had id "Enemy_0", the spawn never overwrites it.
            engine.get_node("player").hp = 50
            assert(engine.get_node("player").hp == 50, "player untouched")
            "#,
        )
        .unwrap();
}
