use craft_kernel::craft_node;
use craft_kernel::serde_json::json;
use craft_kernel::{Engine, NodeRegistry, Scene};
use craft_replay::{InputFrame, Recorder, Recording, ReplayRunner};

craft_node!(CounterRP, {
    components: {
        count: Int = 0,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_node!(EmitterRP, {
    components: {
        count: Int = 0,
    },
});

fn registry() -> NodeRegistry {
    let mut r = NodeRegistry::new();
    r.register::<CounterRP>();
    r.register::<EmitterRP>();
    r
}

fn simple_scene() -> Scene {
    let json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "c1",
                "type": "CounterRP",
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
    Scene::parse(json, "scene.json", &registry()).expect("parse")
}

fn emit_scene() -> Scene {
    let json = r#"{
        "kind": "scene",
        "name": "t",
        "nodes": [
            {
                "id": "e1",
                "type": "EmitterRP",
                "components": { "count": 0 },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "emit", "signal": "tick" }
                        ]
                    },
                    {
                        "kind": "on_signal",
                        "signal": "tick",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "count", "by": 1 }
                        ]
                    }
                ]
            }
        ]
    }"#;
    Scene::parse(json, "scene.json", &registry()).expect("parse")
}

fn record_engine_run(scene: &Scene, ticks: u64) -> Recording {
    let resources = craft_kernel::ResourceRegistry::new();
    let mut engine = Engine::new();
    engine.load_scene(scene.clone());
    let mut recorder = Recorder::start(scene, 0, &resources).expect("start");
    for _ in 0..ticks {
        engine.tick();
        let signals = engine.take_last_signals();
        let hash = engine.state_hash();
        recorder.record_tick(engine.tick, &InputFrame::empty(), hash, signals);
    }
    recorder.finish()
}

#[test]
fn record_then_replay_produces_identical_state_hashes() {
    let scene = simple_scene();
    let recording = record_engine_run(&scene, 10);

    let recorded_hashes: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();
    let mut per_tick_runs: Vec<Vec<u64>> = Vec::new();
    for _ in 0..10 {
        let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
        let mut run_hashes = Vec::new();
        for ev in runner.run_all() {
            run_hashes.push(ev.post_hash);
        }
        per_tick_runs.push(run_hashes);
    }
    for (i, run) in per_tick_runs.iter().enumerate() {
        assert_eq!(
            run, &recorded_hashes,
            "rerun #{} produced hashes that do not match the recorded hashes; got {:?}, want {:?}",
            i, run, recorded_hashes
        );
    }
}

#[test]
fn state_hash_advances_with_each_tick() {
    let scene = simple_scene();
    let recording = record_engine_run(&scene, 5);
    let hashes: Vec<u64> = recording.frames.iter().map(|f| f.state_hash).collect();
    let mut uniq = hashes.clone();
    uniq.sort();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        5,
        "5 ticks with a counter increment should produce 5 distinct hashes; got {:?}",
        hashes
    );
}

#[test]
fn recording_embeds_full_resource_snapshots() {
    let scene = simple_scene();
    let mut resources = craft_kernel::ResourceRegistry::new();
    resources.register("res://data/sprite.json", json!({"texture": "hero.png"}));
    resources.register("res://data/audio.json", json!({"volume": 0.8}));

    let recorder = Recorder::start(&scene, 0, &resources).expect("start");
    let recording = recorder.finish();

    assert_eq!(recording.meta.resource_snapshots.len(), 2);
    assert_eq!(
        recording.meta.resource_snapshots["res://data/sprite.json"],
        json!({"texture": "hero.png"})
    );
    assert_eq!(
        recording.meta.resource_snapshots["res://data/audio.json"],
        json!({"volume": 0.8})
    );

    let json = serde_json::to_string(&recording).expect("serialize");
    let roundtrip: Recording = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(roundtrip.meta.resource_snapshots.len(), 2);
}

#[test]
fn tick_ordering_is_stable_across_runs() {
    let scene = emit_scene();
    let recording1 = record_engine_run(&scene, 5);
    let recording2 = record_engine_run(&scene, 5);

    let hashes1: Vec<u64> = recording1.frames.iter().map(|f| f.state_hash).collect();
    let hashes2: Vec<u64> = recording2.frames.iter().map(|f| f.state_hash).collect();
    assert_eq!(
        hashes1, hashes2,
        "two runs of the same scene with the same input must produce identical per-tick hashes"
    );
}

#[test]
fn recording_meta_preserves_seed_and_tick_hz() {
    let scene = simple_scene();
    let resources = craft_kernel::ResourceRegistry::new();
    let recorder = Recorder::start(&scene, 42, &resources).expect("start");
    let recording = recorder.finish();
    assert_eq!(recording.meta.seed, 42);
    assert_eq!(recording.meta.tick_hz, 60);
}

#[test]
fn ten_reruns_produce_byte_equal_hashes() {
    let scene = simple_scene();
    let recording = record_engine_run(&scene, 20);

    let mut all_hashes: Vec<Vec<u64>> = Vec::new();
    for _ in 0..10 {
        let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
        let mut run_hashes = Vec::new();
        for ev in runner.run_all() {
            run_hashes.push(ev.post_hash);
        }
        all_hashes.push(run_hashes);
    }
    let first = &all_hashes[0];
    for (i, run) in all_hashes.iter().enumerate() {
        assert_eq!(
            run, first,
            "rerun #{} produced different hashes than rerun #0",
            i
        );
    }
}

#[test]
fn replay_advances_through_all_recorded_frames() {
    let scene = simple_scene();
    let recording = record_engine_run(&scene, 7);
    let expected_ticks: Vec<u64> = recording.frames.iter().map(|f| f.tick).collect();
    let mut runner = ReplayRunner::new(recording.clone()).expect("runner");
    let events = runner.run_all();
    assert_eq!(events.len(), 7);
    let actual_ticks: Vec<u64> = events.iter().map(|ev| ev.tick).collect();
    assert_eq!(actual_ticks, expected_ticks);
}
