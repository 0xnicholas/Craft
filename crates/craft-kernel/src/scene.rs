use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::behavior::Behavior;
use crate::error::{
    AutoFix, EngineError, EngineResult, ErrorCollector, ParseError, ValidationError,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ComponentValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    Rect([f64; 4]),
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    #[default]
    Regular,
    Transient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    pub value: ComponentValue,
    #[serde(default, skip_serializing_if = "is_regular")]
    pub kind: ComponentKind,
}

fn is_regular(k: &ComponentKind) -> bool {
    *k == ComponentKind::Regular
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default)]
    pub components: BTreeMap<String, Component>,
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_state: Option<String>,
    /// Optional Lua class binding (ADR 0016). When set, the craft-lua
    /// runtime instantiates the class by name and fires its on_tick /
    /// on_signal / on_spawn hooks each tick (before JSON behaviors).
    /// The class itself is loaded into the runtime separately via
    /// `LuaRuntime::load_class(name, source)`; this field carries only
    /// the class name (not a path or source).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lua_class: Option<String>,
    /// Runtime-only despawn flag. Set by `Node::mark_destroyed` so scripts
    /// can request removal mid-tick without mutating `Scene.nodes` while a
    /// behavior or Lua script may still iterate it. Persist via
    /// `Scene::purge_destroyed` between ticks.
    #[serde(skip)]
    pub destroyed: bool,
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            id: String,
            #[serde(rename = "type")]
            type_name: String,
            #[serde(default)]
            parent: Option<String>,
            #[serde(default)]
            components: BTreeMap<String, serde_json::Value>,
            #[serde(default)]
            behaviors: Vec<Behavior>,
            #[serde(default)]
            active_state: Option<String>,
            #[serde(default)]
            lua_class: Option<String>,
        }

        let raw = Raw::deserialize(deserializer)?;
        let mut components = BTreeMap::new();
        for (k, v) in raw.components {
            components.insert(
                k,
                component_from_value(v).map_err(serde::de::Error::custom)?,
            );
        }
        Ok(Node {
            id: raw.id,
            type_name: raw.type_name,
            parent: raw.parent,
            components,
            behaviors: raw.behaviors,
            active_state: raw.active_state,
            lua_class: raw.lua_class,
            destroyed: false,
        })
    }
}

fn component_from_value(v: Value) -> Result<Component, String> {
    match v {
        Value::Object(obj) => {
            let kind = match obj.get("kind").and_then(Value::as_str) {
                Some("transient") => ComponentKind::Transient,
                Some("regular") | None => ComponentKind::Regular,
                Some(other) => return Err(format!("unknown component kind \"{other}\"")),
            };
            let value_json = obj
                .get("value")
                .ok_or_else(|| "structured component requires a `value` field".to_string())?;
            let value = json_to_component_value(value_json.clone())?;
            Ok(Component { value, kind })
        }
        other => {
            let value = json_to_component_value(other)?;
            Ok(Component {
                value,
                kind: ComponentKind::Regular,
            })
        }
    }
}

fn json_to_component_value(v: Value) -> Result<ComponentValue, String> {
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
                return Err(format!(
                    "expected [x, y] for vec2, got array of length {}",
                    a.len()
                ));
            }
            let x = a[0]
                .as_f64()
                .ok_or_else(|| format!("vec2[0] must be a number, got {:?}", a[0]))?;
            let y = a[1]
                .as_f64()
                .ok_or_else(|| format!("vec2[1] must be a number, got {:?}", a[1]))?;
            Ok(ComponentValue::Vec2([x, y]))
        }
        Value::Object(_) => Err(
            "nested object is not a valid component value; use {\"type\": ..., \"value\": ...}"
                .to_string(),
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    #[serde(default)]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub spawn_counter: u64,
}

pub const SCENE_KIND: &str = "scene";

impl Scene {
    pub fn load(path: &Path, registry: &NodeRegistry) -> EngineResult<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            EngineError::Io(crate::error::IoError {
                file: path.display().to_string(),
                kind: crate::error::IoErrorKind::Read,
                message: e.to_string(),
            })
        })?;
        let file = path.display().to_string();
        Self::parse(&contents, &file, registry)
    }

    pub fn parse(contents: &str, file: &str, registry: &NodeRegistry) -> EngineResult<Self> {
        let value: Value = serde_json::from_str(contents).map_err(|e| {
            EngineError::Parse(ParseError {
                file: file.to_string(),
                line: Some(e.line() as u32),
                column: Some(e.column() as u32),
                message: e.to_string(),
                snippet: snippet_around(contents, Some(e.line())),
            })
        })?;
        Self::from_value(value, file, registry)
    }

    pub fn from_value(value: Value, file: &str, registry: &NodeRegistry) -> EngineResult<Self> {
        let mut errors = ErrorCollector::new(file);

        let kind = value.get("kind").and_then(Value::as_str).unwrap_or("");
        if kind != SCENE_KIND {
            errors.push(ValidationError {
                file: file.to_string(),
                json_path: "$.kind".to_string(),
                message: format!("expected kind \"{SCENE_KIND}\", got {kind:?}"),
                expected_type: format!("string literal \"{SCENE_KIND}\""),
                actual_value: value.get("kind").cloned(),
                suggestion: Some(format!("Set \"kind\": \"{SCENE_KIND}\" at the top level")),
                auto_fixable: AutoFix::Safe,
            });
        }

        reject_unknown_top_level_fields(&value, file, &mut errors);

        let scene: Scene = serde_json::from_value(value.clone()).map_err(|e| {
            EngineError::Parse(ParseError {
                file: file.to_string(),
                line: Some(e.line() as u32),
                column: Some(e.column() as u32),
                message: e.to_string(),
                snippet: None,
            })
        })?;

        let mut seen_ids = std::collections::HashSet::new();
        for (i, node) in scene.nodes.iter().enumerate() {
            if !seen_ids.insert(node.id.as_str()) {
                errors.push(ValidationError {
                    file: file.to_string(),
                    json_path: format!("$.nodes[{i}].id"),
                    message: format!("duplicate node id \"{}\"", node.id),
                    expected_type: "unique string id".to_string(),
                    actual_value: Some(json!(node.id)),
                    suggestion: Some("Give each node a distinct id".to_string()),
                    auto_fixable: AutoFix::NeedsReview,
                });
            }
            validate_node(node, i, file, registry, &mut errors);
        }

        errors.into_result()?;
        Ok(scene)
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).expect("scene serialization is infallible")
    }

    /// Find a live (not destroyed) node by id.
    pub fn find_node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| !n.destroyed && n.id == id)
    }

    /// Find a live (not destroyed) node by id.
    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| !n.destroyed && n.id == id)
    }

    /// Find a node by id regardless of `destroyed` state. Used to mark
    /// already-destroyed nodes (no-op) or to operate on nodes that were
    /// queued for removal by another path.
    pub fn find_node_mut_raw(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    /// Reserved-prefix spawn id; never collides with user-authored ids in
    /// scene.json. `purge_destroyed` does not touch the counter.
    pub fn next_spawn_id(&mut self, type_name: &str) -> String {
        let id = format!("__spawn_{}_{}", type_name, self.spawn_counter);
        self.spawn_counter = self.spawn_counter.wrapping_add(1);
        id
    }

    /// Physically remove all nodes marked `destroyed`. Call between ticks (or
    /// at the end of a Lua run) to apply pending despawns. Returns the count
    /// removed.
    pub fn purge_destroyed(&mut self) -> usize {
        let before = self.nodes.len();
        self.nodes.retain(|n| !n.destroyed);
        before - self.nodes.len()
    }
}

impl Node {
    /// Mark this node for deferred despawn. The node will be filtered out of
    /// `find_node` / `find_node_mut` immediately, but stays in `scene.nodes`
    /// until `Scene::purge_destroyed` is called.
    pub fn mark_destroyed(&mut self) {
        self.destroyed = true;
    }

    pub fn get_component_value(&self, key: &str) -> Option<&ComponentValue> {
        self.components.get(key).map(|c| &c.value)
    }

    pub fn get_component_value_mut(&mut self, key: &str) -> Option<&mut ComponentValue> {
        self.components.get_mut(key).map(|c| &mut c.value)
    }

    /// Strict write to an existing component. Returns a structured `Validation`
    /// error if the component key is absent, mirroring the JSON scene schema's
    /// "components must be declared in the node type" invariant. Use this from
    /// Rust APIs that want guaranteed schemas. Lua scripts and other dynamic
    /// callers should mutate `node.components.entry(key)` directly to allow
    /// creating new component slots.
    pub fn set_component_value(&mut self, key: &str, value: ComponentValue) -> EngineResult<()> {
        match self.components.get_mut(key) {
            Some(component) => {
                component.value = value;
                Ok(())
            }
            None => Err(EngineError::Validation {
                file: String::new(),
                errors: vec![crate::error::ValidationError {
                    file: String::new(),
                    json_path: format!("$.nodes[?id==\"{}\"].components.{}", self.id, key),
                    message: format!("node \"{}\" has no component \"{}\"", self.id, key),
                    expected_type: "existing component key".to_string(),
                    actual_value: None,
                    suggestion: Some(
                        "Define the component in the node's \"components\" block before reading or writing it"
                            .to_string(),
                    ),
                    auto_fixable: AutoFix::NeedsReview,
                }],
            }),
        }
    }
}

pub fn hash_scene_state(scene: &Scene) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    let v = scene.to_value();
    canonicalize(&v).hash(&mut hasher);
    hasher.finish()
}

fn canonicalize(v: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical(&mut out, v);
    out
}

fn write_canonical(out: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => out.push(0),
        Value::Bool(b) => {
            out.push(1);
            out.push(u8::from(*b));
        }
        Value::Number(n) => {
            out.push(2);
            if let Some(i) = n.as_i64() {
                out.extend_from_slice(&i.to_le_bytes());
            } else if let Some(f) = n.as_f64() {
                out.extend_from_slice(&f.to_bits().to_le_bytes());
            } else {
                out.push(0);
            }
        }
        Value::String(s) => {
            out.push(3);
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        Value::Array(a) => {
            out.push(4);
            out.extend_from_slice(&(a.len() as u32).to_le_bytes());
            for item in a {
                write_canonical(out, item);
            }
        }
        Value::Object(o) => {
            out.push(5);
            let mut entries: Vec<(&String, &Value)> = o.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
            for (k, v) in entries {
                out.extend_from_slice(&(k.len() as u32).to_le_bytes());
                out.extend_from_slice(k.as_bytes());
                write_canonical(out, v);
            }
        }
    }
}

fn reject_unknown_top_level_fields(value: &Value, file: &str, errors: &mut ErrorCollector) {
    let Some(obj) = value.as_object() else {
        return;
    };
    const KNOWN: &[&str] = &["kind", "name", "nodes", "spawn_counter"];
    let mut unknown: Vec<&str> = obj
        .keys()
        .map(String::as_str)
        .filter(|k| !KNOWN.contains(k))
        .collect();
    unknown.sort_unstable();
    for key in unknown {
        errors.push(ValidationError {
            file: file.to_string(),
            json_path: format!("$.{key}"),
            message: format!("unknown top-level field \"{key}\""),
            expected_type: format!("one of [{}]", KNOWN.join(", ")),
            actual_value: Some(json!(key)),
            suggestion: Some(format!(
                "Remove \"{key}\" or rename to one of: {}",
                KNOWN.join(", ")
            )),
            auto_fixable: AutoFix::Safe,
        });
    }
}

fn validate_node(
    node: &Node,
    index: usize,
    file: &str,
    registry: &NodeRegistry,
    errors: &mut ErrorCollector,
) {
    let path = |suffix: &str| format!("$.nodes[{index}]{suffix}");

    let Some(def) = registry.get(&node.type_name) else {
        errors.push(ValidationError {
            file: file.to_string(),
            json_path: path(".type"),
            message: format!("unknown node type \"{}\"", node.type_name),
            expected_type: "known node type".to_string(),
            actual_value: Some(json!(node.type_name)),
            suggestion: Some(suggest_known_type(&node.type_name, registry)),
            auto_fixable: AutoFix::Suggested,
        });
        return;
    };

    let specs = def.component_specs();
    let mut known: BTreeMap<&str, &ComponentSpec> = BTreeMap::new();
    for spec in specs {
        known.insert(spec.name.as_str(), spec);
    }

    let mut unexpected: Vec<&str> = node
        .components
        .keys()
        .map(String::as_str)
        .filter(|k| !known.contains_key(k))
        .collect();
    unexpected.sort_unstable();
    for key in unexpected {
        let mut known_names: Vec<&str> = known.keys().copied().collect();
        known_names.sort_unstable();
        let suggestion = suggest_rename(key, &known_names).unwrap_or_else(|| {
            format!("Remove \"{key}\" or use one of: {}", known_names.join(", "))
        });
        errors.push(ValidationError {
            file: file.to_string(),
            json_path: path(&format!(".components.{key}")),
            message: format!(
                "unknown component \"{key}\" for node type \"{}\"",
                node.type_name
            ),
            expected_type: format!("one of [{}]", known_names.join(", ")),
            actual_value: Some(json!(key)),
            suggestion: Some(suggestion),
            auto_fixable: AutoFix::NeedsReview,
        });
    }

    for (key, component) in &node.components {
        let Some(spec) = known.get(key.as_str()) else {
            continue;
        };
        if !component_value_matches(&component.value, spec.ty) {
            errors.push(ValidationError {
                file: file.to_string(),
                json_path: path(&format!(".components.{key}")),
                message: format!("component \"{key}\" has wrong type, expected {}", spec.ty),
                expected_type: spec.ty.to_string(),
                actual_value: Some(json!(component.value)),
                suggestion: Some(format!(
                    "Replace with a {} value, e.g. {}",
                    spec.ty, spec.default
                )),
                auto_fixable: AutoFix::Suggested,
            });
        }
    }

    let declared: std::collections::HashSet<&str> = known.keys().copied().collect();
    let missing: Vec<&str> = declared
        .iter()
        .copied()
        .filter(|k| !node.components.contains_key(*k))
        .collect();
    let mut missing_sorted = missing;
    missing_sorted.sort_unstable();
    for key in missing_sorted {
        let Some(spec) = known.get(key) else { continue };
        errors.push(ValidationError {
            file: file.to_string(),
            json_path: path(&format!(".components.{key}")),
            message: format!("missing required component \"{key}\""),
            expected_type: format!("present {} value", spec.ty),
            actual_value: Some(Value::Null),
            suggestion: Some(format!("Add \"{key}\": {} (default)", spec.default)),
            auto_fixable: AutoFix::Safe,
        });
    }
}

fn suggest_rename(unknown: &str, known: &[&str]) -> Option<String> {
    let mut best: Option<(&str, usize)> = None;
    for name in known {
        let dist = levenshtein(unknown, name);
        if dist <= 3 && (best.is_none() || dist < best.unwrap().1) {
            best = Some((name, dist));
        }
    }
    best.map(|(name, _)| format!("Did you mean \"{name}\"?"))
}

fn suggest_known_type(unknown: &str, registry: &NodeRegistry) -> String {
    let names: Vec<&str> = {
        let mut v: Vec<&str> = registry.type_names().collect();
        v.sort_unstable();
        v
    };
    if let Some(s) = suggest_rename(unknown, &names) {
        return s;
    }
    format!("Known node types: {}", names.join(", "))
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        curr[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn component_value_matches(value: &ComponentValue, ty: ComponentType) -> bool {
    matches!(
        (value, ty),
        (ComponentValue::Nil, ComponentType::Nil)
            | (ComponentValue::Bool(_), ComponentType::Bool)
            | (ComponentValue::Int(_), ComponentType::Int)
            | (ComponentValue::Float(_), ComponentType::Float)
            | (ComponentValue::String(_), ComponentType::String)
            | (ComponentValue::Vec2(_), ComponentType::Vec2)
            | (ComponentValue::Vec3(_), ComponentType::Vec3)
            | (ComponentValue::Rect(_), ComponentType::Rect)
    )
}

fn snippet_around(contents: &str, line: Option<usize>) -> Option<String> {
    let line = line?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = line.saturating_sub(1);
    let end = (line + 2).min(lines.len());
    Some(lines[start..end].join("\n"))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Vec2,
    Vec3,
    Rect,
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Nil => "nil",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::Vec2 => "vec2",
            Self::Vec3 => "vec3",
            Self::Rect => "rect",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub name: String,
    pub ty: ComponentType,
    pub default: Value,
}

impl ComponentSpec {
    pub fn new(name: impl Into<String>, ty: ComponentType, default: Value) -> Self {
        Self {
            name: name.into(),
            ty,
            default,
        }
    }
}

pub trait NodeDef: 'static {
    fn type_name(&self) -> &'static str;
    fn component_specs(&self) -> Vec<ComponentSpec>;
}

pub struct NodeRegistration {
    pub name: &'static str,
    pub instantiate: fn() -> Box<dyn NodeDef>,
}

inventory::collect!(NodeRegistration);

pub fn collected_node_defs() -> Vec<NodeRegistration> {
    inventory::iter::<NodeRegistration>()
        .map(|r| NodeRegistration {
            name: r.name,
            instantiate: r.instantiate,
        })
        .collect()
}

#[derive(Debug, Default, Clone)]
pub struct NodeRegistry {
    by_name: BTreeMap<String, RegistryEntry>,
}

#[derive(Debug, Clone)]
struct RegistryEntry {
    type_name: &'static str,
    specs: Vec<ComponentSpec>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<D: NodeDef + Default>(&mut self) {
        let instance = D::default();
        self.by_name.insert(
            instance.type_name().to_string(),
            RegistryEntry {
                type_name: instance.type_name(),
                specs: instance.component_specs(),
            },
        );
    }

    pub fn get(&self, type_name: &str) -> Option<NodeTypeView<'_>> {
        self.by_name.get(type_name).map(|e| NodeTypeView {
            type_name: e.type_name,
            specs: &e.specs,
        })
    }

    pub fn type_names(&self) -> impl Iterator<Item = &str> {
        self.by_name.keys().map(|s| s.as_str())
    }

    pub fn instantiate_all(&mut self) {
        self.by_name.clear();
        for reg in collected_node_defs() {
            let instance = (reg.instantiate)();
            self.by_name.insert(
                reg.name.to_string(),
                RegistryEntry {
                    type_name: instance.type_name(),
                    specs: instance.component_specs(),
                },
            );
        }
    }
}

pub struct NodeTypeView<'a> {
    pub type_name: &'static str,
    pub specs: &'a [ComponentSpec],
}

impl<'a> NodeTypeView<'a> {
    pub fn component_specs(&self) -> &[ComponentSpec] {
        self.specs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Player;
    impl Default for Player {
        fn default() -> Self {
            Player
        }
    }
    impl NodeDef for Player {
        fn type_name(&self) -> &'static str {
            "Player"
        }
        fn component_specs(&self) -> Vec<ComponentSpec> {
            vec![
                ComponentSpec::new("position", ComponentType::Vec2, json!([0.0, 0.0])),
                ComponentSpec::new("health", ComponentType::Int, json!(100)),
            ]
        }
    }

    struct Enemy;
    impl Default for Enemy {
        fn default() -> Self {
            Enemy
        }
    }
    impl NodeDef for Enemy {
        fn type_name(&self) -> &'static str {
            "Enemy"
        }
        fn component_specs(&self) -> Vec<ComponentSpec> {
            vec![ComponentSpec::new(
                "position",
                ComponentType::Vec2,
                json!([0.0, 0.0]),
            )]
        }
    }

    struct Wizard;
    impl Default for Wizard {
        fn default() -> Self {
            Wizard
        }
    }
    impl NodeDef for Wizard {
        fn type_name(&self) -> &'static str {
            "Wizard"
        }
        fn component_specs(&self) -> Vec<ComponentSpec> {
            vec![
                ComponentSpec::new("position", ComponentType::Vec2, json!([0.0, 0.0])),
                ComponentSpec::new("health", ComponentType::Int, json!(100)),
                ComponentSpec::new("flash", ComponentType::String, json!("normal")),
            ]
        }
    }

    fn registry_with_wizard() -> NodeRegistry {
        let mut r = registry();
        r.register::<Wizard>();
        r
    }

    fn registry() -> NodeRegistry {
        let mut r = NodeRegistry::new();
        r.register::<Player>();
        r.register::<Enemy>();
        r
    }

    #[test]
    fn loads_valid_scene() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                {
                    "id": "player_1",
                    "type": "Player",
                    "components": {
                        "position": [10.0, 20.0],
                        "health": 100
                    }
                }
            ]
        }"#;
        let scene = Scene::parse(json, "scene.json", &registry()).expect("parse");
        assert_eq!(scene.kind, "scene");
        assert_eq!(scene.name, "main");
        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.nodes[0].id, "player_1");
        assert_eq!(scene.nodes[0].type_name, "Player");
    }

    #[test]
    fn rejects_missing_kind() {
        let json = r#"{
            "name": "main",
            "nodes": []
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                assert!(errors.iter().any(|e| e.json_path == "$.kind"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_wrong_kind() {
        let json = r#"{
            "kind": "level",
            "name": "main",
            "nodes": []
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors.iter().find(|e| e.json_path == "$.kind").unwrap();
                assert_eq!(e.expected_type, "string literal \"scene\"");
                assert_eq!(e.auto_fixable, AutoFix::Safe);
                assert!(e.suggestion.is_some());
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_node_type_with_typo_suggestion() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                { "id": "p1", "type": "Plyer", "components": {"position": [0.0, 0.0], "health": 100} }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors
                    .iter()
                    .find(|e| e.json_path == "$.nodes[0].type")
                    .unwrap();
                assert!(e.suggestion.as_deref().unwrap().contains("Player"));
                assert_eq!(e.auto_fixable, AutoFix::Suggested);
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn unknown_component_key_uses_needs_review() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                { "id": "p1", "type": "Player", "components": { "position": [0.0, 0.0], "health": 100, "helth": 100 } }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors
                    .iter()
                    .find(|e| e.json_path == "$.nodes[0].components.helth")
                    .unwrap();
                assert_eq!(
                    e.auto_fixable,
                    AutoFix::NeedsReview,
                    "ADR 0008: typo may be intended; never auto-delete without review"
                );
                assert!(e.suggestion.as_deref().unwrap().contains("health"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_wrong_component_type() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                { "id": "p1", "type": "Player", "components": { "position": [0.0, 0.0], "health": "fast" } }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors
                    .iter()
                    .find(|e| e.json_path == "$.nodes[0].components.health")
                    .unwrap();
                assert_eq!(e.expected_type, "int");
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn reports_missing_required_component() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                { "id": "p1", "type": "Player", "components": { "position": [0.0, 0.0] } }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors
                    .iter()
                    .find(|e| e.json_path == "$.nodes[0].components.health")
                    .unwrap();
                assert_eq!(e.auto_fixable, AutoFix::Safe);
                assert!(e.suggestion.as_deref().unwrap().contains("100"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodez": [],
            "nodes": []
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors.iter().find(|e| e.json_path == "$.nodez").unwrap();
                assert_eq!(e.auto_fixable, AutoFix::Safe);
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_duplicate_node_ids() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                { "id": "p1", "type": "Player", "components": { "position": [0.0, 0.0], "health": 100 } },
                { "id": "p1", "type": "Player", "components": { "position": [1.0, 1.0], "health": 50 } }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        match err {
            EngineError::Validation { errors, .. } => {
                let e = errors
                    .iter()
                    .find(|e| e.json_path == "$.nodes[1].id")
                    .unwrap();
                assert!(e.message.contains("duplicate"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn collects_multiple_errors() {
        let json = r#"{
            "kind": "level",
            "name": "main",
            "nodez": [],
            "nodes": [
                { "id": "p1", "type": "Player", "components": { "position": [0.0, 0.0], "health": "fast" } },
                { "id": "p2", "type": "Plyer", "components": {} }
            ]
        }"#;
        let err = Scene::parse(json, "scene.json", &registry()).expect_err("must fail");
        if let EngineError::Validation { errors, .. } = err {
            assert!(
                errors.len() >= 4,
                "expected >=4 errors, got {}: {:#?}",
                errors.len(),
                errors
            );
        } else {
            panic!("expected validation error");
        }
    }

    #[test]
    fn accepts_empty_node_components() {
        let json = r#"{
            "kind": "scene",
            "name": "empty",
            "nodes": []
        }"#;
        let scene = Scene::parse(json, "scene.json", &registry()).expect("parse");
        assert_eq!(scene.kind, "scene");
        assert!(scene.nodes.is_empty());
    }

    #[test]
    fn structured_component_form_round_trips() {
        let json = r#"{
            "kind": "scene",
            "name": "main",
            "nodes": [
                {
                    "id": "w1",
                    "type": "Wizard",
                    "components": {
                        "position": [1.0, 2.0],
                        "health": 75,
                        "flash": { "type": "String", "value": "hit", "kind": "transient" }
                    }
                }
            ]
        }"#;
        let scene = Scene::parse(json, "scene.json", &registry_with_wizard()).expect("parse");
        let serialized = serde_json::to_string(&scene).expect("serialize");
        let reloaded =
            Scene::parse(&serialized, "scene.json", &registry_with_wizard()).expect("reload");
        assert_eq!(reloaded.nodes.len(), 1);
        assert_eq!(
            reloaded.nodes[0].components["flash"].value,
            ComponentValue::String("hit".to_string())
        );
        assert_eq!(
            reloaded.nodes[0].components["flash"].kind,
            ComponentKind::Transient
        );
        assert_eq!(
            reloaded.nodes[0].components["health"].kind,
            ComponentKind::Regular
        );
    }

    fn make_node(id: &str, type_name: &str, components: &[(&str, ComponentValue)]) -> Node {
        let mut map = BTreeMap::new();
        for (k, v) in components {
            map.insert(
                (*k).to_string(),
                Component {
                    value: v.clone(),
                    kind: ComponentKind::Regular,
                },
            );
        }
        Node {
            id: id.to_string(),
            type_name: type_name.to_string(),
            parent: None,
            components: map,
            behaviors: Vec::new(),
            active_state: None,
            lua_class: None,
            destroyed: false,
        }
    }

    #[test]
    fn find_node_returns_matching_node() {
        let mut scene = Scene {
            kind: SCENE_KIND.to_string(),
            name: "s".to_string(),
            nodes: Vec::new(),
            spawn_counter: 0,
        };
        scene.add_node(make_node("a", "X", &[("hp", ComponentValue::Int(10))]));
        scene.add_node(make_node("b", "Y", &[("hp", ComponentValue::Int(20))]));
        assert_eq!(
            scene.find_node("a").map(|n| n.type_name.as_str()),
            Some("X")
        );
        assert!(scene.find_node("missing").is_none());
    }

    #[test]
    fn next_spawn_id_increments_counter() {
        let mut scene = Scene {
            kind: SCENE_KIND.to_string(),
            name: "s".to_string(),
            nodes: Vec::new(),
            spawn_counter: 0,
        };
        assert_eq!(scene.next_spawn_id("Enemy"), "__spawn_Enemy_0");
        assert_eq!(scene.next_spawn_id("Enemy"), "__spawn_Enemy_1");
        assert_eq!(scene.next_spawn_id("Tower"), "__spawn_Tower_2");
    }

    #[test]
    fn set_and_get_component_value_round_trips() {
        let mut node = make_node("a", "X", &[("hp", ComponentValue::Int(10))]);
        assert_eq!(
            node.get_component_value("hp"),
            Some(&ComponentValue::Int(10))
        );
        node.set_component_value("hp", ComponentValue::Int(99))
            .unwrap();
        assert_eq!(
            node.get_component_value("hp"),
            Some(&ComponentValue::Int(99))
        );
    }

    #[test]
    fn set_component_value_errors_on_missing_key() {
        let mut node = make_node("a", "X", &[("hp", ComponentValue::Int(10))]);
        let err = node
            .set_component_value("mana", ComponentValue::Int(5))
            .unwrap_err();
        assert!(matches!(err, EngineError::Validation { .. }));
    }

    #[test]
    fn find_node_skips_destroyed_nodes() {
        let mut scene = Scene {
            kind: SCENE_KIND.to_string(),
            name: "s".to_string(),
            nodes: Vec::new(),
            spawn_counter: 0,
        };
        scene.add_node(make_node("a", "X", &[("hp", ComponentValue::Int(10))]));
        scene.add_node(make_node("b", "Y", &[("hp", ComponentValue::Int(20))]));
        assert!(scene.find_node("a").is_some());
        if let Some(node) = scene.find_node_mut_raw("a") {
            node.mark_destroyed();
        }
        assert!(
            scene.find_node("a").is_none(),
            "find_node must skip destroyed nodes"
        );
        assert!(
            scene.find_node_mut("a").is_none(),
            "find_node_mut must skip destroyed nodes"
        );
        assert!(
            scene.find_node_mut_raw("a").is_some(),
            "find_node_mut_raw ignores destroyed flag"
        );
    }

    #[test]
    fn purge_destroyed_physically_removes_marked_nodes() {
        let mut scene = Scene {
            kind: SCENE_KIND.to_string(),
            name: "s".to_string(),
            nodes: Vec::new(),
            spawn_counter: 0,
        };
        scene.add_node(make_node("a", "X", &[("hp", ComponentValue::Int(10))]));
        scene.add_node(make_node("b", "Y", &[("hp", ComponentValue::Int(20))]));
        scene.add_node(make_node("c", "Z", &[("hp", ComponentValue::Int(30))]));
        scene.find_node_mut_raw("a").unwrap().mark_destroyed();
        scene.find_node_mut_raw("c").unwrap().mark_destroyed();
        let removed = scene.purge_destroyed();
        assert_eq!(removed, 2);
        assert_eq!(scene.nodes.len(), 1);
        assert_eq!(scene.nodes[0].id, "b");
    }

    #[test]
    fn set_component_value_succeeds_on_existing_key() {
        let mut node = make_node("a", "X", &[("hp", ComponentValue::Int(10))]);
        node.set_component_value("hp", ComponentValue::Int(99))
            .unwrap();
        assert_eq!(
            node.get_component_value("hp"),
            Some(&ComponentValue::Int(99))
        );
    }

    #[test]
    fn get_component_value_mut_returns_mutable_reference() {
        let mut node = make_node("a", "X", &[("hp", ComponentValue::Int(10))]);
        if let Some(v) = node.get_component_value_mut("hp") {
            *v = ComponentValue::Int(77);
        } else {
            panic!("expected hp component");
        }
        assert_eq!(
            node.get_component_value("hp"),
            Some(&ComponentValue::Int(77))
        );
    }
}
