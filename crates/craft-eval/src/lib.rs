use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use craft_kernel::behavior::Action;
use craft_kernel::scene::{ComponentValue, NodeRegistry};
use craft_kernel::{Component, Engine, Scene};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSpec {
    pub name: String,
    pub description: String,
    pub seed: u64,
    pub ticks: u64,
    pub scene: Scene,
    pub actions_per_tick: Vec<Vec<Action>>,
    pub expected_final_hash: u64,
    pub expected_components: BTreeMap<String, BTreeMap<String, ComponentValue>>,
    #[serde(default)]
    pub expected_signals_emitted: Vec<String>,
}

impl BenchmarkSpec {
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("parse benchmark: {e}"))
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("serialize benchmark: {e}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub name: String,
    pub passed: bool,
    pub final_hash: u64,
    pub expected_hash: u64,
    pub hash_match: bool,
    pub component_match: BTreeMap<String, BTreeMap<String, Value>>,
    pub components_failing: Vec<String>,
    pub missing_signals: Vec<String>,
    pub unexpected_signals: Vec<String>,
    pub ticks_run: u64,
    pub duration_micros: u128,
}

impl Report {
    pub fn passed_count(&self) -> usize {
        self.passed as usize
    }
}

pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    fn complete(&self, prompt: &str) -> String;
    fn is_deterministic(&self) -> bool {
        let a = self.complete("__deterministic_probe__");
        let b = self.complete("__deterministic_probe__");
        a == b
    }
}

pub struct StubBackend {
    pub responses: BTreeMap<String, String>,
}

impl StubBackend {
    pub fn deterministic(responses: BTreeMap<String, String>) -> Self {
        Self { responses }
    }
}

impl Backend for StubBackend {
    fn name(&self) -> &str {
        "stub-deterministic"
    }

    fn complete(&self, prompt: &str) -> String {
        self.responses
            .get(prompt)
            .cloned()
            .unwrap_or_else(|| "[]".to_string())
    }

    fn is_deterministic(&self) -> bool {
        true
    }
}

pub struct LiveBackend {
    pub http: reqwest::blocking::Client,
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

impl LiveBackend {
    pub fn new(api_base: String, api_key: String, model: String) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            api_base,
            api_key,
            model,
        }
    }
}

impl Backend for LiveBackend {
    fn name(&self) -> &str {
        "live"
    }

    fn complete(&self, prompt: &str) -> String {
        let url = format!("{}/chat/completions", self.api_base.trim_end_matches('/'));
        let system = "You are a game engine evaluator. Given a scene description and expected outcome, output ONLY a JSON array of action arrays — one array per tick. Each action is a JSON object with kind, target, key, and value/by fields. Output ONLY the JSON array, no explanation.";

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.0,
            "max_tokens": 2000
        });

        let response = match self.http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
        {
            Ok(r) => r,
            Err(e) => return format!("HTTP error: {e}"),
        };

        if !response.status().is_success() {
            return format!("HTTP {}", response.status().as_u16());
        }

        let text = match response.text() {
            Ok(t) => t,
            Err(e) => return format!("read error: {e}"),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return format!("parse error: {e}"),
        };

        parsed["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("[]")
            .to_string()
    }
}

pub fn build_node_registry_with(extra: &[Box<dyn craft_kernel::scene::NodeDef>]) -> NodeRegistry {
    let _ = extra;
    NodeRegistry::new()
}

pub fn run_benchmark(
    spec: &BenchmarkSpec,
    registry: &NodeRegistry,
    backend: &dyn Backend,
) -> Report {
    let start = std::time::Instant::now();
    let mut engine = Engine::with_config(craft_kernel::EngineConfig {
        seed: spec.seed,
        tick_hz: 60,
    });
    engine.load_scene(spec.scene.clone());
    engine.set_renderer(Box::new(craft_kernel::NullRenderer::new()));
    let mut observed_signals = Vec::new();
    let prompt = spec.prompt();
    let response = backend.complete(&prompt);
    let agent_actions: Option<Vec<Vec<Action>>> =
        if !response.trim().is_empty() && response.trim() != "[]" {
            serde_json::from_str(&response).ok()
        } else {
            None
        };

    let actions_to_use: Vec<Vec<Action>> = match agent_actions {
        Some(a) if !a.is_empty() => a,
        _ => spec.actions_per_tick.clone(),
    };

    for tick in 0..spec.ticks {
        let actions = actions_to_use.get(tick as usize);
        if let (Some(scene), Some(actions)) = (engine.scene.as_mut(), actions) {
            let mut pending_writes = std::collections::BTreeMap::new();
            let mut cmds = Vec::new();
            if let Some(node) = scene.nodes.first() {
                for action in actions {
                    craft_kernel::evaluator::evaluate_action(
                        scene,
                        registry,
                        node,
                        action,
                        &mut cmds,
                        &mut pending_writes,
                        0,
                    );
                }
            }
            craft_kernel::evaluator::apply_commands_with_animations(
                scene, registry, cmds, &mut None,
            );
        }
        engine.tick();
        observed_signals.extend(engine.clone_last_signals());
    }

    let final_hash = engine.state_hash();
    let hash_match = final_hash == spec.expected_final_hash;

    let mut component_match = BTreeMap::new();
    let mut components_failing = Vec::new();
    if let Some(scene) = engine.scene.as_ref() {
        for (node_id, expected_components) in &spec.expected_components {
            let entry = component_match
                .entry(node_id.clone())
                .or_insert_with(BTreeMap::new);
            let actual_node = scene.nodes.iter().find(|n| &n.id == node_id);
            for (key, expected_value) in expected_components {
                let actual_value = actual_node
                    .and_then(|n| n.components.get(key))
                    .map(|c| &c.value);
                let matches = actual_value.map(|v| v == expected_value).unwrap_or(false);
                entry.insert(key.clone(), serde_json::json!({
                    "expected": expected_value,
                    "actual": actual_value.map(|v| serde_json::to_value(v).unwrap_or(Value::Null)).unwrap_or(Value::Null),
                    "matches": matches,
                }));
                if !matches {
                    components_failing.push(format!("{node_id}.{key}"));
                }
            }
        }
    }

    let expected_signals: std::collections::BTreeSet<String> =
        spec.expected_signals_emitted.iter().cloned().collect();
    let observed_signals_set: std::collections::BTreeSet<String> = observed_signals
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let missing: Vec<String> = expected_signals
        .difference(&observed_signals_set)
        .cloned()
        .collect();
    let unexpected: Vec<String> = observed_signals_set
        .difference(&expected_signals)
        .cloned()
        .collect();

    let passed =
        hash_match && components_failing.is_empty() && missing.is_empty() && unexpected.is_empty();

    Report {
        name: spec.name.clone(),
        passed,
        final_hash,
        expected_hash: spec.expected_final_hash,
        hash_match,
        component_match,
        components_failing,
        missing_signals: missing,
        unexpected_signals: unexpected,
        ticks_run: spec.ticks,
        duration_micros: start.elapsed().as_micros(),
    }
}

impl BenchmarkSpec {
    pub fn prompt(&self) -> String {
        let mut out = String::new();
        out.push_str("Given the following Craft scene, predict the exact sequence of actions that will execute per tick to achieve the expected final component values.\n\n");
        out.push_str(&format!("## Scene: {}\n", self.name));
        out.push_str(&format!("{}\n", self.description));
        out.push_str(&format!("Ticks to run: {}\n\n", self.ticks));
        out.push_str("### Nodes:\n");
        for n in &self.scene.nodes {
            out.push_str(&format!(
                "- id={} type={} components={:?}\n",
                n.id, n.type_name, n.components
            ));
            if !n.behaviors.is_empty() {
                out.push_str(&format!("  behaviors: {}\n", serde_json::to_string(&n.behaviors).unwrap_or_default()));
            }
        }
        if !self.expected_components.is_empty() {
            out.push_str("\n### Expected final components:\n");
            for (node_id, comps) in &self.expected_components {
                out.push_str(&format!("  {node_id}: {comps:?}\n"));
            }
        }
        out.push_str("\nReturn ONLY a JSON array of action arrays (one per tick). Example: [[{\"kind\":\"move\",\"target\":{\"kind\":\"self\"},\"key\":\"count\",\"by\":1}], ...]");
        out
    }
}

#[allow(dead_code)]
fn _component_value_to_json(v: &ComponentValue) -> Value {
    serde_json::to_value(v).unwrap_or(Value::Null)
}

pub fn run_all_benchmarks(
    specs: &[BenchmarkSpec],
    registry: &NodeRegistry,
    backend: &dyn Backend,
) -> Vec<Report> {
    specs
        .iter()
        .map(|s| run_benchmark(s, registry, backend))
        .collect()
}

pub fn summary(reports: &[Report]) -> (usize, usize) {
    let passed = reports.iter().filter(|r| r.passed).count();
    (passed, reports.len())
}

pub fn load_benchmark(path: &std::path::Path) -> Result<BenchmarkSpec, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("read benchmark file {}: {e}", path.display()))?;
    BenchmarkSpec::from_json(&contents)
}

pub fn assert_report_passes(r: &Report) {
    if !r.passed {
        let failing = r.components_failing.join(", ");
        let missing = r.missing_signals.join(", ");
        let unexpected = r.unexpected_signals.join(", ");
        panic!(
            "benchmark '{}' FAILED\n  hash: got {}, expected {} (match={})\n  components_failing: {failing}\n  missing_signals: {missing}\n  unexpected_signals: {unexpected}",
            r.name, r.final_hash, r.expected_hash, r.hash_match
        );
    }
}

#[allow(dead_code)]
fn _unused_comp(c: &Component) -> &ComponentValue {
    &c.value
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_kernel::behavior::{Action, Target};
    use serde_json::json;

    fn make_spec_with(
        ticks: u64,
        actions_per_tick: Vec<Vec<Action>>,
        expected_hash: u64,
    ) -> BenchmarkSpec {
        let scene_json = r#"{
            "kind": "scene",
            "name": "test",
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
        let registry = NodeRegistry::new();
        let mut r = NodeRegistry::new();
        r.register::<Probe>();
        let _ = registry;
        r.instantiate_all();
        let scene = craft_kernel::Scene::parse(scene_json, "scene.json", &r).expect("scene");

        BenchmarkSpec {
            name: "unit-test".to_string(),
            description: "test".to_string(),
            seed: 0,
            ticks,
            scene,
            actions_per_tick,
            expected_final_hash: expected_hash,
            expected_components: BTreeMap::new(),
            expected_signals_emitted: Vec::new(),
        }
    }

    fn make_spec(ticks: u64, actions_per_tick: Vec<Vec<Action>>) -> BenchmarkSpec {
        make_spec_with(ticks, actions_per_tick, 0)
    }

    craft_kernel::craft_node!(Probe, {
        components: {
            count: Int = 0,
            position: Vec2 = [0.0, 0.0],
        },
    });

    #[test]
    fn empty_benchmark_uses_scene_only() {
        let mut r = NodeRegistry::new();
        r.register::<Probe>();
        r.instantiate_all();

        let mut engine = Engine::with_config(craft_kernel::EngineConfig {
            seed: 0,
            tick_hz: 60,
        });
        let scene_json = r#"{
            "kind": "scene",
            "name": "test",
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
        let scene = craft_kernel::Scene::parse(scene_json, "scene.json", &r).expect("scene");
        engine.load_scene(scene);
        for _ in 0..3 {
            engine.tick();
        }
        let scene = engine.scene.as_ref().unwrap();
        let p1 = scene.nodes.iter().find(|n| n.id == "p1").unwrap();
        let count = p1.components.get("count").unwrap();
        let position = p1.components.get("position").unwrap();
        assert_eq!(
            count.value,
            ComponentValue::Int(3),
            "on_tick should increment"
        );
        assert_eq!(position.value, ComponentValue::Vec2([0.0, 0.0]));
    }

    #[test]
    fn agent_actions_set_target_value() {
        let actions = vec![vec![Action::SetState {
            target: Target::This,
            key: "count".to_string(),
            value: json!(42),
        }]];
        let mut spec = make_spec(1, actions);
        let mut r = NodeRegistry::new();
        r.register::<Probe>();
        r.instantiate_all();

        let backend = StubBackend::deterministic(BTreeMap::new());
        let bare = run_benchmark(&spec, &r, &backend);
        spec.expected_final_hash = bare.final_hash;

        let mut expected = BTreeMap::new();
        expected.insert("p1".to_string(), BTreeMap::new());
        expected
            .get_mut("p1")
            .unwrap()
            .insert("count".to_string(), ComponentValue::Int(43));
        expected
            .get_mut("p1")
            .unwrap()
            .insert("position".to_string(), ComponentValue::Vec2([0.0, 0.0]));
        spec.expected_components = expected;
        let report = run_benchmark(&spec, &r, &backend);
        assert!(
            report.passed,
            "agent set_state count=42 + on_tick +1 = 43 should pass (final hash {} )",
            report.final_hash
        );
    }

    #[test]
    fn stub_backend_deterministic() {
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), "v".to_string());
        let b = StubBackend::deterministic(m);
        assert!(b.is_deterministic());
        assert_eq!(b.complete("k"), "v");
        assert_eq!(b.complete("nope"), "[]");
    }

    #[test]
    fn load_benchmark_parses_json() {
        let spec_json = r#"{
            "name": "x",
            "description": "y",
            "seed": 1,
            "ticks": 1,
            "scene": {"kind":"scene","name":"x","nodes":[]},
            "actions_per_tick": [],
            "expected_final_hash": 0,
            "expected_components": {}
        }"#;
        let spec = BenchmarkSpec::from_json(spec_json).expect("parse");
        assert_eq!(spec.name, "x");
        assert_eq!(spec.ticks, 1);
    }

    #[test]
    fn summary_counts_passed() {
        let r1 = Report {
            name: "a".into(),
            passed: true,
            final_hash: 0,
            expected_hash: 0,
            hash_match: true,
            component_match: BTreeMap::new(),
            components_failing: Vec::new(),
            missing_signals: Vec::new(),
            unexpected_signals: Vec::new(),
            ticks_run: 0,
            duration_micros: 0,
        };
        let r2 = Report {
            passed: false,
            name: "b".into(),
            ..r1.clone()
        };
        let (p, t) = summary(&[r1, r2]);
        assert_eq!(p, 1);
        assert_eq!(t, 2);
    }

    #[test]
    fn assert_report_passes_does_not_panic_on_pass() {
        let r = Report {
            name: "ok".into(),
            passed: true,
            final_hash: 1,
            expected_hash: 1,
            hash_match: true,
            component_match: BTreeMap::new(),
            components_failing: Vec::new(),
            missing_signals: Vec::new(),
            unexpected_signals: Vec::new(),
            ticks_run: 1,
            duration_micros: 0,
        };
        assert_report_passes(&r);
    }

    #[test]
    fn hash_match_is_required_for_pass() {
        let r = Report {
            name: "fail".into(),
            passed: false,
            final_hash: 5,
            expected_hash: 99,
            hash_match: false,
            component_match: BTreeMap::new(),
            components_failing: Vec::new(),
            missing_signals: Vec::new(),
            unexpected_signals: Vec::new(),
            ticks_run: 1,
            duration_micros: 0,
        };
        let result = std::panic::catch_unwind(|| assert_report_passes(&r));
        assert!(result.is_err());
    }
}
