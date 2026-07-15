use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::behavior::{Behavior, Target};
use crate::scene::{Node, NodeRegistry, Scene};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LintSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintWarning {
    pub code: LintCode,
    pub severity: LintSeverity,
    pub node: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LintCode {
    SignalWithNoSubscribers,
    StateUnreachable,
    UnusedComponent,
    UndefinedNodeReference,
    ActionReferencesMissing,
    ResourcePathUnresolved,
}

pub fn lint(scene: &Scene, registry: &NodeRegistry) -> Vec<LintWarning> {
    let mut warnings = Vec::new();

    let known_ids: BTreeSet<&str> = scene.nodes.iter().map(|n| n.id.as_str()).collect();

    lint_signals_with_no_subscribers(scene, &mut warnings);
    lint_unreachable_states(scene, &mut warnings);
    lint_undefined_node_references(scene, &known_ids, &mut warnings);
    lint_action_references(scene, registry, &known_ids, &mut warnings);
    lint_unused_components(scene, registry, &mut warnings);

    warnings.sort_by(|a, b| {
        a.code
            .as_str()
            .cmp(b.code.as_str())
            .then_with(|| a.node.cmp(&b.node))
    });
    warnings
}

fn lint_signals_with_no_subscribers(scene: &Scene, out: &mut Vec<LintWarning>) {
    let mut emitted: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut listened: BTreeSet<String> = BTreeSet::new();

    for node in &scene.nodes {
        for behavior in &node.behaviors {
            for action_seq in actions_in_behavior(behavior) {
                collect_emitted(action_seq, &node.id, &mut emitted);
            }
            if let Behavior::OnSignal { signal, .. } = behavior {
                listened.insert(signal.clone());
            }
        }
    }

    for (signal, emitters) in emitted {
        if !listened.contains(&signal) {
            out.push(LintWarning {
                code: LintCode::SignalWithNoSubscribers,
                severity: LintSeverity::Warning,
                node: Some(emitters[0].clone()),
                message: format!("signal \"{signal}\" is emitted but has no on_signal subscribers"),
                suggestion: Some(format!(
                    "Add an on_signal handler for \"{signal}\" or remove the emit"
                )),
            });
        }
    }
}

fn lint_unreachable_states(scene: &Scene, out: &mut Vec<LintWarning>) {
    for node in &scene.nodes {
        for behavior in &node.behaviors {
            let Behavior::StateMachine { initial, states } = behavior else {
                continue;
            };
            let mut reachable: BTreeSet<String> = BTreeSet::new();
            reachable.insert(initial.clone());

            let mut frontier = vec![initial.clone()];
            while let Some(current) = frontier.pop() {
                if let Some(state) = states.get(&current) {
                    for tr in &state.transitions {
                        if reachable.insert(tr.to.clone()) {
                            frontier.push(tr.to.clone());
                        }
                    }
                }
            }

            let mut declared: BTreeSet<&str> = states.keys().map(String::as_str).collect();
            for name in &reachable {
                declared.remove(name.as_str());
            }
            for unreachable in declared {
                out.push(LintWarning {
                    code: LintCode::StateUnreachable,
                    severity: LintSeverity::Warning,
                    node: Some(node.id.clone()),
                    message: format!(
                        "state \"{unreachable}\" in node \"{}\" is never reached from initial \"{initial}\"",
                        node.id
                    ),
                    suggestion: Some(format!(
                        "Add a transition to \"{unreachable}\" or remove it"
                    )),
                });
            }
        }
    }
}

fn lint_undefined_node_references(
    scene: &Scene,
    known_ids: &BTreeSet<&str>,
    out: &mut Vec<LintWarning>,
) {
    for node in &scene.nodes {
        if let Some(parent) = &node.parent {
            if !known_ids.contains(parent.as_str()) {
                out.push(LintWarning {
                    code: LintCode::UndefinedNodeReference,
                    severity: LintSeverity::Error,
                    node: Some(node.id.clone()),
                    message: format!(
                        "node \"{}\" has parent \"{}\" but no node with that id exists",
                        node.id, parent
                    ),
                    suggestion: Some(format!(
                        "Define a node with id \"{parent}\" or remove the parent reference"
                    )),
                });
            }
        }
        for behavior in &node.behaviors {
            for action_seq in actions_in_behavior(behavior) {
                check_targets_in_actions(action_seq, &node.id, known_ids, out);
            }
        }
    }
}

fn lint_action_references(
    scene: &Scene,
    _registry: &NodeRegistry,
    known_ids: &BTreeSet<&str>,
    out: &mut Vec<LintWarning>,
) {
    for node in &scene.nodes {
        for behavior in &node.behaviors {
            for action_seq in actions_in_behavior(behavior) {
                check_action_system_names(action_seq, &node.id, out);
                check_action_signal_names(action_seq, &node.id, out);
            }
        }
        let _ = scene;
        let _ = known_ids;
    }
}

fn lint_unused_components(scene: &Scene, registry: &NodeRegistry, out: &mut Vec<LintWarning>) {
    for node in &scene.nodes {
        let Some(def) = registry.get(&node.type_name) else {
            continue;
        };
        let used = collect_used_components(node);
        for spec in def.component_specs() {
            if node.components.contains_key(&spec.name) && !used.contains(spec.name.as_str()) {
                out.push(LintWarning {
                    code: LintCode::UnusedComponent,
                    severity: LintSeverity::Warning,
                    node: Some(node.id.clone()),
                    message: format!(
                        "component \"{}\" on node \"{}\" is declared but never read by any action",
                        spec.name, node.id
                    ),
                    suggestion: Some(format!(
                        "Remove \"{}\" or reference it from an action",
                        spec.name
                    )),
                });
            }
        }
    }
}

fn collect_used_components(node: &Node) -> BTreeSet<String> {
    let mut used = BTreeSet::new();
    for behavior in &node.behaviors {
        for action_seq in actions_in_behavior(behavior) {
            for action in action_seq {
                collect_refs(action, &mut used);
            }
        }
        if let Behavior::StateMachine { states, .. } = behavior {
            for state in states.values() {
                for tr in &state.transitions {
                    collect_refs_from_expr(&tr.when, &mut used);
                }
            }
        }
    }
    used
}

fn collect_refs(action: &crate::behavior::Action, out: &mut BTreeSet<String>) {
    use crate::behavior::Action::*;
    match action {
        SetState { key, target, .. } | Move { key, target, .. } | Animate { key, target, .. } => {
            if matches!(target, Target::This) {
                out.insert(key.clone());
            }
        }
        Emit { .. } | Destroy { .. } | Log { .. } | CallSystem { .. } => {}
        If { cond, then, else_ } => {
            collect_refs_from_expr(cond, out);
            for a in then {
                collect_refs(a, out);
            }
            for a in else_ {
                collect_refs(a, out);
            }
        }
        Spawn { .. } => {}
    }
}

fn collect_refs_from_expr(expr: &crate::behavior::Expression, out: &mut BTreeSet<String>) {
    use crate::behavior::Expression::*;
    match expr {
        Ref { r#ref } => {
            if let Some((target, key)) = r#ref.split_once('.') {
                if target == "self" {
                    if let Some(k) = key.split('.').next() {
                        out.insert(k.to_string());
                    }
                }
            }
        }
        Eq { eq } => {
            for inner in eq {
                collect_refs_from_expr(inner, out);
            }
        }
        Neq { neq } => {
            for inner in neq {
                collect_refs_from_expr(inner, out);
            }
        }
        Lt { lt } => {
            for inner in lt {
                collect_refs_from_expr(inner, out);
            }
        }
        Gt { gt } => {
            for inner in gt {
                collect_refs_from_expr(inner, out);
            }
        }
        Add { add } => {
            for inner in add {
                collect_refs_from_expr(inner, out);
            }
        }
        Sub { sub } => {
            for inner in sub {
                collect_refs_from_expr(inner, out);
            }
        }
        Literal(_) | Bare(_) => {}
    }
}

fn actions_in_behavior(b: &Behavior) -> Vec<&Vec<crate::behavior::Action>> {
    match b {
        Behavior::StateMachine { states, .. } => {
            let mut out = Vec::new();
            for state in states.values() {
                out.push(&state.on_enter);
                out.push(&state.on_tick);
            }
            out
        }
        Behavior::OnTick { actions } => vec![actions],
        Behavior::OnSignal { actions, .. } => vec![actions],
    }
}

fn collect_emitted(
    actions: &[crate::behavior::Action],
    owner: &str,
    out: &mut BTreeMap<String, Vec<String>>,
) {
    use crate::behavior::Action::*;
    for action in actions {
        match action {
            Emit { signal, .. } => {
                out.entry(signal.clone())
                    .or_default()
                    .push(owner.to_string());
            }
            If { then, else_, .. } => {
                collect_emitted(then, owner, out);
                collect_emitted(else_, owner, out);
            }
            _ => {}
        }
    }
}

fn check_targets_in_actions(
    actions: &[crate::behavior::Action],
    owner: &str,
    known_ids: &BTreeSet<&str>,
    out: &mut Vec<LintWarning>,
) {
    use crate::behavior::Action::*;
    for action in actions {
        match action {
            SetState { target, .. }
            | Move { target, .. }
            | Animate { target, .. }
            | Destroy { target } => {
                if let Target::Node { id } = target {
                    if !known_ids.contains(id.as_str()) {
                        out.push(LintWarning {
                            code: LintCode::ActionReferencesMissing,
                            severity: LintSeverity::Error,
                            node: Some(owner.to_string()),
                            message: format!(
                                "action references node \"{}\" but no node with that id exists",
                                id
                            ),
                            suggestion: Some(format!(
                                "Define a node with id \"{}\" or change the target",
                                id
                            )),
                        });
                    }
                }
            }
            Spawn { parent, .. } => {
                if let Target::Node { id } = parent {
                    if !known_ids.contains(id.as_str()) {
                        out.push(LintWarning {
                            code: LintCode::ActionReferencesMissing,
                            severity: LintSeverity::Error,
                            node: Some(owner.to_string()),
                            message: format!(
                                "spawn parent references node \"{}\" but no node with that id exists",
                                id
                            ),
                            suggestion: Some(format!(
                                "Define a node with id \"{}\" or change the parent",
                                id
                            )),
                        });
                    }
                }
            }
            If { then, else_, .. } => {
                check_targets_in_actions(then, owner, known_ids, out);
                check_targets_in_actions(else_, owner, known_ids, out);
            }
            Emit { .. } | Log { .. } | CallSystem { .. } => {}
        }
    }
}

fn check_action_system_names(
    actions: &[crate::behavior::Action],
    owner: &str,
    out: &mut Vec<LintWarning>,
) {
    use crate::behavior::Action::*;
    for action in actions {
        match action {
            CallSystem { system, .. } => {
                out.push(LintWarning {
                    code: LintCode::ActionReferencesMissing,
                    severity: LintSeverity::Warning,
                    node: Some(owner.to_string()),
                    message: format!(
                        "call_system references system \"{system}\" — registry lookup not yet implemented for lint"
                    ),
                    suggestion: Some(format!(
                        "Verify \"{system}\" is registered via craft_system!"
                    )),
                });
            }
            If { then, else_, .. } => {
                check_action_system_names(then, owner, out);
                check_action_system_names(else_, owner, out);
            }
            _ => {}
        }
    }
}

fn check_action_signal_names(
    actions: &[crate::behavior::Action],
    owner: &str,
    out: &mut Vec<LintWarning>,
) {
    use crate::behavior::Action::*;
    for action in actions {
        match action {
            Emit { signal, .. } => {
                if signal.is_empty() {
                    out.push(LintWarning {
                        code: LintCode::ActionReferencesMissing,
                        severity: LintSeverity::Error,
                        node: Some(owner.to_string()),
                        message: "emit action has an empty signal name".to_string(),
                        suggestion: Some("Provide a non-empty signal name".to_string()),
                    });
                }
            }
            If { then, else_, .. } => {
                check_action_signal_names(then, owner, out);
                check_action_signal_names(else_, owner, out);
            }
            _ => {}
        }
    }
}

impl LintCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SignalWithNoSubscribers => "signal_with_no_subscribers",
            Self::StateUnreachable => "state_unreachable",
            Self::UnusedComponent => "unused_component",
            Self::UndefinedNodeReference => "undefined_node_reference",
            Self::ActionReferencesMissing => "action_references_missing",
            Self::ResourcePathUnresolved => "resource_path_unresolved",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateDef;
    use crate::scene::{Component, ComponentType, ComponentValue};
    use serde_json::json;

    struct TestNode;
    impl Default for TestNode {
        fn default() -> Self {
            TestNode
        }
    }
    impl crate::scene::NodeDef for TestNode {
        fn type_name(&self) -> &'static str {
            "TestNode"
        }
        fn component_specs(&self) -> Vec<crate::scene::ComponentSpec> {
            vec![
                crate::scene::ComponentSpec::new("hp", ComponentType::Int, json!(100)),
                crate::scene::ComponentSpec::new(
                    "position",
                    ComponentType::Vec2,
                    json!([0.0, 0.0]),
                ),
            ]
        }
    }

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        r.register::<TestNode>();
        r
    }

    fn node_with_behaviors(id: &str, behaviors: Vec<Behavior>) -> Node {
        Node {
            id: id.to_string(),
            type_name: "TestNode".to_string(),
            parent: None,
            components: BTreeMap::from([
                (
                    "hp".to_string(),
                    Component {
                        value: ComponentValue::Int(100),
                        kind: Default::default(),
                    },
                ),
                (
                    "position".to_string(),
                    Component {
                        value: ComponentValue::Vec2([0.0, 0.0]),
                        kind: Default::default(),
                    },
                ),
            ]),
            behaviors,
            active_state: None,
        }
    }

    #[test]
    fn detects_signal_with_no_subscribers() {
        let node = node_with_behaviors(
            "a",
            vec![Behavior::OnTick {
                actions: vec![crate::behavior::Action::Emit {
                    signal: "ghost".to_string(),
                    args: BTreeMap::new(),
                }],
            }],
        );
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.code == LintCode::SignalWithNoSubscribers),
            "expected signal_with_no_subscribers, got: {warnings:?}"
        );
    }

    #[test]
    fn detects_unreachable_state() {
        let node = node_with_behaviors(
            "a",
            vec![Behavior::StateMachine {
                initial: "idle".to_string(),
                states: BTreeMap::from([
                    (
                        "idle".to_string(),
                        StateDef {
                            transitions: vec![],
                            ..Default::default()
                        },
                    ),
                    (
                        "dead".to_string(),
                        StateDef {
                            transitions: vec![],
                            ..Default::default()
                        },
                    ),
                ]),
            }],
        );
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.code == LintCode::StateUnreachable),
            "expected state_unreachable"
        );
    }

    #[test]
    fn detects_undefined_parent() {
        let mut node = node_with_behaviors("a", vec![]);
        node.parent = Some("missing".to_string());
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(warnings.iter().any(
            |w| w.code == LintCode::UndefinedNodeReference && w.severity == LintSeverity::Error
        ),);
    }

    #[test]
    fn detects_undefined_action_target() {
        let node = node_with_behaviors(
            "a",
            vec![Behavior::OnTick {
                actions: vec![crate::behavior::Action::SetState {
                    target: Target::Node {
                        id: "missing".to_string(),
                    },
                    key: "hp".to_string(),
                    value: json!(1),
                }],
            }],
        );
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(
            warnings
                .iter()
                .any(|w| w.code == LintCode::ActionReferencesMissing
                    && w.severity == LintSeverity::Error),
        );
    }

    #[test]
    fn detects_unused_component() {
        let node = node_with_behaviors("a", vec![]);
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(warnings.iter().any(|w| w.code == LintCode::UnusedComponent),);
    }

    #[test]
    fn detects_missing_resource_uri_is_stubbed() {
        let _ = json!({});
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        let _ = warnings
            .iter()
            .filter(|w| w.code == LintCode::ResourcePathUnresolved)
            .count();
    }

    #[test]
    fn clean_scene_has_no_warnings() {
        let node = node_with_behaviors(
            "a",
            vec![Behavior::OnSignal {
                signal: "hit".to_string(),
                actions: vec![
                    crate::behavior::Action::SetState {
                        target: Target::This,
                        key: "hp".to_string(),
                        value: json!(50),
                    },
                    crate::behavior::Action::SetState {
                        target: Target::This,
                        key: "position".to_string(),
                        value: json!([1.0, 2.0]),
                    },
                ],
            }],
        );
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        let real = warnings
            .iter()
            .filter(|w| w.code != LintCode::ResourcePathUnresolved)
            .collect::<Vec<_>>();
        assert!(real.is_empty(), "expected no warnings, got: {real:?}");
    }

    #[test]
    fn reachable_transitions_are_not_flagged() {
        let node = node_with_behaviors(
            "a",
            vec![Behavior::StateMachine {
                initial: "a".to_string(),
                states: BTreeMap::from([
                    (
                        "a".to_string(),
                        StateDef {
                            transitions: vec![crate::behavior::Transition {
                                to: "b".to_string(),
                                when: crate::behavior::Expression::Literal(json!(true)),
                            }],
                            ..Default::default()
                        },
                    ),
                    (
                        "b".to_string(),
                        StateDef {
                            transitions: vec![],
                            ..Default::default()
                        },
                    ),
                ]),
            }],
        );
        let scene = Scene {
            kind: "scene".to_string(),
            name: "s".to_string(),
            nodes: vec![node],
            spawn_counter: 0,
        };
        let warnings = lint(&scene, &registry());
        assert!(
            !warnings
                .iter()
                .any(|w| w.code == LintCode::StateUnreachable),
            "a and b are both reachable"
        );
    }
}
