use crate::behavior::{Action, ActionCommand, Behavior};
use crate::error::EngineResult;
use crate::evaluator::{Animation, SceneState, Trigger, apply_commands, evaluate_behaviors};
use crate::hook::EngineHook;
use crate::lint::{LintWarning, lint};
use crate::render::{ComponentView, NullRenderer, Render};
use crate::resource::ResourceRegistry;
use crate::scene::{Node, NodeRegistry, Scene};
use crate::signal::SignalBus;
use crate::system::{SystemContext, SystemInfo, SystemPhase, SystemRegistry};

#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    pub seed: u64,
    pub tick_hz: u32,
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub direction: [f64; 2],
    pub action: bool,
}

pub struct Engine {
    pub bus: SignalBus,
    pub resources: ResourceRegistry,
    pub nodes: NodeRegistry,
    pub systems: SystemRegistry,
    pub rng_seed: u64,
    pub tick_hz: u32,
    pub tick: u64,
    pub scene: Option<Scene>,
    pub logs: Vec<crate::evaluator::LogEntry>,
    pub animations: SceneState,
    /// Signals queued for dispatch in the next tick. Each entry carries
    /// the signal name and the JSON payload that was emitted, so that
    /// external hooks (e.g. Lua on_signal) receive the same args that
    /// JSON handlers see.
    pub pending_signals: Vec<(String, serde_json::Value)>,
    pub renderer: Box<dyn Render>,
    pub render_each_tick: bool,
    /// Optional host-side hook (e.g. craft-lua) that participates in the
    /// tick loop and signal dispatch. See `EngineHook` for the contract.
    pub hook: Option<Box<dyn EngineHook>>,
    pub input: InputState,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("bus", &self.bus)
            .field("resources", &"ResourceRegistry")
            .field("nodes", &"NodeRegistry")
            .field("systems", &self.systems.list())
            .field("rng_seed", &self.rng_seed)
            .field("tick_hz", &self.tick_hz)
            .field("tick", &self.tick)
            .field("renderer_viewport", &self.renderer.viewport())
            .field("render_each_tick", &self.render_each_tick)
            .field("scene_loaded", &self.scene.is_some())
            .finish()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::with_config(EngineConfig::default())
    }

    pub fn with_config(config: EngineConfig) -> Self {
        let mut systems = SystemRegistry::new();
        systems.instantiate_all();
        let mut nodes = NodeRegistry::new();
        nodes.instantiate_all();
        Self {
            bus: SignalBus::new(),
            resources: ResourceRegistry::new(),
            nodes,
            systems,
            rng_seed: config.seed,
            tick_hz: if config.tick_hz == 0 {
                60
            } else {
                config.tick_hz
            },
            tick: 0,
            scene: None,
            logs: Vec::new(),
            animations: SceneState::default(),
            pending_signals: Vec::new(),
            renderer: Box::new(NullRenderer::new()),
            render_each_tick: false,
            hook: None,
            input: InputState::default(),
        }
    }

    pub fn load_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }

    pub fn scene(&self) -> Option<&Scene> {
        self.scene.as_ref()
    }

    pub fn scene_mut(&mut self) -> Option<&mut Scene> {
        self.scene.as_mut()
    }

    pub fn signal_bus(&self) -> &SignalBus {
        &self.bus
    }

    pub fn signal_bus_mut(&mut self) -> &mut SignalBus {
        &mut self.bus
    }

    pub fn list_systems(&self) -> Vec<SystemInfo> {
        self.systems.list()
    }

    pub fn lint(&self) -> Vec<LintWarning> {
        match &self.scene {
            Some(scene) => lint(scene, &self.nodes),
            None => Vec::new(),
        }
    }

    pub fn node_registry_mut(&mut self) -> &mut NodeRegistry {
        &mut self.nodes
    }

    pub fn lint_scene(scene: &Scene, registry: &NodeRegistry) -> Vec<LintWarning> {
        lint(scene, registry)
    }

    pub fn set_input_direction(&mut self, dx: f64, dy: f64) {
        self.input.direction = [dx, dy];
    }

    pub fn set_input_action(&mut self, pressed: bool) {
        self.input.action = pressed;
    }

    fn populate_input_node(&mut self) {
        let Some(ref mut scene) = self.scene else {
            return;
        };
        let dir = self.input.direction;
        let action = self.input.action;
        if let Some(input_node) = scene.nodes.iter_mut().find(|n| n.id == "__input") {
            input_node.components.insert(
                "direction".to_string(),
                crate::scene::Component {
                    value: crate::scene::ComponentValue::Vec2(dir),
                    kind: Default::default(),
                },
            );
            input_node.components.insert(
                "action".to_string(),
                crate::scene::Component {
                    value: crate::scene::ComponentValue::Bool(action),
                    kind: Default::default(),
                },
            );
        }
    }

    pub fn tick(&mut self) {
        self.populate_input_node();
        self.bus.deliver_pending();
        for phase in SystemPhase::ALL {
            let mut ctx = SystemContext {
                bus: &mut self.bus,
                resources: &self.resources,
                scene: self.scene.as_mut(),
                pending_signals: &mut self.pending_signals,
                tick: self.tick,
            };
            self.systems.run_phase(phase, &mut ctx);
        }
        let pending = std::mem::take(&mut self.pending_signals);
        for (sig, args) in pending {
            self.dispatch_signal(&sig, &args);
            self.bus.emit_by_name(&sig, &args);
        }
        if let Some(mut hook) = self.hook.take() {
            hook.before_behaviors(self);
            self.hook = Some(hook);
        }
        self.tick_behaviors();
        if self.render_each_tick {
            self.render_now();
        }
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn set_hook(&mut self, hook: Option<Box<dyn EngineHook>>) {
        self.hook = hook;
    }

    pub fn render_now(&mut self) {
        let views: Vec<ComponentView> = match &self.scene {
            Some(scene) => scene.nodes.iter().map(ComponentView::from_node).collect(),
            None => Vec::new(),
        };
        self.renderer.render(&views, self.tick);
    }

    pub fn set_renderer(&mut self, renderer: Box<dyn Render>) {
        self.renderer = renderer;
    }

    pub fn enable_rendering(&mut self, on: bool) {
        self.render_each_tick = on;
        if on {
            self.render_now();
        }
    }

    pub fn renderer(&self) -> &dyn Render {
        &*self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut dyn Render {
        &mut *self.renderer
    }

    fn tick_behaviors(&mut self) {
        if self.scene.is_none() {
            return;
        }
        let mut emitted: Vec<(String, serde_json::Value)> = Vec::new();
        let all_cmds: Vec<ActionCommand>;
        {
            let scene = self.scene.as_mut().unwrap();
            let mut cmds = Vec::new();
            let node_snapshot: Vec<Node> = scene.nodes.clone();
            for node in &node_snapshot {
                cmds.extend(evaluate_behaviors(
                    scene,
                    &self.nodes,
                    node,
                    Trigger::Tick,
                    self.tick,
                ));
            }
            for cmd in &cmds {
                if let crate::behavior::ActionCommand::Emit { signal, args } = cmd {
                    let args_value = if args.is_empty() {
                        serde_json::Value::Null
                    } else {
                        serde_json::to_value(args).unwrap_or(serde_json::Value::Null)
                    };
                    emitted.push((signal.clone(), args_value));
                }
            }
            all_cmds = cmds;
        }
        let logs = crate::evaluator::apply_commands_with_animations(
            self.scene.as_mut().unwrap(),
            &self.nodes,
            all_cmds,
            &mut Some(&mut self.animations),
        );
        self.logs.extend(logs);
        self.pending_signals.extend(emitted);
        tick_animations(&mut self.animations, self.scene.as_mut().unwrap());
    }

    pub fn dispatch_signal(&mut self, signal_name: &str, args: &serde_json::Value) {
        if let Some(scene) = &mut self.scene {
            let mut all_cmds = Vec::new();
            let snapshot: Vec<Node> = scene.nodes.clone();
            let target_ids: Vec<&str> = extract_target_ids(args);
            let targeted = !target_ids.is_empty();
            for node in &snapshot {
                if targeted && !target_ids.contains(&node.id.as_str()) {
                    continue;
                }
                let mut has_handler = false;
                for b in &node.behaviors {
                    if let Behavior::OnSignal { signal, .. } = b {
                        if signal == signal_name {
                            has_handler = true;
                            break;
                        }
                    }
                }
                if !has_handler {
                    continue;
                }
                let cmds = evaluate_behaviors(
                    scene,
                    &self.nodes,
                    node,
                    Trigger::Signal(signal_name.to_string()),
                    self.tick,
                );
                all_cmds.extend(cmds);
            }
            let logs = apply_commands(self.scene.as_mut().unwrap(), &self.nodes, all_cmds);
            self.logs.extend(logs);
        }
        if let Some(mut hook) = self.hook.take() {
            hook.on_signal(self, signal_name, args);
            self.hook = Some(hook);
        }
    }

    pub fn emit(&mut self, signal_id: crate::SignalId, payload: serde_json::Value) {
        self.bus.emit(signal_id, payload);
    }

    /// Queue a signal for dispatch on the next tick. Equivalent to a JSON
    /// behavior emitting the signal — the signal will be delivered to JSON
    /// `on_signal` handlers and to the Lua hook in the next `tick()` call.
    pub fn queue_signal(&mut self, signal_name: impl Into<String>, args: serde_json::Value) {
        self.pending_signals.push((signal_name.into(), args));
    }

    pub fn apply_hot_reload(
        &mut self,
        new_scene: &Scene,
    ) -> EngineResult<crate::hot_reload::HotReloadResult> {
        let result = if let Some(scene) = &mut self.scene {
            crate::hot_reload::hot_reload_scene(scene, &self.nodes, new_scene)?
        } else {
            self.load_scene(new_scene.clone());
            return Ok(crate::hot_reload::HotReloadResult {
                diff: crate::hot_reload::SceneDiff::default(),
                affected_node_ids: Vec::new(),
                applied: false,
            });
        };
        if result.applied {
            let sig = self.bus.declare("hot_reload");
            self.bus.emit(
                sig,
                serde_json::json!({
                    "affected": result.affected_node_ids,
                    "diff_size": result.diff.component_changes.len()
                        + result.diff.added_nodes.len()
                        + result.diff.removed_node_ids.len()
                }),
            );
        }
        Ok(result)
    }

    pub fn state_hash(&self) -> u64 {
        match &self.scene {
            Some(scene) => crate::scene::hash_scene_state(scene),
            None => 0,
        }
    }

    pub fn last_signals(&self) -> Vec<String> {
        self.pending_signals
            .iter()
            .map(|(n, _)| n.clone())
            .collect()
    }

    pub fn take_last_signals(&mut self) -> Vec<(String, serde_json::Value)> {
        std::mem::take(&mut self.pending_signals)
    }

    pub fn clone_last_signals(&self) -> Vec<(String, serde_json::Value)> {
        self.pending_signals.clone()
    }

    pub fn animations_for(&self, node_id: &str) -> &[Animation] {
        self.animations
            .animations
            .get(node_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn _unused() -> Action {
        Action::Emit {
            signal: String::new(),
            args: Default::default(),
        }
    }
}

fn extract_target_ids(args: &serde_json::Value) -> Vec<&str> {
    let mut ids = Vec::new();
    if let Some(obj) = args.as_object() {
        if let Some(a) = obj.get("a").and_then(|v| v.as_str()) {
            ids.push(a);
        }
        if let Some(b) = obj.get("b").and_then(|v| v.as_str()) {
            ids.push(b);
        }
    }
    ids
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

fn tick_animations(state: &mut SceneState, scene: &mut Scene) {
    let mut to_remove: Vec<(String, usize)> = Vec::new();
    for (node_id, anims) in state.animations.iter_mut() {
        for (idx, anim) in anims.iter_mut().enumerate() {
            if anim.remaining == 0 {
                to_remove.push((node_id.clone(), idx));
                continue;
            }
            let step = interpolate(&anim.from, &anim.to, anim.remaining);
            if let Some(node) = scene.nodes.iter_mut().find(|n| &n.id == node_id) {
                node.components.insert(
                    anim.key.clone(),
                    crate::scene::Component {
                        value: step,
                        kind: Default::default(),
                    },
                );
            }
            anim.remaining -= 1;
            if anim.remaining == 0 {
                to_remove.push((node_id.clone(), idx));
            }
        }
    }
    for (node_id, idx) in to_remove {
        if let Some(anims) = state.animations.get_mut(&node_id) {
            if idx < anims.len() {
                anims.remove(idx);
            }
        }
    }
}

fn interpolate(
    _from: &crate::scene::ComponentValue,
    to: &crate::scene::ComponentValue,
    remaining: u32,
) -> crate::scene::ComponentValue {
    let _ = remaining;
    to.clone()
}

pub fn evaluate_dry_run(
    scene: &Scene,
    registry: &NodeRegistry,
    node_id: &str,
    actions: &[Action],
) -> crate::error::EngineResult<Vec<crate::evaluator::ComponentChange>> {
    crate::evaluator::evaluate_dry_run(scene, registry, node_id, actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineError;
    use crate::SignalId;
    use serde_json::json;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn new_engine_starts_at_tick_zero() {
        let engine = Engine::new();
        assert_eq!(engine.tick, 0);
        assert!(engine.scene.is_none());
    }

    #[test]
    fn with_config_records_seed_without_affecting_tick() {
        let engine = Engine::with_config(EngineConfig {
            seed: 42,
            tick_hz: 0,
        });
        assert_eq!(engine.rng_seed, 42);
        assert_eq!(engine.tick, 0, "seed must not be used as initial tick");
        assert_eq!(engine.tick_hz, 60, "tick_hz 0 falls back to 60 Hz");
    }

    #[test]
    fn with_config_preserves_tick_hz() {
        let engine = Engine::with_config(EngineConfig {
            seed: 0,
            tick_hz: 120,
        });
        assert_eq!(engine.tick_hz, 120);
    }

    #[test]
    fn load_scene_stores_scene() {
        let mut engine = Engine::new();
        let scene = Scene {
            kind: "scene".to_string(),
            name: "test".to_string(),
            nodes: vec![],
            spawn_counter: 0,
        };
        engine.load_scene(scene);
        assert!(engine.scene.is_some());
        let loaded = engine.scene.as_ref().expect("loaded");
        assert_eq!(loaded.name, "test");
    }

    #[test]
    fn engine_tick_delivers_pre_tick_signals_at_start() {
        let mut engine = Engine::new();
        let hit = engine.bus.declare("hit");
        let counter = Rc::new(RefCell::new(0u32));
        let counter_inner = counter.clone();
        engine
            .bus
            .subscribe(hit, move |_| *counter_inner.borrow_mut() += 1);

        engine.emit(hit, json!({"damage": 5}));
        assert_eq!(
            *counter.borrow(),
            0,
            "pre-tick signals must not fire before tick()"
        );
        assert_eq!(engine.bus.pending_count(), 1);

        engine.tick();
        assert_eq!(
            *counter.borrow(),
            1,
            "ADR 0003: signals queued before tick() are delivered at start of tick"
        );
        assert_eq!(engine.bus.pending_count(), 0);
    }

    #[test]
    fn engine_tick_does_not_deliver_same_tick_signals() {
        let mut engine = Engine::new();
        let hit: SignalId = engine.bus.declare("hit");
        let counter = Rc::new(RefCell::new(0u32));
        let counter_inner = counter.clone();
        engine
            .bus
            .subscribe(hit, move |_| *counter_inner.borrow_mut() += 1);

        engine.emit(hit, json!(null));
        engine.tick();
        assert_eq!(
            *counter.borrow(),
            1,
            "the pre-tick signal delivered at start of tick 1"
        );

        engine.emit(hit, json!(null));
        engine.tick();
        assert_eq!(
            *counter.borrow(),
            2,
            "the signal emitted during tick 1 is delivered at start of tick 2"
        );
    }

    #[test]
    fn tick_increments_each_call() {
        let mut engine = Engine::new();
        for expected in 1..=3u64 {
            engine.tick();
            assert_eq!(engine.tick, expected);
        }
    }

    #[test]
    fn empty_scene_lint_has_no_warnings() {
        let engine = Engine::new();
        let warnings = engine.lint();
        assert!(warnings.is_empty());
    }

    #[test]
    fn dry_run_returns_err_for_unknown_node() {
        let scene = Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes: vec![],
            spawn_counter: 0,
        };
        let reg = NodeRegistry::new();
        let err = evaluate_dry_run(&scene, &reg, "missing", &[]).expect_err("must fail");
        assert!(matches!(err, EngineError::Internal(_)));
    }

    #[test]
    fn targeted_signal_only_fires_on_named_nodes() {
        use crate::behavior::Behavior;
        use crate::scene::{Component, ComponentKind};

        let mut engine = Engine::new();
        let node_a = Node {
            id: "a".to_string(),
            type_name: "Test".to_string(),
            parent: None,
            components: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "hp".to_string(),
                    Component {
                        value: crate::scene::ComponentValue::Int(10),
                        kind: ComponentKind::Regular,
                    },
                );
                m
            },
            behaviors: vec![Behavior::OnSignal {
                signal: "hit".to_string(),
                actions: vec![crate::behavior::Action::Move {
                    target: crate::behavior::Target::This,
                    key: "hp".to_string(),
                    by: serde_json::json!(-1),
                }],
            }],
            active_state: None,
            lua_class: None,
            destroyed: false,
        };
        let node_b = Node {
            id: "b".to_string(),
            type_name: "Test".to_string(),
            parent: None,
            components: {
                let mut m = std::collections::BTreeMap::new();
                m.insert(
                    "hp".to_string(),
                    Component {
                        value: crate::scene::ComponentValue::Int(20),
                        kind: ComponentKind::Regular,
                    },
                );
                m
            },
            behaviors: vec![Behavior::OnSignal {
                signal: "hit".to_string(),
                actions: vec![crate::behavior::Action::Move {
                    target: crate::behavior::Target::This,
                    key: "hp".to_string(),
                    by: serde_json::json!(-1),
                }],
            }],
            active_state: None,
            lua_class: None,
            destroyed: false,
        };
        let scene = Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes: vec![node_a, node_b],
            spawn_counter: 0,
        };
        engine.load_scene(scene);

        engine.queue_signal(
            "hit",
            serde_json::json!({"a": "a", "b": "nonexistent"}),
        );
        engine.tick();
        engine.tick();

        let hp_a = engine
            .scene()
            .unwrap()
            .find_node("a")
            .and_then(|n| n.components.get("hp"))
            .map(|c| match &c.value {
                crate::scene::ComponentValue::Int(i) => *i,
                _ => 0,
            })
            .unwrap();
        let hp_b = engine
            .scene()
            .unwrap()
            .find_node("b")
            .and_then(|n| n.components.get("hp"))
            .map(|c| match &c.value {
                crate::scene::ComponentValue::Int(i) => *i,
                _ => 0,
            })
            .unwrap();

        assert_eq!(hp_a, 9, "node a was targeted, should lose 1 HP");
        assert_eq!(hp_b, 20, "node b was NOT targeted, should keep full HP");
    }

    #[test]
    fn populate_input_node_updates_existing() {
        let mut engine = Engine::new();
        let input_node = Node {
            id: "__input".to_string(),
            type_name: "Input".to_string(),
            parent: None,
            components: std::collections::BTreeMap::new(),
            behaviors: vec![],
            active_state: None,
            lua_class: None,
            destroyed: false,
        };
        let scene = Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes: vec![input_node],
            spawn_counter: 0,
        };
        engine.load_scene(scene);
        engine.set_input_direction(1.0, -1.0);
        engine.set_input_action(true);
        engine.tick();

        let input_node = engine
            .scene()
            .unwrap()
            .find_node("__input")
            .expect("Input node created");
        let dir = input_node.components.get("direction").unwrap();
        assert_eq!(dir.value, crate::scene::ComponentValue::Vec2([1.0, -1.0]));
        let action = input_node.components.get("action").unwrap();
        assert_eq!(action.value, crate::scene::ComponentValue::Bool(true));

        engine.set_input_direction(0.0, 1.0);
        engine.tick();
        let dir2 = engine
            .scene()
            .unwrap()
            .find_node("__input")
            .unwrap()
            .components
            .get("direction")
            .unwrap();
        assert_eq!(dir2.value, crate::scene::ComponentValue::Vec2([0.0, 1.0]));
    }
}
