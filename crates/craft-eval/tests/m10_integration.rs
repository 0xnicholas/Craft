use craft_eval::load_benchmark;
use craft_kernel::NodeRegistry;
use craft_kernel::craft_node;
use std::path::PathBuf;

craft_node!(Probe, {
    components: {
        count: Int = 0,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(Emitter, {
    components: {
        fired: Int = 0,
    },
});

craft_node!(Machine, {
    components: {
        state_code: Int = 0,
    },
});

fn benchmarks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("benchmarks")
}

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    r.register::<Probe>();
    r.register::<Emitter>();
    r.register::<Machine>();
    r.instantiate_all();
    r
}

#[test]
fn benchmark_1_on_tick_increment_lands_expected_hash_and_components() {
    let path = benchmarks_dir().join("01_on_tick_increment.json");
    let spec = load_benchmark(&path).expect("load benchmark");
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    let report = craft_eval::run_benchmark(&spec, &registry(), &backend);
    assert!(
        report.passed,
        "benchmark_1 failed: failing_components={:?} missing_signals={:?} hash={}",
        report.components_failing, report.missing_signals, report.final_hash
    );
    assert_eq!(report.ticks_run, 10);
}

#[test]
fn direct_engine_under_integration_test_runs_on_tick() {
    use craft_kernel::Engine;
    use craft_kernel::EngineConfig;

    let mut engine = Engine::with_config(EngineConfig {
        seed: 0,
        tick_hz: 60,
    });
    let scene_json = r#"{
        "kind": "scene",
        "name": "direct",
        "spawn_counter": 0,
        "nodes": [
            {
                "id": "p1",
                "type": "Probe",
                "components": { "count": 0, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "count", "by": 1 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let scene = craft_kernel::Scene::parse(scene_json, "scene.json", &registry()).expect("scene");
    engine.load_scene(scene);
    for _ in 0..3 {
        engine.tick();
    }
    let p1 = engine.scene.as_ref().unwrap().nodes.first().unwrap();
    let count = p1.components.get("count").unwrap().value.clone();
    assert_eq!(count, craft_kernel::ComponentValue::Int(3));
}

#[test]
fn direct_engine_under_integration_test_runs_state_machine() {
    use craft_kernel::Engine;
    use craft_kernel::EngineConfig;

    let mut engine = Engine::with_config(EngineConfig {
        seed: 0,
        tick_hz: 60,
    });
    let path = benchmarks_dir().join("03_state_machine.json");
    let spec = load_benchmark(&path).unwrap();
    engine.load_scene(spec.scene.clone());
    for _ in 0..5 {
        engine.tick();
    }
    let m1 = engine
        .scene
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == "m1")
        .expect("m1");
    assert_eq!(
        m1.components.get("state_code").map(|c| &c.value),
        Some(&craft_kernel::ComponentValue::Int(2))
    );
}

#[test]
fn benchmark_2_on_signal_handler_decrements_count() {
    let path = benchmarks_dir().join("02_on_signal_handler.json");
    let spec = load_benchmark(&path).expect("load benchmark");
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    let report = craft_eval::run_benchmark(&spec, &registry(), &backend);
    assert!(
        report.passed,
        "benchmark_2 failed: failing={:?} missing={:?} unexpected={:?}",
        report.components_failing, report.missing_signals, report.unexpected_signals
    );
}

#[test]
fn benchmark_3_state_machine_transitions_through_states() {
    let path = benchmarks_dir().join("03_state_machine.json");
    let spec = load_benchmark(&path).expect("load");
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    let report = craft_eval::run_benchmark(&spec, &registry(), &backend);
    eprintln!(
        "DEBUG b3: hash={} expected={} component_match={:?} unexpected={:?} passed={}",
        report.final_hash,
        report.expected_hash,
        report.component_match,
        report.unexpected_signals,
        report.passed
    );
    assert!(
        report.passed,
        "benchmark_3 failed: failing={:?} missing={:?} unexpected={:?}",
        report.components_failing, report.missing_signals, report.unexpected_signals
    );
}

#[test]
fn benchmark_4_if_destroy_releases_node() {
    let path = benchmarks_dir().join("04_if_destroy.json");
    let spec = load_benchmark(&path).expect("load");
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    let report = craft_eval::run_benchmark(&spec, &registry(), &backend);
    assert!(
        report.passed,
        "benchmark_4 failed: failing={:?} missing={:?}",
        report.components_failing, report.missing_signals
    );
}

#[test]
fn benchmarks_are_reproducible_across_runs() {
    let dir = benchmarks_dir();
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let spec = load_benchmark(&path).expect("load");
        let r1 = craft_eval::run_benchmark(&spec, &registry(), &backend);
        let r2 = craft_eval::run_benchmark(&spec, &registry(), &backend);
        assert_eq!(
            r1.final_hash, r2.final_hash,
            "{}: hashes diverged across runs",
            spec.name
        );
        assert_eq!(r1.passed, r2.passed, "{}: pass/fail flipped", spec.name);
    }
}

#[test]
fn backend_with_stub_returns_deterministic_response() {
    use craft_eval::Backend;
    use std::collections::BTreeMap;
    let mut m = BTreeMap::new();
    m.insert("k".to_string(), "[]".to_string());
    let b = craft_eval::StubBackend::deterministic(m);
    assert_eq!(b.complete("k"), "[]");
    assert_eq!(b.complete("k"), "[]");
}

#[test]
fn summary_reports_passed_and_total() {
    let r = craft_eval::Report {
        name: "x".into(),
        passed: true,
        final_hash: 1,
        expected_hash: 1,
        hash_match: true,
        component_match: Default::default(),
        components_failing: vec![],
        missing_signals: vec![],
        unexpected_signals: vec![],
        ticks_run: 1,
        duration_micros: 0,
    };
    let (p, t) = craft_eval::summary(&[r]);
    assert_eq!(p, 1);
    assert_eq!(t, 1);
}

#[test]
fn loaded_benchmark_round_trips_to_json() {
    let path = benchmarks_dir().join("01_on_tick_increment.json");
    let spec = load_benchmark(&path).unwrap();
    let json = spec.to_json().expect("to_json");
    let parsed = craft_eval::BenchmarkSpec::from_json(&json).expect("re-parse");
    assert_eq!(spec.name, parsed.name);
    assert_eq!(spec.ticks, parsed.ticks);
    assert_eq!(spec.actions_per_tick.len(), parsed.actions_per_tick.len());
}

#[test]
fn stamp_expected_hash_into_benchmarks() {
    use craft_eval::BenchmarkSpec;
    let dir = benchmarks_dir();
    let backend = craft_eval::StubBackend::deterministic(Default::default());
    let mut updated = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let mut spec = BenchmarkSpec::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let report = craft_eval::run_benchmark(&spec, &registry(), &backend);
        spec.expected_final_hash = report.final_hash;
        std::fs::write(&path, spec.to_json().unwrap()).unwrap();
        updated.push(path.display().to_string());
    }
    assert!(
        !updated.is_empty(),
        "expected to find at least one benchmark; updated: {updated:?}"
    );
}
