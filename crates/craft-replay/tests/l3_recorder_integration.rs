use craft_kernel::ResourceRegistry;
use craft_replay::{ModuleRecord, Recorder, ReplayRunner};

fn scene() -> craft_kernel::Scene {
    craft_kernel::Scene {
        kind: craft_kernel::SCENE_KIND.to_string(),
        name: "t".to_string(),
        nodes: Vec::new(),
        spawn_counter: 0,
    }
}

#[test]
fn recorder_start_carries_empty_module_records() {
    let recorder = Recorder::start(&scene(), 0, &ResourceRegistry::new()).unwrap();
    assert!(recorder.module_records().is_empty());
    assert!(!recorder.rng_locked());
}

#[test]
fn recorder_start_validated_stores_module_records_and_rng_locked_flag() {
    let modules = vec![
        ModuleRecord {
            name: "lib.vec2".to_string(),
            version: "1.0.0".to_string(),
            sha256: "abc".to_string(),
        },
        ModuleRecord {
            name: "lib.math3d".to_string(),
            version: "0.2.0".to_string(),
            sha256: "def".to_string(),
        },
    ];
    let recorder =
        Recorder::start_validated(&scene(), 42, &ResourceRegistry::new(), &modules, true).unwrap();
    assert_eq!(recorder.module_records().len(), 2);
    assert!(recorder.rng_locked());
    let serialized = serde_json::to_string(recorder.recording()).unwrap();
    assert!(
        serialized.contains("lib.vec2"),
        "module records must persist in serialized recording: {serialized}"
    );
    assert!(
        serialized.contains("rng_locked"),
        "rng_locked must persist: {serialized}"
    );
}

#[test]
fn recorder_start_validated_rejects_empty_module_names_when_rng_locked() {
    let modules = vec![ModuleRecord {
        name: String::new(),
        version: "1.0.0".to_string(),
        sha256: "x".to_string(),
    }];
    let result = Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &modules, true);
    assert!(
        result.is_err(),
        "rng_locked recording must reject empty module names"
    );
}

#[test]
fn replay_runner_validates_module_records_returns_no_mismatches_on_match() {
    let modules = vec![ModuleRecord {
        name: "lib.foo".to_string(),
        version: "1.0.0".to_string(),
        sha256: "abc".to_string(),
    }];
    let recorder =
        Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &modules, true).unwrap();
    let recording =
        serde_json::from_str(&serde_json::to_string(recorder.recording()).unwrap()).unwrap();
    let runner = ReplayRunner::new(recording).unwrap();
    let mismatches = runner.validate_module_records(&modules);
    assert!(
        mismatches.is_empty(),
        "no mismatches expected: {mismatches:?}"
    );
    assert!(runner.rng_locked());
}

#[test]
fn replay_runner_detects_version_drift() {
    let recorded = vec![ModuleRecord {
        name: "lib.foo".to_string(),
        version: "1.0.0".to_string(),
        sha256: "abc".to_string(),
    }];
    let recorder =
        Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &recorded, true).unwrap();
    let recording =
        serde_json::from_str(&serde_json::to_string(recorder.recording()).unwrap()).unwrap();
    let runner = ReplayRunner::new(recording).unwrap();
    let current = vec![ModuleRecord {
        name: "lib.foo".to_string(),
        version: "2.0.0".to_string(),
        sha256: "abc".to_string(),
    }];
    let mismatches = runner.validate_module_records(&current);
    assert_eq!(mismatches.len(), 1, "{mismatches:?}");
    assert!(mismatches[0].contains("version drift"));
}

#[test]
fn replay_runner_detects_source_hash_drift() {
    let recorded = vec![ModuleRecord {
        name: "lib.bar".to_string(),
        version: "1.0.0".to_string(),
        sha256: "aaa".to_string(),
    }];
    let recorder =
        Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &recorded, true).unwrap();
    let recording =
        serde_json::from_str(&serde_json::to_string(recorder.recording()).unwrap()).unwrap();
    let runner = ReplayRunner::new(recording).unwrap();
    let current = vec![ModuleRecord {
        name: "lib.bar".to_string(),
        version: "1.0.0".to_string(),
        sha256: "bbb".to_string(),
    }];
    let mismatches = runner.validate_module_records(&current);
    assert_eq!(mismatches.len(), 1);
    assert!(mismatches[0].contains("hash drift"));
}

#[test]
fn replay_runner_detects_added_and_removed_modules() {
    let recorded = vec![ModuleRecord {
        name: "lib.used".to_string(),
        version: "1.0.0".to_string(),
        sha256: "x".to_string(),
    }];
    let recorder =
        Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &recorded, true).unwrap();
    let recording =
        serde_json::from_str(&serde_json::to_string(recorder.recording()).unwrap()).unwrap();
    let runner = ReplayRunner::new(recording).unwrap();
    let current = vec![
        ModuleRecord {
            name: "lib.used".to_string(),
            version: "1.0.0".to_string(),
            sha256: "x".to_string(),
        },
        ModuleRecord {
            name: "lib.added".to_string(),
            version: "1.0.0".to_string(),
            sha256: "y".to_string(),
        },
    ];
    let mismatches = runner.validate_module_records(&current);
    assert_eq!(mismatches.len(), 1);
    assert!(
        mismatches[0].contains("not used during recording"),
        "{mismatches:?}"
    );
}

#[test]
fn rng_locked_flag_round_trips_through_serde() {
    let modules = vec![ModuleRecord {
        name: "x".to_string(),
        version: "1".to_string(),
        sha256: "h".to_string(),
    }];
    let recorder =
        Recorder::start_validated(&scene(), 0, &ResourceRegistry::new(), &modules, true).unwrap();
    let json = serde_json::to_string(recorder.recording()).unwrap();
    let restored: craft_replay::Recording = serde_json::from_str(&json).unwrap();
    assert!(restored.meta.rng_locked);
    assert_eq!(restored.meta.module_records.len(), 1);
    assert_eq!(restored.meta.module_records[0].name, "x");
}
