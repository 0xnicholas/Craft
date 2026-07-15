use std::fs;
use std::path::PathBuf;

use craft_lua::{
    DeterminismMode, DeterminismSwitches, LockEntry, Lockfile, LuaRuntime, ModuleLoader,
};
use tempfile::TempDir;

fn workspace() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let modules_dir = dir.path().join("modules");
    fs::create_dir_all(&modules_dir).expect("mkdir modules");
    fs::create_dir_all(modules_dir.join("lib")).expect("mkdir lib");
    (dir, modules_dir)
}

fn empty_engine() -> craft_kernel::Engine {
    use craft_kernel::{SCENE_KIND, Scene};
    let mut engine = craft_kernel::Engine::new();
    engine.load_scene(Scene {
        kind: SCENE_KIND.to_string(),
        name: "test".to_string(),
        nodes: Vec::new(),
        spawn_counter: 0,
    });
    engine
}

#[test]
fn lockfile_empty_default() {
    let lf = Lockfile::empty();
    assert!(lf.is_empty());
    assert_eq!(lf.len(), 0);
}

#[test]
fn lockfile_toml_round_trip() {
    let lf = Lockfile {
        entries: vec![
            LockEntry {
                name: "lib.vec2".to_string(),
                version: "1.0.0".to_string(),
                path: PathBuf::from("lib/vec2.lua"),
                sha256: "deadbeef".to_string(),
            },
            LockEntry {
                name: "lib.math3d".to_string(),
                version: "0.2.1".to_string(),
                path: PathBuf::from("lib/math3d.lua"),
                sha256: "cafebabe".to_string(),
            },
        ],
    };
    let s = lf.to_toml().expect("serialize");
    let parsed = Lockfile::from_toml(&s).expect("parse");
    assert_eq!(lf, parsed);
}

#[test]
fn module_loader_resolve_path() {
    let (dir, modules_dir) = workspace();
    fs::write(modules_dir.join("lib/vec2.lua"), "return {}").expect("write");
    let loader = ModuleLoader::new(modules_dir);
    let resolved = loader.resolve_path("lib.vec2").expect("found");
    assert!(resolved.ends_with("lib/vec2.lua"));
    drop(dir);
}

#[test]
fn require_loads_module_from_disk() {
    let (dir, modules_dir) = workspace();
    fs::write(
        modules_dir.join("lib/vec2.lua"),
        r#"
local M = {}
function M.add(a, b) return a + b end
return M
"#,
    )
    .expect("write");

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_modules_dir(modules_dir);
    runtime
        .run(
            &mut empty_engine(),
            r#"
            local vec2 = require("lib.vec2")
            assert(vec2.add(2, 3) == 5, "expected 2+3=5")
            "#,
        )
        .expect("require works");
    drop(dir);
}

#[test]
fn require_caches_module_in_package_loaded() {
    let (dir, modules_dir) = workspace();
    fs::write(
        modules_dir.join("lib/once.lua"),
        r#"
counter = (counter or 0) + 1
local M = { count = counter }
return M
"#,
    )
    .expect("write");

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_modules_dir(modules_dir);
    runtime
        .run(
            &mut empty_engine(),
            r#"
            local a = require("lib.once")
            local b = require("lib.once")
            assert(a.count == 1, "first require should set count=1")
            assert(b.count == 1, "second require should return cached module")
            assert(a == b, "should be the same table from package.loaded")
            "#,
        )
        .expect("cache works");
    drop(dir);
}

#[test]
fn record_module_stores_metadata_for_lockfile() {
    let (_dir, modules_dir) = workspace();
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_modules_dir(modules_dir);
    runtime
        .record_module(
            "lib.vec2",
            "1.0.0",
            "return { add = function(a,b) return a+b end }",
        )
        .unwrap();
    assert_eq!(runtime.loaded_module_count(), 1);
    let lf = runtime.lock_dependencies();
    assert_eq!(lf.len(), 1);
    let entry = &lf.entries[0];
    assert_eq!(entry.name, "lib.vec2");
    assert_eq!(entry.version, "1.0.0");
    let hash = craft_lua::modules::sha256_of_str("return { add = function(a,b) return a+b end }");
    assert_eq!(entry.sha256, hash);
}

#[test]
fn lockfile_validates_hashes_match_files() {
    let (dir, modules_dir) = workspace();
    let source = "return { x = 1 }";
    fs::write(modules_dir.join("lib/const.lua"), source).expect("write");

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_modules_dir(modules_dir.clone());
    runtime.record_module("lib.const", "1.0.0", source).unwrap();
    let lock_path = dir.path().join("luarocks.lock");
    runtime.write_lockfile_to_path(&lock_path).unwrap();

    let mut fresh = LuaRuntime::new(0).unwrap();
    fresh.set_modules_dir(modules_dir.clone());
    fresh
        .load_lockfile_from_path(&lock_path)
        .expect("load lockfile");
    fresh
        .validate_lockfile()
        .expect("clean lockfile must validate");

    fs::write(modules_dir.join("lib/const.lua"), "return { x = 999 }").expect("drift");
    let err = fresh.validate_lockfile().expect_err("drift must fail");
    assert!(err.contains("drift"), "error should mention drift: {err}");
    drop(dir);
}

#[test]
fn require_without_modules_dir_errors_cleanly() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    let result = runtime.run(&mut empty_engine(), r#"require("anything")"#);
    assert!(
        result.is_err(),
        "missing modules dir should be a no-op (searcher returns nil)"
    );
}

#[test]
fn set_determinism_recording_locks_math_random() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Recording).unwrap();
    assert!(runtime.switches().rng);
    assert!(runtime.determinism_mode().is_some());
}

#[test]
fn set_determinism_replay_locks_all_three() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Replay).unwrap();
    let s = runtime.switches();
    assert!(s.rng);
    assert!(s.float);
    assert!(s.order);
}

#[test]
fn set_determinism_development_keeps_all_off() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .set_determinism(DeterminismMode::Development)
        .unwrap();
    let s = runtime.switches();
    assert_eq!(
        s,
        DeterminismSwitches::default(),
        "development mode leaves all switches off"
    );
}

#[test]
fn recording_mode_replaces_math_random_with_engine_rng() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Recording).unwrap();
    runtime
        .run(
            &mut empty_engine(),
            r#"
            for _ = 1, 20 do
                local v = math.random(1, 1000)
                assert(v >= 1 and v <= 1000, "rng out of range")
            end
            "#,
        )
        .expect("rng in recording mode uses engine RNG");
}

#[test]
fn math_random_is_still_blocked_after_development_setup() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .set_determinism(DeterminismMode::Development)
        .unwrap();
    let result = runtime.run(&mut empty_engine(), r#"math.random()"#);
    assert!(
        result.is_err(),
        "Development mode preserves sandbox: math.random remains nil"
    );
}

#[test]
fn recording_log_captures_engine_calls() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Recording).unwrap();
    runtime
        .run(
            &mut empty_engine(),
            r#"
            for _ = 1, 3 do
                local v = math.random(1, 10)
                assert(v >= 1 and v <= 10, "rng out of range")
            end
            "#,
        )
        .expect("rng works");
    let log = runtime.take_recording_log();
    assert_eq!(log.calls.len(), 3, "should record 3 engine.rng calls");
    assert_eq!(log.calls[0].api, "engine.rng");
    assert_eq!(log.calls[0].args_summary, "1,10");
}

#[test]
fn determinism_switches_compose_independently() {
    let s1 = DeterminismSwitches {
        rng: true,
        float: false,
        order: false,
    };
    let s2 = s1;
    assert_eq!(s1, s2);
    let mode = DeterminismMode::Recording;
    assert!(mode.switches().rng);
    assert!(!mode.switches().float);
    assert!(!mode.switches().order);
}

#[test]
fn lockfile_lookup_finds_entry() {
    let lf = Lockfile {
        entries: vec![LockEntry {
            name: "foo".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("foo.lua"),
            sha256: "abc".to_string(),
        }],
    };
    assert!(lf.lookup("foo").is_some());
    assert!(lf.lookup("bar").is_none());
}

#[test]
fn validate_lockfile_without_modules_dir_errors() {
    let runtime = LuaRuntime::new(0).unwrap();
    let err = runtime.validate_lockfile().expect_err("must error");
    assert!(err.contains("set_modules_dir"));
}

#[test]
fn float_lock_exposes_craft_table_with_is_finite_sanitize() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Replay).unwrap();
    runtime
        .run(
            &mut empty_engine(),
            r#"
            assert(type(craft) == "table", "craft table must be exposed when float lock is on")
            assert(craft.is_finite(1.5) == true, "1.5 must be finite")
            assert(craft.is_finite(0/0) == false, "NaN must not be finite")
            assert(craft.sanitize(0/0, 0) == 0, "NaN sanitize default 0")
            assert(craft.sanitize(1.5, 0) == 1.5, "finite value passes through")
            local ok = pcall(function() craft.require_finite(1/0) end)
            assert(not ok, "inf must error under float lock")
            "#,
        )
        .expect("finite validation works");
}

#[test]
fn float_lock_rejects_non_finite_component_writes_via_noderef() {
    use craft_kernel::{Component, ComponentKind, ComponentValue, Engine, Node, SCENE_KIND, Scene};
    use std::collections::BTreeMap;
    let mut map = BTreeMap::new();
    map.insert(
        "speed".to_string(),
        Component {
            value: ComponentValue::Float(1.0),
            kind: ComponentKind::Regular,
        },
    );
    let scene = Scene {
        kind: SCENE_KIND.to_string(),
        name: "test".to_string(),
        nodes: vec![Node {
            id: "p".to_string(),
            type_name: "P".to_string(),
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

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Replay).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("p")
            local ok, err = pcall(function() n.speed = 1/0 end)
            assert(not ok, "inf must error under float lock")
            assert(err ~= nil, "error message must be present")
            assert(string.find(tostring(err), "float lock", 1, true) ~= nil,
                "expected error to mention float lock, got: " .. tostring(err))
            "#,
        )
        .expect("finite rejection");
}

#[test]
fn float_lock_off_allows_non_finite_values() {
    use craft_kernel::{Component, ComponentKind, ComponentValue, Engine, Node, SCENE_KIND, Scene};
    use std::collections::BTreeMap;
    let mut map = BTreeMap::new();
    map.insert(
        "speed".to_string(),
        Component {
            value: ComponentValue::Float(1.0),
            kind: ComponentKind::Regular,
        },
    );
    let scene = Scene {
        kind: SCENE_KIND.to_string(),
        name: "test".to_string(),
        nodes: vec![Node {
            id: "p".to_string(),
            type_name: "P".to_string(),
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

    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut engine,
            r#"
            local n = engine.get_node("p")
            n.speed = 1/0
            assert(n.speed > 1e308, "inf should be stored when float lock is off")
            "#,
        )
        .expect("no enforcement when lock is off");
}

#[test]
fn order_lock_makes_pairs_iterate_in_lexicographic_key_order() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime.set_determinism(DeterminismMode::Replay).unwrap();
    runtime
        .run(
            &mut empty_engine(),
            r#"
            local t = { zzz = 1, aaa = 2, mmm = 3, ["1"] = 4, ["2"] = 5 }
            local order = {}
            for k, v in pairs(t) do
                order[#order + 1] = tostring(k)
            end
            local str_keys = {}
            for _, k in ipairs(order) do
                if type(tonumber(k)) ~= "number" then
                    str_keys[#str_keys + 1] = k
                end
            end
            local sorted = { "aaa", "mmm", "zzz" }
            for i = 1, 3 do
                assert(str_keys[i] == sorted[i],
                    "string keys must be in lexicographic order; got " .. tostring(str_keys[i]))
            end
            "#,
        )
        .expect("pairs must iterate sorted");
}

#[test]
fn order_lock_off_yields_unspecified_iteration_order() {
    let mut runtime = LuaRuntime::new(0).unwrap();
    runtime
        .run(
            &mut empty_engine(),
            r#"
            local t = { zzz = 1, aaa = 2, mmm = 3 }
            local found_any = false
            for k, _ in pairs(t) do
                found_any = true
                break
            end
            assert(found_any, "pairs should still work without order lock")
            "#,
        )
        .expect("pairs without order lock should still work");
}
