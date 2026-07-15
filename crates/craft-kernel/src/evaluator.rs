use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::behavior::{Action, ActionCommand, Behavior, Expression, LogLevel, Target};
use crate::error::EngineResult;
use crate::scene::{Component, ComponentValue, Node, NodeRegistry, Scene};

#[derive(Debug, Clone)]
pub enum EvalError {
    UnknownNode(String),
    UnknownComponent { node: String, key: String },
    BadRef(String),
    BadNumeric(String),
    WrongType(String),
}

#[derive(Debug, Clone)]
pub struct Animation {
    pub key: String,
    pub from: ComponentValue,
    pub to: ComponentValue,
    pub remaining: u32,
}

#[derive(Debug, Default)]
pub struct SceneState {
    pub animations: BTreeMap<String, Vec<Animation>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Trigger {
    Tick,
    Signal(String),
}

pub fn evaluate_behaviors(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    trigger: Trigger,
    tick: u64,
) -> Vec<ActionCommand> {
    let mut pending_writes: std::collections::BTreeMap<(String, String), ComponentValue> =
        std::collections::BTreeMap::new();
    let mut cmds = Vec::new();
    evaluate_behaviors_inner(
        scene,
        registry,
        node,
        trigger,
        tick,
        &mut pending_writes,
        &mut cmds,
    );
    cmds
}

fn evaluate_behaviors_inner(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    trigger: Trigger,
    tick: u64,
    pending_writes: &mut std::collections::BTreeMap<(String, String), ComponentValue>,
    cmds: &mut Vec<ActionCommand>,
) {
    for behavior in &node.behaviors {
        match behavior {
            Behavior::StateMachine { initial, states } => {
                let active = node.active_state.clone().unwrap_or_else(|| initial.clone());
                if node.active_state.is_none() {
                    cmds.push(ActionCommand::SetState {
                        target: Target::This,
                        key: "_active_state".to_string(),
                        value: ComponentValue::String(active.clone()),
                    });
                }
                if let Some(state) = states.get(&active) {
                    if matches!(trigger, Trigger::Tick) {
                        let mut transitioned = false;
                        for tr in &state.transitions {
                            if let Ok(cond) = eval_expression(
                                scene,
                                registry,
                                node,
                                &tr.when,
                                pending_writes,
                                tick,
                            ) {
                                if truthy(&cond) {
                                    cmds.push(ActionCommand::Emit {
                                        signal: format!("state:exit:{active}"),
                                        args: BTreeMap::new(),
                                    });
                                    cmds.push(ActionCommand::SetState {
                                        target: Target::This,
                                        key: "_active_state".to_string(),
                                        value: ComponentValue::String(tr.to.clone()),
                                    });
                                    if let Some(next) = states.get(&tr.to) {
                                        cmds.push(ActionCommand::Emit {
                                            signal: format!("state:enter:{}", tr.to),
                                            args: BTreeMap::new(),
                                        });
                                        for action in &next.on_enter {
                                            evaluate_action(
                                                scene,
                                                registry,
                                                node,
                                                action,
                                                cmds,
                                                &mut *pending_writes,
                                                tick,
                                            );
                                        }
                                        for action in &next.on_tick {
                                            evaluate_action(
                                                scene,
                                                registry,
                                                node,
                                                action,
                                                cmds,
                                                &mut *pending_writes,
                                                tick,
                                            );
                                        }
                                    }
                                    transitioned = true;
                                    break;
                                }
                            }
                        }
                        if !transitioned {
                            for action in &state.on_tick {
                                evaluate_action(
                                    scene,
                                    registry,
                                    node,
                                    action,
                                    cmds,
                                    pending_writes,
                                    tick,
                                );
                            }
                        }
                    }
                }
            }
            Behavior::OnTick { actions } => {
                if matches!(trigger, Trigger::Tick) {
                    for action in actions {
                        evaluate_action(scene, registry, node, action, cmds, pending_writes, tick);
                    }
                }
            }
            Behavior::OnSignal { signal, actions } => {
                if let Trigger::Signal(sig) = &trigger {
                    if sig == signal {
                        for action in actions {
                            evaluate_action(
                                scene,
                                registry,
                                node,
                                action,
                                cmds,
                                pending_writes,
                                tick,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn evaluate_action(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    action: &Action,
    out: &mut Vec<ActionCommand>,
    pending_writes: &mut std::collections::BTreeMap<(String, String), ComponentValue>,
    tick: u64,
) {
    match action {
        Action::SetState { target, key, value } => {
            if !target_allowed_for_tick(node, target) {
                return;
            }
            if let Ok(v) = json_to_component_value(value.clone()) {
                let resolved = resolve_target_for_command(scene, node, target);
                if let Target::Node { id } = &resolved {
                    pending_writes.insert((id.clone(), key.clone()), v.clone());
                }
                out.push(ActionCommand::SetState {
                    target: resolved,
                    key: key.clone(),
                    value: v,
                });
            }
        }
        Action::Emit { signal, args } => {
            out.push(ActionCommand::Emit {
                signal: signal.clone(),
                args: args.clone(),
            });
        }
        Action::Destroy { target } => {
            let resolved = resolve_target_for_command(scene, node, target);
            out.push(ActionCommand::Destroy { target: resolved });
        }
        Action::Spawn {
            node_type,
            parent,
            components,
            behaviors,
        } => {
            let mut conv = BTreeMap::new();
            for (k, v) in components {
                if let Ok(v) = json_to_component_value(v.clone()) {
                    conv.insert(k.clone(), v);
                }
            }
            let resolved_parent = resolve_target_for_command(scene, node, parent);
            out.push(ActionCommand::Spawn {
                node_type: node_type.clone(),
                parent: resolved_parent,
                components: conv,
                behaviors: behaviors.clone(),
            });
        }
        Action::If { cond, then, else_ } => {
            let v = eval_expression(scene, registry, node, cond, pending_writes, tick);
            let branch = if v.as_ref().is_ok_and(truthy) {
                then
            } else {
                else_
            };
            for action in branch {
                evaluate_action(scene, registry, node, action, out, pending_writes, tick);
            }
        }
        Action::Move { target, key, by } => {
            if !target_allowed_for_tick(node, target) {
                return;
            }
            let by_value = json_to_component_value(by.clone()).ok();
            let Some(by_value) = by_value else { return };
            let Some(target_node) = resolve_target(scene, node, target) else {
                return;
            };
            let Some(current) = target_node.components.get(key).map(|c| c.value.clone()) else {
                return;
            };
            let Some(next) = numeric_add(&current, &by_value) else {
                return;
            };
            let resolved = resolve_target_for_command(scene, node, target);
            if let Target::Node { id } = &resolved {
                pending_writes.insert((id.clone(), key.clone()), next.clone());
            }
            out.push(ActionCommand::SetState {
                target: resolved,
                key: key.clone(),
                value: next,
            });
        }
        Action::Animate {
            target,
            key,
            to,
            duration,
        } => {
            if !target_allowed_for_tick(node, target) {
                return;
            }
            if let Ok(to) = json_to_component_value(to.clone()) {
                let resolved = resolve_target_for_command(scene, node, target);
                out.push(ActionCommand::StartAnimation {
                    target: resolved,
                    key: key.clone(),
                    to,
                    remaining: *duration,
                });
            }
        }
        Action::Log {
            level,
            message,
            fields,
        } => {
            out.push(ActionCommand::Log {
                level: *level,
                message: message.clone(),
                fields: fields.clone(),
            });
        }
        Action::CallSystem {
            system,
            args,
            result_in,
        } => {
            out.push(ActionCommand::CallSystem {
                system: system.clone(),
                args: args.clone(),
                result_target: result_in.clone(),
            });
        }
    }
}

fn target_allowed_for_tick(node: &Node, target: &Target) -> bool {
    match target {
        Target::This => true,
        Target::Children { .. } => true,
        Target::Node { id } => id == &node.id,
        Target::Parent => true,
    }
}

pub fn resolve_target_for_command(scene: &Scene, node: &Node, target: &Target) -> Target {
    match target {
        Target::This => Target::Node {
            id: node.id.clone(),
        },
        other => {
            if let Some(n) = resolve_target(scene, node, other) {
                return Target::Node { id: n.id.clone() };
            }
            other.clone()
        }
    }
}

pub fn resolve_target<'a>(scene: &'a Scene, node: &'a Node, target: &Target) -> Option<&'a Node> {
    match target {
        Target::This => Some(node),
        Target::Node { id } => scene.nodes.iter().find(|n| &n.id == id),
        Target::Parent => {
            let parent_id = node.parent.as_ref()?;
            scene.nodes.iter().find(|n| &n.id == parent_id)
        }
        Target::Children { filter } => {
            if let Some(child_id) = filter {
                scene
                    .nodes
                    .iter()
                    .find(|n| n.parent.as_ref() == Some(&node.id) && &n.id == child_id)
            } else {
                scene
                    .nodes
                    .iter()
                    .find(|n| n.parent.as_ref() == Some(&node.id))
            }
        }
    }
}

pub fn eval_expression(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    expr: &Expression,
    pending_writes: &std::collections::BTreeMap<(String, String), ComponentValue>,
    tick: u64,
) -> Result<Value, EvalError> {
    match expr {
        Expression::Literal(v) => Ok(v.clone()),
        Expression::Bare(s) => match s.as_str() {
            "self" => Ok(Value::String(node.id.clone())),
            "none" => Ok(Value::Null),
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(EvalError::BadRef(format!("reserved token expected: {s}"))),
        },
        Expression::Ref { r#ref } => eval_ref(scene, node, r#ref, pending_writes),
        Expression::Eq { eq } => {
            eval_binary_comparison(scene, registry, node, eq, pending_writes, tick, |a, b| {
                a == b
            })
        }
        Expression::Neq { neq } => Ok(Value::Bool(
            !eval_binary_comparison(scene, registry, node, neq, pending_writes, tick, |a, b| {
                a == b
            })?
            .as_bool()
            .unwrap_or(false),
        )),
        Expression::Lt { lt } => {
            let (a, b) = eval_binary_pair(scene, registry, node, lt, pending_writes, tick)?;
            Ok(Value::Bool(
                compare_ord(&a, &b)
                    .map(|o| o == std::cmp::Ordering::Less)
                    .unwrap_or(false),
            ))
        }
        Expression::Gt { gt } => {
            let (a, b) = eval_binary_pair(scene, registry, node, gt, pending_writes, tick)?;
            Ok(Value::Bool(
                compare_ord(&a, &b)
                    .map(|o| o == std::cmp::Ordering::Greater)
                    .unwrap_or(false),
            ))
        }
        Expression::Add { add } => {
            let (a, b) = eval_binary_pair(scene, registry, node, add, pending_writes, tick)?;
            match (a.as_i64(), b.as_i64()) {
                (Some(x), Some(y)) => Ok(json!(x + y)),
                _ => match (a.as_f64(), b.as_f64()) {
                    (Some(x), Some(y)) => Ok(json!(x + y)),
                    _ => Err(EvalError::BadNumeric(format!(
                        "add: not numeric ({a}, {b})"
                    ))),
                },
            }
        }
        Expression::Sub { sub } => {
            let (a, b) = eval_binary_pair(scene, registry, node, sub, pending_writes, tick)?;
            match (a.as_i64(), b.as_i64()) {
                (Some(x), Some(y)) => Ok(json!(x - y)),
                _ => match (a.as_f64(), b.as_f64()) {
                    (Some(x), Some(y)) => Ok(json!(x - y)),
                    _ => Err(EvalError::BadNumeric(format!(
                        "sub: not numeric ({a}, {b})"
                    ))),
                },
            }
        }
    }
}

fn eval_ref(
    scene: &Scene,
    node: &Node,
    path: &str,
    pending_writes: &std::collections::BTreeMap<(String, String), ComponentValue>,
) -> Result<Value, EvalError> {
    let (target_part, key_part) = match path.split_once('.') {
        Some((t, k)) => (t, Some(k)),
        None => (path, None),
    };
    let target_id = match target_part {
        "self" => node.id.clone(),
        "parent" => node
            .parent
            .clone()
            .ok_or_else(|| EvalError::UnknownNode("parent".to_string()))?,
        other => other.to_string(),
    };
    let Some(key) = key_part else {
        return Ok(Value::String(target_id));
    };
    if let Some(v) = pending_writes.get(&(target_id.clone(), key.to_string())) {
        return Ok(component_value_to_json(v));
    }
    let target_node = scene
        .nodes
        .iter()
        .find(|n| n.id == target_id)
        .ok_or_else(|| EvalError::UnknownNode(target_part.to_string()))?;
    let component = target_node
        .components
        .get(key)
        .ok_or_else(|| EvalError::UnknownComponent {
            node: target_node.id.clone(),
            key: key.to_string(),
        })?;
    Ok(component_value_to_json(&component.value))
}

fn normalize(v: Value) -> Value {
    if let Value::Object(o) = &v {
        if let Some(type_str) = o.get("type").and_then(|v| v.as_str()) {
            if let Some(inner) = o.get("value") {
                return match type_str {
                    "Int" | "Float" | "Bool" | "String" => inner.clone(),
                    _ => v,
                };
            }
        }
    }
    v
}

fn eval_binary_comparison(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    args: &[Expression],
    pending_writes: &std::collections::BTreeMap<(String, String), ComponentValue>,
    tick: u64,
    cmp: impl Fn(&Value, &Value) -> bool,
) -> Result<Value, EvalError> {
    if args.len() != 2 {
        return Err(EvalError::WrongType(format!(
            "binary op expects 2 args, got {}",
            args.len()
        )));
    }
    let a = normalize(eval_expression(
        scene,
        registry,
        node,
        &args[0],
        pending_writes,
        tick,
    )?);
    let b = normalize(eval_expression(
        scene,
        registry,
        node,
        &args[1],
        pending_writes,
        tick,
    )?);
    Ok(Value::Bool(cmp(&a, &b)))
}

fn eval_binary_pair(
    scene: &Scene,
    registry: &NodeRegistry,
    node: &Node,
    args: &[Expression],
    pending_writes: &std::collections::BTreeMap<(String, String), ComponentValue>,
    tick: u64,
) -> Result<(Value, Value), EvalError> {
    if args.len() != 2 {
        return Err(EvalError::WrongType(format!(
            "binary op expects 2 args, got {}",
            args.len()
        )));
    }
    let a = normalize(eval_expression(
        scene,
        registry,
        node,
        &args[0],
        pending_writes,
        tick,
    )?);
    let b = normalize(eval_expression(
        scene,
        registry,
        node,
        &args[1],
        pending_writes,
        tick,
    )?);
    Ok((a, b))
}

fn compare_ord(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let x = x.as_f64()?;
            let y = y.as_f64()?;
            x.partial_cmp(&y)
        }
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        _ => None,
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

pub fn json_to_component_value(v: Value) -> Result<ComponentValue, String> {
    match v {
        Value::Null => Ok(ComponentValue::Nil),
        Value::Bool(b) => Ok(ComponentValue::Bool(b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(ComponentValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                Ok(ComponentValue::Float(f))
            } else {
                Err(format!("unsupported number: {n}"))
            }
        }
        Value::String(s) => Ok(ComponentValue::String(s)),
        Value::Array(a) => {
            if a.len() != 2 {
                return Err(format!("expected [x, y] for vec2, got length {}", a.len()));
            }
            let x = a[0]
                .as_f64()
                .ok_or_else(|| format!("vec2[0] must be a number, got {:?}", a[0]))?;
            let y = a[1]
                .as_f64()
                .ok_or_else(|| format!("vec2[1] must be a number, got {:?}", a[1]))?;
            Ok(ComponentValue::Vec2([x, y]))
        }
        Value::Object(o) => {
            if let Some(type_str) = o.get("type").and_then(|v| v.as_str()) {
                let inner = o
                    .get("value")
                    .ok_or_else(|| format!("tagged {type_str} missing 'value' field"))?;
                match type_str {
                    "Nil" => Ok(ComponentValue::Nil),
                    "Bool" => inner
                        .as_bool()
                        .map(ComponentValue::Bool)
                        .ok_or_else(|| format!("Bool value not bool: {inner:?}")),
                    "Int" => inner
                        .as_i64()
                        .map(ComponentValue::Int)
                        .ok_or_else(|| format!("Int value not int: {inner:?}")),
                    "Float" => inner
                        .as_f64()
                        .map(ComponentValue::Float)
                        .ok_or_else(|| format!("Float value not float: {inner:?}")),
                    "String" => inner
                        .as_str()
                        .map(|s| ComponentValue::String(s.to_string()))
                        .ok_or_else(|| format!("String value not str: {inner:?}")),
                    "Vec2" => {
                        let arr = inner
                            .as_array()
                            .ok_or_else(|| format!("Vec2 value not array: {inner:?}"))?;
                        if arr.len() != 2 {
                            return Err(format!("Vec2 needs 2 elements, got {}", arr.len()));
                        }
                        let x = arr[0]
                            .as_f64()
                            .ok_or_else(|| format!("Vec2[0] not number: {:?}", arr[0]))?;
                        let y = arr[1]
                            .as_f64()
                            .ok_or_else(|| format!("Vec2[1] not number: {:?}", arr[1]))?;
                        Ok(ComponentValue::Vec2([x, y]))
                    }
                    other => Err(format!("unknown tagged component type: {other}")),
                }
            } else {
                Err(format!("object value not a tagged component: {o:?}"))
            }
        }
    }
}

pub fn component_value_to_json(v: &ComponentValue) -> Value {
    match v {
        ComponentValue::Nil => Value::Null,
        ComponentValue::Bool(b) => Value::Bool(*b),
        ComponentValue::Int(i) => Value::Number((*i).into()),
        ComponentValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ComponentValue::String(s) => Value::String(s.clone()),
        ComponentValue::Vec2([x, y]) => json!([*x, *y]),
    }
}

fn numeric_add(a: &ComponentValue, b: &ComponentValue) -> Option<ComponentValue> {
    match (a, b) {
        (ComponentValue::Int(x), ComponentValue::Int(y)) => Some(ComponentValue::Int(x + y)),
        (ComponentValue::Float(x), ComponentValue::Float(y)) => Some(ComponentValue::Float(x + y)),
        (ComponentValue::Int(x), ComponentValue::Float(y)) => {
            Some(ComponentValue::Float(*x as f64 + y))
        }
        (ComponentValue::Float(x), ComponentValue::Int(y)) => {
            Some(ComponentValue::Float(x + *y as f64))
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyError {
    UnknownNode,
    BadComponentType,
}

pub fn apply_commands(
    scene: &mut Scene,
    registry: &NodeRegistry,
    cmds: Vec<ActionCommand>,
) -> Vec<LogEntry> {
    apply_commands_with_animations(scene, registry, cmds, &mut None)
}

pub fn apply_commands_with_animations(
    scene: &mut Scene,
    registry: &NodeRegistry,
    cmds: Vec<ActionCommand>,
    animations_out: &mut Option<&mut SceneState>,
) -> Vec<LogEntry> {
    let mut logs = Vec::new();
    for cmd in cmds {
        match cmd {
            ActionCommand::SetState { target, key, value } => {
                if let Some(node) = resolve_target_mut(scene, &target) {
                    if key == "_active_state" {
                        if let ComponentValue::String(s) = &value {
                            node.active_state = Some(s.clone());
                        }
                    } else {
                        node.components.insert(
                            key,
                            Component {
                                value,
                                kind: Default::default(),
                            },
                        );
                    }
                }
            }
            ActionCommand::Destroy { target } => {
                let id = match &target {
                    Target::Node { id } => Some(id.clone()),
                    Target::This => None,
                    _ => None,
                };
                if let Some(id) = id {
                    scene.nodes.retain(|n| n.id != id);
                }
            }
            ActionCommand::Spawn {
                node_type,
                parent,
                components,
                behaviors,
            } => {
                let parent_id = match &parent {
                    Target::Node { id } => Some(id.clone()),
                    Target::This => None,
                    _ => None,
                };
                if registry.get(&node_type).is_none() {
                    continue;
                }
                let id = format!("{}_{}", node_type, scene.spawn_counter);
                scene.spawn_counter += 1;
                let comps = components
                    .into_iter()
                    .map(|(k, v)| {
                        (
                            k,
                            Component {
                                value: v,
                                kind: Default::default(),
                            },
                        )
                    })
                    .collect();
                scene.nodes.push(Node {
                    id,
                    type_name: node_type,
                    parent: parent_id,
                    components: comps,
                    behaviors,
                    active_state: None,
                    destroyed: false,
                });
            }
            ActionCommand::Emit { signal, args } => {
                let _ = (signal, args);
            }
            ActionCommand::StartAnimation {
                target,
                key,
                to,
                remaining,
            } => {
                if let Some(target_node) = resolve_target_mut(scene, &target) {
                    if let Some(current) = target_node.components.get(&key) {
                        let anim = Animation {
                            key: key.clone(),
                            from: current.value.clone(),
                            to: to.clone(),
                            remaining,
                        };
                        let node_id = target_node.id.clone();
                        if let Some(anim_state) = animations_out {
                            anim_state.animations.entry(node_id).or_default().push(anim);
                        }
                    }
                }
            }
            ActionCommand::Log {
                level,
                message,
                fields,
            } => {
                logs.push(LogEntry {
                    level,
                    message,
                    fields,
                });
            }
            ActionCommand::CallSystem { .. } => {}
        }
    }
    logs
}

#[allow(dead_code)]
fn _unused_check_schema_object(_v: &Value) {
    let _ = _v;
}

pub fn resolve_target_mut<'a>(scene: &'a mut Scene, target: &Target) -> Option<&'a mut Node> {
    match target {
        Target::This => scene.nodes.iter_mut().next(),
        Target::Node { id } => scene.nodes.iter_mut().find(|n| &n.id == id),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub fields: BTreeMap<String, Value>,
}

pub fn evaluate_dry_run(
    scene: &Scene,
    registry: &NodeRegistry,
    node_id: &str,
    actions: &[Action],
) -> EngineResult<Vec<ComponentChange>> {
    let Some(node) = scene.nodes.iter().find(|n| n.id == node_id) else {
        return Err(crate::error::EngineError::Internal(format!(
            "dryRun: node \"{node_id}\" not found"
        )));
    };
    let mut cmds = Vec::new();
    let mut pending_writes = std::collections::BTreeMap::new();
    for action in actions {
        evaluate_action(
            scene,
            registry,
            node,
            action,
            &mut cmds,
            &mut pending_writes,
            0,
        );
    }
    let changes = compute_dry_run_diff(scene, node_id, &cmds, &pending_writes);
    Ok(changes)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ComponentChange {
    Added {
        key: String,
        value: crate::scene::ComponentValue,
    },
    Updated {
        key: String,
        from: crate::scene::ComponentValue,
        to: crate::scene::ComponentValue,
    },
    Removed {
        key: String,
        previous: crate::scene::ComponentValue,
    },
}

fn compute_dry_run_diff(
    scene: &Scene,
    node_id: &str,
    cmds: &[ActionCommand],
    pending_writes: &std::collections::BTreeMap<(String, String), crate::scene::ComponentValue>,
) -> Vec<ComponentChange> {
    use crate::scene::ComponentValue;
    let mut changes: std::collections::BTreeMap<String, ComponentChange> =
        std::collections::BTreeMap::new();
    for cmd in cmds {
        let (target, key, new_value_opt) = match cmd {
            ActionCommand::SetState { target, key, value } => {
                (target, key.as_str(), Some(value.clone()))
            }
            ActionCommand::StartAnimation {
                target, key, to, ..
            } => (target, key.as_str(), Some(to.clone())),
            _ => continue,
        };
        let resolved_id = match target {
            Target::Node { id } => id.clone(),
            _ => node_id.to_string(),
        };
        if resolved_id != node_id {
            continue;
        }
        let key_owned = key.to_string();
        if let Some(new_value) = new_value_opt {
            let prev = scene
                .nodes
                .iter()
                .find(|n| n.id == resolved_id)
                .and_then(|n| n.components.get(key))
                .map(|c| c.value.clone());
            let entry = changes
                .entry(key_owned.clone())
                .or_insert_with(|| match prev {
                    Some(ComponentValue::Nil) | None => ComponentChange::Added {
                        key: key_owned.clone(),
                        value: new_value.clone(),
                    },
                    Some(p) => ComponentChange::Updated {
                        key: key_owned.clone(),
                        from: p,
                        to: new_value.clone(),
                    },
                });
            if let ComponentChange::Updated { to, .. } = entry {
                *to = new_value.clone();
            }
            if let ComponentChange::Added { value, .. } = entry {
                *value = new_value.clone();
            }
            let _ = pending_writes.get(&(resolved_id.clone(), key_owned.clone()));
        }
    }
    changes.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Player;
    impl Default for Player {
        fn default() -> Self {
            Player
        }
    }
    impl crate::scene::NodeDef for Player {
        fn type_name(&self) -> &'static str {
            "Player"
        }
        fn component_specs(&self) -> Vec<crate::scene::ComponentSpec> {
            vec![
                crate::scene::ComponentSpec::new(
                    "position",
                    crate::scene::ComponentType::Vec2,
                    json!([0.0, 0.0]),
                ),
                crate::scene::ComponentSpec::new(
                    "health",
                    crate::scene::ComponentType::Int,
                    json!(100),
                ),
            ]
        }
    }

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        r.register::<Player>();
        r
    }

    fn player_node(health: i64) -> Node {
        let mut components = BTreeMap::new();
        components.insert(
            "position".to_string(),
            Component {
                value: ComponentValue::Vec2([0.0, 0.0]),
                kind: Default::default(),
            },
        );
        components.insert(
            "health".to_string(),
            Component {
                value: ComponentValue::Int(health),
                kind: Default::default(),
            },
        );
        Node {
            id: "p1".to_string(),
            type_name: "Player".to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            destroyed: false,
        }
    }

    #[test]
    fn eval_self_dot_health() {
        let scene = Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes: vec![player_node(0)],
            spawn_counter: 0,
        };
        let node = &scene.nodes[0];
        let e = Expression::from_value(json!({"eq": ["self.health", 0]}));
        let v = eval_expression(
            &scene,
            &registry(),
            node,
            &e,
            &std::collections::BTreeMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(v, json!(true));
    }

    #[test]
    fn eval_add_two_numbers() {
        let scene = Scene {
            kind: "scene".to_string(),
            name: "t".to_string(),
            nodes: vec![player_node(0)],
            spawn_counter: 0,
        };
        let node = &scene.nodes[0];
        let e = Expression::from_value(json!({"add": [3, 4]}));
        let v = eval_expression(
            &scene,
            &registry(),
            node,
            &e,
            &std::collections::BTreeMap::new(),
            0,
        )
        .unwrap();
        assert_eq!(v, json!(7));
    }
}
