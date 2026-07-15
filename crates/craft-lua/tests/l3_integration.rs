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
