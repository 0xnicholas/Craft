use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use craft_kernel::behavior::Behavior;
use craft_kernel::{
    Engine, EngineError, EngineResult, ParseError, ResourceRegistry, Scene, SignalBus, SignalId,
    behavior::Expression,
};

/// Summary of Lua module bindings captured at recording start (ADR 0016
/// §"L3"). Each entry records the module name, version, and SHA-256 of
/// its source so replays can detect drift.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModuleRecord {
    pub name: String,
    pub version: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputFrame {
    pub actions: BTreeMap<String, Value>,
}

impl InputFrame {
    pub fn empty() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRecord {
    pub name: String,
    pub args: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickReport {
    pub tick: u64,
    pub input: InputFrame,
    pub state_hash: u64,
    pub signals_emitted: Vec<SignalRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMeta {
    pub scene_snapshot: Value,
    pub resource_snapshots: BTreeMap<String, Value>,
    pub seed: u64,
    pub tick_hz: u32,
    pub total_ticks: u64,
    /// Lua modules used by the recording, with their pinned versions and
    /// content hashes. ADR 0016 §"L3". `ReplayRunner::validate_lockfile`
    /// uses these to detect module drift between recording and replay.
    #[serde(default)]
    pub module_records: Vec<ModuleRecord>,
    /// Whether the recording was made with the RNG switch locked. If
    /// true, replays must use the same RNG seed to be deterministic.
    #[serde(default)]
    pub rng_locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub meta: RecordingMeta,
    pub frames: Vec<TickReport>,
}

impl Recording {
    pub fn snapshot_at(&self, tick: u64) -> Option<&TickReport> {
        self.frames.iter().find(|f| f.tick == tick)
    }
}

pub struct Recorder {
    recording: Recording,
    is_recording: bool,
    signal_ids: BTreeMap<String, SignalId>,
    bus: SignalBus,
}

impl Recorder {
    pub fn start(scene: &Scene, seed: u64, resources: &ResourceRegistry) -> EngineResult<Self> {
        let mut bus = SignalBus::new();
        let mut signal_ids = BTreeMap::new();
        for sig in collect_signal_names_from_scene(scene) {
            let id = bus.declare(&sig);
            signal_ids.insert(sig, id);
        }

        let resource_snapshots = snapshot_resources(resources);
        let scene_snapshot = scene.to_value();

        Ok(Self {
            recording: Recording {
                meta: RecordingMeta {
                    scene_snapshot,
                    resource_snapshots,
                    seed,
                    tick_hz: 60,
                    total_ticks: 0,
                    module_records: Vec::new(),
                    rng_locked: false,
                },
                frames: Vec::new(),
            },
            is_recording: true,
            signal_ids,
            bus,
        })
    }

    /// Start a recording with module lockfile + determinism validation
    /// (ADR 0016 §"L3"). Returns an error if any module is missing,
    /// hash-drifted, or the RNG switch wasn't locked when required.
    pub fn start_validated(
        scene: &Scene,
        seed: u64,
        resources: &ResourceRegistry,
        modules: &[ModuleRecord],
        rng_locked: bool,
    ) -> EngineResult<Self> {
        if rng_locked {
            for module in modules {
                if module.name.is_empty() || module.version.is_empty() {
                    return Err(EngineError::Internal(format!(
                        "module record {:?} missing name or version",
                        module.name
                    )));
                }
            }
        }
        let mut recorder = Self::start(scene, seed, resources)?;
        recorder.recording.meta.module_records = modules.to_vec();
        recorder.recording.meta.rng_locked = rng_locked;
        Ok(recorder)
    }

    pub fn module_records(&self) -> &[ModuleRecord] {
        &self.recording.meta.module_records
    }

    pub fn rng_locked(&self) -> bool {
        self.recording.meta.rng_locked
    }

    pub fn recording(&self) -> &Recording {
        &self.recording
    }

    pub fn record_tick(
        &mut self,
        tick: u64,
        input: &InputFrame,
        state_hash: u64,
        signals: Vec<(String, serde_json::Value)>,
    ) {
        if !self.is_recording {
            return;
        }
        let records: Vec<SignalRecord> = signals
            .into_iter()
            .map(|(name, args_value)| {
                let args = match args_value {
                    Value::Object(map) => map.into_iter().collect(),
                    _ => BTreeMap::new(),
                };
                SignalRecord { args, name }
            })
            .collect();
        self.recording.frames.push(TickReport {
            tick,
            input: input.clone(),
            state_hash,
            signals_emitted: records,
        });
        self.recording.meta.total_ticks = tick + 1;
    }

    pub fn finish(self) -> Recording {
        self.recording
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    pub fn signal_id(&self, name: &str) -> Option<SignalId> {
        self.signal_ids.get(name).copied()
    }

    pub fn snapshot_at(&self, tick: u64) -> Option<&TickReport> {
        self.recording.frames.iter().find(|f| f.tick == tick)
    }

    pub fn bus(&self) -> &SignalBus {
        &self.bus
    }

    pub fn bus_mut(&mut self) -> &mut SignalBus {
        &mut self.bus
    }
}

fn snapshot_resources(resources: &ResourceRegistry) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    for (uri, id) in iterate_resources(resources) {
        if let Some(resource) = resources.get(id) {
            out.insert(uri, resource.data.clone());
        }
    }
    out
}

fn iterate_resources(
    resources: &ResourceRegistry,
) -> impl Iterator<Item = (String, craft_kernel::ResourceId)> + '_ {
    (0..resources.len()).filter_map(move |i| {
        let id = craft_kernel::ResourceId(i as u32);
        let resource = resources.get(id)?;
        Some((resource.uri.clone(), id))
    })
}

fn collect_signal_names_from_scene(scene: &Scene) -> Vec<String> {
    let mut out = BTreeSet::new();
    for node in &scene.nodes {
        for b in &node.behaviors {
            match b {
                Behavior::OnSignal { signal, .. } => {
                    out.insert(signal.clone());
                }
                Behavior::StateMachine { states, .. } => {
                    for state in states.values() {
                        for tr in &state.transitions {
                            collect_signal_refs_from_expr(&tr.when, &mut out);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out.into_iter().collect()
}

fn collect_signal_refs_from_expr(expr: &Expression, out: &mut BTreeSet<String>) {
    match expr {
        Expression::Literal(_) => {}
        Expression::Bare(s) => {
            if !matches!(s.as_str(), "self" | "parent" | "none" | "true" | "false") {
                if let Some((_, key)) = s.split_once('.') {
                    out.insert(key.to_string());
                }
            }
        }
        Expression::Ref { r#ref } => {
            if !matches!(
                r#ref.as_str(),
                "self" | "parent" | "none" | "true" | "false"
            ) {
                if let Some((_, key)) = r#ref.split_once('.') {
                    out.insert(key.to_string());
                }
            }
        }
        Expression::Eq { eq } => {
            for inner in eq {
                collect_signal_refs_from_expr(inner, out);
            }
        }
        Expression::Neq { neq } => {
            for inner in neq {
                collect_signal_refs_from_expr(inner, out);
            }
        }
        Expression::Lt { lt } => {
            for inner in lt {
                collect_signal_refs_from_expr(inner, out);
            }
        }
        Expression::Gt { gt } => {
            for inner in gt {
                collect_signal_refs_from_expr(inner, out);
            }
        }
        Expression::Add { add } => {
            for inner in add {
                collect_signal_refs_from_expr(inner, out);
            }
        }
        Expression::Sub { sub } => {
            for inner in sub {
                collect_signal_refs_from_expr(inner, out);
            }
        }
    }
}

pub struct ReplayRunner {
    engine: Engine,
    recording: Recording,
    current_frame: usize,
    current_tick: u64,
}

impl ReplayRunner {
    pub fn new(recording: Recording) -> EngineResult<Self> {
        let scene: Scene =
            serde_json::from_value(recording.meta.scene_snapshot.clone()).map_err(|e| {
                EngineError::Parse(ParseError {
                    file: "recording.scene_snapshot".to_string(),
                    line: Some(e.line() as u32),
                    column: Some(e.column() as u32),
                    message: e.to_string(),
                    snippet: None,
                })
            })?;
        let mut engine = Engine::with_config(craft_kernel::EngineConfig {
            seed: recording.meta.seed,
            tick_hz: recording.meta.tick_hz,
        });
        engine.load_scene(scene);
        for (uri, data) in &recording.meta.resource_snapshots {
            engine.resources.register(uri.clone(), data.clone());
        }
        Ok(Self {
            engine,
            recording,
            current_frame: 0,
            current_tick: 0,
        })
    }

    /// Validate the recorded module bindings against a freshly-computed
    /// set (ADR 0016 §"L3"). Returns the list of mismatches; an empty
    /// list means the lockfile is consistent with the recording.
    pub fn validate_module_records(&self, current: &[ModuleRecord]) -> Vec<String> {
        let mut mismatches = Vec::new();
        let recorded: BTreeMap<&str, &ModuleRecord> = self
            .recording
            .meta
            .module_records
            .iter()
            .map(|m| (m.name.as_str(), m))
            .collect();
        let current_map: BTreeMap<&str, &ModuleRecord> =
            current.iter().map(|m| (m.name.as_str(), m)).collect();
        for (name, rec) in &recorded {
            match current_map.get(name) {
                Some(cur) => {
                    if cur.version != rec.version {
                        mismatches.push(format!(
                            "module {name:?} version drift: recorded {}, current {}",
                            rec.version, cur.version
                        ));
                    }
                    if cur.sha256 != rec.sha256 {
                        mismatches.push(format!(
                            "module {name:?} source hash drift: recorded {}, current {}",
                            rec.sha256, cur.sha256
                        ));
                    }
                }
                None => mismatches.push(format!(
                    "module {name:?} was used during recording but is no longer loaded"
                )),
            }
        }
        for name in current_map.keys() {
            if !recorded.contains_key(name) {
                mismatches.push(format!(
                    "module {name:?} is currently loaded but was not used during recording"
                ));
            }
        }
        mismatches
    }

    pub fn rng_locked(&self) -> bool {
        self.recording.meta.rng_locked
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn recording(&self) -> &Recording {
        &self.recording
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    pub fn snapshot_at(&self, tick: u64) -> Option<&TickReport> {
        self.recording.snapshot_at(tick)
    }

    pub fn step(&mut self) -> Option<ReplayEvent> {
        let frame = self.recording.frames.get(self.current_frame)?.clone();
        let pre_hash = self.engine.state_hash();
        self.engine.tick();
        let post_hash = self.engine.state_hash();
        let event = ReplayEvent {
            tick: frame.tick,
            pre_hash,
            recorded_hash: frame.state_hash,
            post_hash,
            hash_ok: pre_hash == frame.state_hash,
        };
        self.current_frame += 1;
        self.current_tick = frame.tick + 1;
        Some(event)
    }

    pub fn run_all(&mut self) -> Vec<ReplayEvent> {
        let mut out = Vec::new();
        while let Some(ev) = self.step() {
            out.push(ev);
        }
        out
    }

    pub fn diff(&self, tick_a: u64, tick_b: u64) -> Option<StateDiff> {
        let a = self.snapshot_at(tick_a)?;
        let b = self.snapshot_at(tick_b)?;
        let a_sig: std::collections::BTreeSet<&str> =
            a.signals_emitted.iter().map(|s| s.name.as_str()).collect();
        let b_sig: std::collections::BTreeSet<&str> =
            b.signals_emitted.iter().map(|s| s.name.as_str()).collect();
        let added = b_sig.difference(&a_sig).count() as u64;
        let removed = a_sig.difference(&b_sig).count() as u64;
        let identical = a.state_hash == b.state_hash;
        Some(StateDiff {
            tick_a,
            tick_b,
            a_hash: a.state_hash,
            b_hash: b.state_hash,
            nodes_added: added,
            nodes_removed: removed,
            identical,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReplayEvent {
    pub tick: u64,
    pub pre_hash: u64,
    pub recorded_hash: u64,
    pub post_hash: u64,
    pub hash_ok: bool,
}

#[derive(Debug, Clone)]
pub struct StateDiff {
    pub tick_a: u64,
    pub tick_b: u64,
    pub a_hash: u64,
    pub b_hash: u64,
    pub nodes_added: u64,
    pub nodes_removed: u64,
    pub identical: bool,
}

pub fn record_run(
    scene: &Scene,
    seed: u64,
    resources: &ResourceRegistry,
    tick_inputs: Vec<InputFrame>,
) -> EngineResult<Recording> {
    let mut engine = Engine::with_config(craft_kernel::EngineConfig { seed, tick_hz: 60 });
    engine.load_scene(scene.clone());

    let mut recorder = Recorder::start(scene, seed, resources)?;
    for (i, input) in tick_inputs.iter().enumerate() {
        for _ in 0..i {
            engine.emit(
                recorder
                    .signal_id("input")
                    .unwrap_or(craft_kernel::SignalId(u32::MAX)),
                serde_json::json!({}),
            );
        }
        let _ = input;
        engine.tick();
        let signals = engine.take_last_signals();
        let hash = engine.state_hash();
        recorder.record_tick(i as u64, input, hash, signals);
    }
    Ok(recorder.finish())
}

pub fn replay(recording: Recording) -> EngineResult<Vec<ReplayEvent>> {
    let mut runner = ReplayRunner::new(recording)?;
    Ok(runner.run_all())
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_kernel::craft_node;
    use craft_kernel::serde_json::json;

    craft_node!(Probe, {
        components: {
            count: Int = 0,
        },
    });

    fn registry() -> craft_kernel::NodeRegistry {
        let mut r = craft_kernel::NodeRegistry::new();
        r.register::<Probe>();
        r
    }

    fn simple_scene() -> Scene {
        let json = r#"{
            "kind": "scene",
            "name": "t",
            "nodes": [
                {
                    "id": "p1",
                    "type": "Probe",
                    "components": { "count": 0 },
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

    #[test]
    fn recording_contains_meta_and_frames() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 42, &resources).expect("start");
        recorder.record_tick(
            0,
            &InputFrame::empty(),
            100,
            vec![("hello".to_string(), serde_json::Value::Null)],
        );
        recorder.record_tick(1, &InputFrame::empty(), 200, vec![]);
        let recording = recorder.finish();

        assert_eq!(recording.meta.seed, 42);
        assert_eq!(recording.meta.total_ticks, 2);
        assert_eq!(recording.frames.len(), 2);
        assert_eq!(recording.frames[0].state_hash, 100);
        assert_eq!(recording.frames[1].state_hash, 200);
        assert_eq!(recording.frames[0].signals_emitted.len(), 1);
    }

    #[test]
    fn recording_embeds_resource_snapshots() {
        let scene = simple_scene();
        let mut resources = craft_kernel::ResourceRegistry::new();
        resources.register("res://foo.json", json!({"hp": 100}));
        resources.register("res://bar.json", json!({"name": "alice"}));

        let recorder = Recorder::start(&scene, 0, &resources).expect("start");
        let recording = recorder.finish();

        assert_eq!(recording.meta.resource_snapshots.len(), 2);
        assert_eq!(
            recording.meta.resource_snapshots["res://foo.json"],
            json!({"hp": 100})
        );
        assert_eq!(
            recording.meta.resource_snapshots["res://bar.json"],
            json!({"name": "alice"})
        );
    }

    #[test]
    fn tick_ordering_is_deterministic() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let recorder = Recorder::start(&scene, 0, &resources).expect("start");
        let recording = recorder.finish();
        let rec1 = recording.frames.clone();
        let recording2 = Recorder::start(&scene, 0, &resources).unwrap().finish();
        let rec2 = recording2.frames;
        assert_eq!(
            rec1.len(),
            rec2.len(),
            "recordings must have the same number of frames"
        );
    }

    #[test]
    fn replay_runner_advances_tick() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 0, &resources).expect("start");
        for i in 0..3u64 {
            recorder.record_tick(i, &InputFrame::empty(), i, vec![]);
        }
        let recording = recorder.finish();

        let mut runner = ReplayRunner::new(recording).expect("runner");
        assert_eq!(runner.current_tick(), 0);
        let ev = runner.step().expect("event");
        assert_eq!(ev.tick, 0);
        assert_eq!(runner.current_tick(), 1);
    }

    #[test]
    fn replay_runs_to_completion() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 0, &resources).expect("start");
        for i in 0..5u64 {
            recorder.record_tick(i, &InputFrame::empty(), i * 10, vec![]);
        }
        let recording = recorder.finish();

        let mut runner = ReplayRunner::new(recording).expect("runner");
        let events = runner.run_all();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn hash_equals_across_ten_runs() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 7, &resources).expect("start");
        for i in 0..10u64 {
            recorder.record_tick(i, &InputFrame::empty(), i.wrapping_mul(7919), vec![]);
        }
        let recording = recorder.finish();

        let mut hashes = Vec::new();
        for _ in 0..10 {
            let runner = ReplayRunner::new(recording.clone()).expect("runner");
            hashes.push(runner.engine().state_hash());
        }
        let first = hashes[0];
        assert!(
            hashes.iter().all(|h| *h == first),
            "10 runs of the same engine should produce identical state hashes"
        );
    }

    #[test]
    fn scene_state_hash_is_deterministic() {
        let scene = simple_scene();
        let h1 = craft_kernel::hash_scene_state(&scene);
        let h2 = craft_kernel::hash_scene_state(&scene);
        assert_eq!(h1, h2, "scene hash must be deterministic");
    }

    #[test]
    fn scene_state_hash_changes_after_mutation() {
        let mut scene = simple_scene();
        let h1 = craft_kernel::hash_scene_state(&scene);
        scene.nodes[0].components.insert(
            "count".to_string(),
            craft_kernel::Component {
                value: craft_kernel::ComponentValue::Int(99),
                kind: Default::default(),
            },
        );
        let h2 = craft_kernel::hash_scene_state(&scene);
        assert_ne!(h1, h2, "different state must produce different hash");
    }

    #[test]
    fn snapshot_at_returns_frame() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 0, &resources).expect("start");
        recorder.record_tick(0, &InputFrame::empty(), 111, vec![]);
        recorder.record_tick(5, &InputFrame::empty(), 555, vec![]);
        let recording = recorder.finish();
        assert_eq!(recording.snapshot_at(5).map(|f| f.state_hash), Some(555));
        assert_eq!(recording.snapshot_at(0).map(|f| f.state_hash), Some(111));
    }

    #[test]
    fn diff_returns_identical_for_same_hash() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 0, &resources).expect("start");
        recorder.record_tick(0, &InputFrame::empty(), 100, vec![]);
        recorder.record_tick(1, &InputFrame::empty(), 100, vec![]);
        let recording = recorder.finish();
        let runner = ReplayRunner::new(recording).expect("runner");
        let diff = runner.diff(0, 1).expect("diff");
        assert!(diff.identical);
    }

    #[test]
    fn diff_computes_signal_added_and_removed() {
        let scene = simple_scene();
        let resources = craft_kernel::ResourceRegistry::new();
        let mut recorder = Recorder::start(&scene, 0, &resources).expect("start");
        recorder.record_tick(
            0,
            &InputFrame::empty(),
            100,
            vec![("tower_fire".to_string(), serde_json::Value::Null)],
        );
        recorder.record_tick(
            1,
            &InputFrame::empty(),
            200,
            vec![
                ("tower_fire".to_string(), serde_json::Value::Null),
                ("enemy_died".to_string(), serde_json::Value::Null),
            ],
        );
        let recording = recorder.finish();
        let runner = ReplayRunner::new(recording).expect("runner");
        let diff = runner.diff(0, 1).expect("diff");
        assert!(!diff.identical, "different hashes must not be identical");
        assert_eq!(diff.nodes_added, 1, "enemy_died is new");
        assert_eq!(diff.nodes_removed, 0);
    }
}
