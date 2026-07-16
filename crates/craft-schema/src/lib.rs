use std::collections::{BTreeMap, HashMap};

use schemars::JsonSchema;
use schemars::schema::{Schema, SchemaObject};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use craft_kernel::behavior::{Expression, LogLevel, Target};

pub const ACTION_VERBS: &[&str] = &[
    "set_state",
    "emit",
    "destroy",
    "spawn",
    "if",
    "move",
    "animate",
    "log",
    "call_system",
];

pub const SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetStateParams {
    pub target: Target,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmitParams {
    pub signal: String,
    #[serde(default)]
    pub args: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DestroyParams {
    pub target: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SpawnParams {
    #[serde(rename = "type")]
    pub node_type: String,
    pub parent: Target,
    #[serde(default)]
    pub components: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IfParams {
    pub cond: Expression,
    pub then: Vec<ActionParams>,
    #[serde(rename = "else", default)]
    pub else_: Vec<ActionParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MoveParams {
    pub target: Target,
    pub key: String,
    pub by: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnimateParams {
    pub target: Target,
    pub key: String,
    pub to: Value,
    pub duration: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LogParams {
    pub level: LogLevel,
    pub message: String,
    #[serde(default)]
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CallSystemParams {
    pub system: String,
    #[serde(default)]
    pub args: HashMap<String, Value>,
    #[serde(default)]
    pub result_in: Option<ResultTargetParams>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResultTargetParams {
    pub key: String,
    pub on: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionParams {
    #[serde(rename = "set_state")]
    SetState(SetStateParams),
    #[serde(rename = "emit")]
    Emit(EmitParams),
    #[serde(rename = "destroy")]
    Destroy(DestroyParams),
    #[serde(rename = "spawn")]
    Spawn(SpawnParams),
    #[serde(rename = "if")]
    If(IfParams),
    #[serde(rename = "move")]
    Move(MoveParams),
    #[serde(rename = "animate")]
    Animate(AnimateParams),
    #[serde(rename = "log")]
    Log(LogParams),
    #[serde(rename = "call_system")]
    CallSystem(CallSystemParams),
}

pub fn get_action_schema(verb: &str) -> Option<Schema> {
    let mut settings = schemars::r#gen::SchemaSettings::openapi3();
    settings.option_add_null_type = false;
    let generator = settings.into_generator();
    let schema = match verb {
        "set_state" => generator.into_root_schema_for::<SetStateParams>(),
        "emit" => generator.into_root_schema_for::<EmitParams>(),
        "destroy" => generator.into_root_schema_for::<DestroyParams>(),
        "spawn" => generator.into_root_schema_for::<SpawnParams>(),
        "if" => generator.into_root_schema_for::<IfParams>(),
        "move" => generator.into_root_schema_for::<MoveParams>(),
        "animate" => generator.into_root_schema_for::<AnimateParams>(),
        "log" => generator.into_root_schema_for::<LogParams>(),
        "call_system" => generator.into_root_schema_for::<CallSystemParams>(),
        _ => return None,
    };
    serde_json::from_value(serde_json::to_value(schema).ok()?).ok()
}

pub fn action_verb_list() -> Vec<&'static str> {
    ACTION_VERBS.to_vec()
}

pub fn action_verb_descriptions() -> BTreeMap<&'static str, &'static str> {
    let mut out = BTreeMap::new();
    out.insert(
        "set_state",
        "Write a component value on the target node. Writing to a transient component restarts its lifetime.",
    );
    out.insert("emit", "Fire a signal (queued; resolved next tick).");
    out.insert(
        "destroy",
        "Remove a node. References to the destroyed node resolve to `none` at access time.",
    );
    out.insert(
        "spawn",
        "Create a new node at runtime. The `parent` target is the new node's parent.",
    );
    out.insert(
        "if",
        "Conditional execution. May be nested arbitrarily deep inside other `if` / `actions` lists.",
    );
    out.insert(
        "move",
        "Add a delta to any numeric component. The `by` field is a literal or expression.",
    );
    out.insert(
        "animate",
        "Interpolate any component to a target value over `duration` ticks.",
    );
    out.insert(
        "log",
        "Debug output. Surfaces in agent subscription stream.",
    );
    out.insert(
        "call_system",
        "Invoke a registered Rust system. Use `result_in` to write the return value into a component.",
    );
    out
}

pub fn get_full_schema() -> Value {
    let mut settings = schemars::r#gen::SchemaSettings::openapi3();
    settings.option_add_null_type = false;
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<ActionParams>();
    let value = serde_json::to_value(schema).expect("schema serialization");
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "version": SCHEMA_VERSION,
        "title": "CraftAction",
        "description": "Closed-set action vocabulary. Agents cannot invent new verbs.",
        "verbs": ACTION_VERBS,
        "verb_descriptions": action_verb_descriptions(),
        "action_schema": value,
        "behavior_schema": get_behavior_schema_description(),
        "per_verb_schemas": {
            "set_state": get_action_schema("set_state").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "emit": get_action_schema("emit").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "destroy": get_action_schema("destroy").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "spawn": get_action_schema("spawn").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "if": get_action_schema("if").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "move": get_action_schema("move").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "animate": get_action_schema("animate").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "log": get_action_schema("log").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
            "call_system": get_action_schema("call_system").map(|s| serde_json::to_value(s).unwrap()).unwrap_or(Value::Null),
        }
    })
}

pub fn get_behavior_schema_description() -> serde_json::Value {
    serde_json::json!({
        "title": "CraftBehavior",
        "description": "Runtime composition primitive. Agents interact via Action, not Behavior directly. Behaviors are authored in scene.json and triggered by tick or signal.",
        "primitives": [
            {
                "kind": "state_machine",
                "fields": {
                    "initial": { "type": "string", "required": true, "description": "Initial state name" },
                    "states": { "type": "object<string, StateDef>", "required": true, "description": "Map of state name to definition" }
                },
                "state_def": {
                    "on_enter": { "type": "Action[]", "required": false, "description": "Actions emitted on entering this state" },
                    "on_tick": { "type": "Action[]", "required": false, "description": "Actions emitted each tick while in this state" },
                    "transitions": { "type": "Transition[]", "required": false, "description": "Conditional transitions to other states" }
                },
                "transition": {
                    "to": { "type": "string", "required": true, "description": "Destination state name" },
                    "when": { "type": "Expression", "required": true, "description": "Expression evaluated against current scene state" }
                }
            },
            {
                "kind": "on_tick",
                "fields": {
                    "actions": { "type": "Action[]", "required": true, "description": "Actions emitted each tick" }
                }
            },
            {
                "kind": "on_signal",
                "fields": {
                    "signal": { "type": "string", "required": true, "description": "Signal name to subscribe to" },
                    "actions": { "type": "Action[]", "required": true, "description": "Actions emitted when the signal fires" }
                }
            }
        ],
        "expression": {
            "ops": ["ref", "eq", "neq", "lt", "gt", "add", "sub"],
            "literals": ["string", "number", "bool", "null"],
            "ref": "Bare-string reference like 'self.foo.bar' or explicit {\"ref\": \"...\"}; targets: self, parent, <node-id>"
        }
    })
}

pub fn get_target_schema() -> Schema {
    let mut settings = schemars::r#gen::SchemaSettings::openapi3();
    settings.option_add_null_type = false;
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<Target>();
    serde_json::from_value(serde_json::to_value(schema).expect("serialize schema"))
        .expect("deserialize schema")
}

pub fn is_valid_verb(verb: &str) -> bool {
    ACTION_VERBS.contains(&verb)
}

pub fn generate_typescript_types() -> String {
    let mut out = String::new();
    out.push_str("// AUTO-GENERATED by craft-schema — do not edit.\n");
    out.push_str("// Source of truth: ActionParams enum in crates/craft-schema/src/lib.rs\n");
    out.push_str("// Regenerate by running `cargo run -p craft-schema --bin gen-sdk`.\n\n");

    out.push_str("export type SceneKind = \"scene\";\n\n");

    out.push_str("export type Target =\n");
    out.push_str("  | { kind: \"self\" }\n");
    out.push_str("  | { kind: \"node\"; id: string }\n");
    out.push_str("  | { kind: \"parent\" }\n");
    out.push_str("  | { kind: \"children\"; filter?: string };\n\n");

    out.push_str("export type Expression =\n");
    out.push_str("  | { ref: string }\n");
    out.push_str("  | { eq: Expression[] }\n");
    out.push_str("  | { neq: Expression[] }\n");
    out.push_str("  | { lt: Expression[] }\n");
    out.push_str("  | { gt: Expression[] }\n");
    out.push_str("  | { add: Expression[] }\n");
    out.push_str("  | { sub: Expression[] }\n");
    out.push_str("  | string | number | boolean | null;\n\n");

    out.push_str("export type LogLevel = \"debug\" | \"info\" | \"warning\" | \"error\";\n\n");

    out.push_str("export type SetStateAction = {\n");
    out.push_str("  kind: \"set_state\";\n");
    out.push_str("  target: Target;\n");
    out.push_str("  key: string;\n");
    out.push_str("  value: unknown;\n");
    out.push_str("};\n\n");

    out.push_str("export type EmitAction = {\n");
    out.push_str("  kind: \"emit\";\n");
    out.push_str("  signal: string;\n");
    out.push_str("  args?: Record<string, unknown>;\n");
    out.push_str("};\n\n");

    out.push_str("export type DestroyAction = {\n");
    out.push_str("  kind: \"destroy\";\n");
    out.push_str("  target: Target;\n");
    out.push_str("};\n\n");

    out.push_str("export type SpawnAction = {\n");
    out.push_str("  kind: \"spawn\";\n");
    out.push_str("  type: string;\n");
    out.push_str("  parent: Target;\n");
    out.push_str("  components?: Record<string, unknown>;\n");
    out.push_str("};\n\n");

    out.push_str("export type IfAction = {\n");
    out.push_str("  kind: \"if\";\n");
    out.push_str("  cond: Expression;\n");
    out.push_str("  then: Action[];\n");
    out.push_str("  else?: Action[];\n");
    out.push_str("};\n\n");

    out.push_str("export type MoveAction = {\n");
    out.push_str("  kind: \"move\";\n");
    out.push_str("  target: Target;\n");
    out.push_str("  key: string;\n");
    out.push_str("  by: unknown;\n");
    out.push_str("};\n\n");

    out.push_str("export type AnimateAction = {\n");
    out.push_str("  kind: \"animate\";\n");
    out.push_str("  target: Target;\n");
    out.push_str("  key: string;\n");
    out.push_str("  to: unknown;\n");
    out.push_str("  duration: number;\n");
    out.push_str("};\n\n");

    out.push_str("export type LogAction = {\n");
    out.push_str("  kind: \"log\";\n");
    out.push_str("  level: LogLevel;\n");
    out.push_str("  message: string;\n");
    out.push_str("  fields?: Record<string, unknown>;\n");
    out.push_str("};\n\n");

    out.push_str("export type CallSystemAction = {\n");
    out.push_str("  kind: \"call_system\";\n");
    out.push_str("  system: string;\n");
    out.push_str("  args?: Record<string, unknown>;\n");
    out.push_str("  result_in?: { key: string; on: Target };\n");
    out.push_str("};\n\n");

    out.push_str("export type Action =\n");
    out.push_str("  | SetStateAction\n");
    out.push_str("  | EmitAction\n");
    out.push_str("  | DestroyAction\n");
    out.push_str("  | SpawnAction\n");
    out.push_str("  | IfAction\n");
    out.push_str("  | MoveAction\n");
    out.push_str("  | AnimateAction\n");
    out.push_str("  | LogAction\n");
    out.push_str("  | CallSystemAction;\n\n");

    out.push_str("export const ACTION_VERBS = [\n");
    for v in ACTION_VERBS {
        out.push_str(&format!("  \"{v}\",\n"));
    }
    out.push_str("] as const;\n\n");

    out.push_str("export type ActionVerb = typeof ACTION_VERBS[number];\n");

    out
}

pub fn typescript_sdk_types() -> &'static str {
    use std::sync::OnceLock;
    static CACHE: OnceLock<String> = OnceLock::new();
    CACHE.get_or_init(generate_typescript_types).as_str()
}

#[allow(dead_code)]
fn _unused_check_schema_object(s: &SchemaObject) {
    let _ = s;
}

/// Returns the contents of `lua_engine_stub.lua` as a String, prefixed with
/// the current `SCHEMA_VERSION` for cache-busting by the editor.
pub fn lua_engine_stub() -> String {
    format!(
        "-- schema-version: {}\n-- AUTO-GENERATED by craft-editor. Do not edit.\n\n{}",
        SCHEMA_VERSION,
        include_str!("lua_engine_stub.lua")
    )
}

#[cfg(test)]
mod stub_tests {
    use super::*;

    #[test]
    fn stub_starts_with_schema_version_header() {
        let s = lua_engine_stub();
        assert!(s.starts_with(&format!("-- schema-version: {SCHEMA_VERSION}\n")));
        assert!(s.contains("--- @class Engine"));
        assert!(s.contains("--- @class Node"));
        assert!(s.contains("--- @class Vec2"));
        assert!(s.contains("--- @class SignalBus"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_verb_list_has_nine_verbs() {
        assert_eq!(ACTION_VERBS.len(), 9);
    }

    #[test]
    fn each_verb_has_a_description() {
        let descs = action_verb_descriptions();
        for verb in ACTION_VERBS {
            assert!(
                descs.contains_key(verb),
                "missing description for verb {verb}"
            );
        }
    }

    #[test]
    fn is_valid_verb_recognises_all_nine() {
        for verb in ACTION_VERBS {
            assert!(is_valid_verb(verb));
        }
        assert!(!is_valid_verb("bogus"));
    }

    #[test]
    fn get_action_schema_returns_schema_for_each_verb() {
        for verb in ACTION_VERBS {
            let schema = get_action_schema(verb);
            assert!(schema.is_some(), "no schema for {verb}");
        }
    }

    #[test]
    fn get_action_schema_returns_none_for_unknown_verb() {
        assert!(get_action_schema("nope").is_none());
    }

    #[test]
    fn set_state_schema_has_required_fields() {
        let schema = get_action_schema("set_state").expect("schema");
        let v: Value = serde_json::to_value(&schema).unwrap();
        let required = v["required"].as_array().expect("required array");
        let names: Vec<&str> = required.iter().filter_map(|r| r.as_str()).collect();
        for f in ["target", "key", "value"] {
            assert!(names.contains(&f), "set_state missing required field: {f}");
        }
    }

    #[test]
    fn call_system_schema_has_optional_result_in() {
        let schema = get_action_schema("call_system").expect("schema");
        let v: Value = serde_json::to_value(&schema).unwrap();
        let properties = v["properties"].as_object().expect("properties");
        assert!(properties.contains_key("result_in"));
    }

    #[test]
    fn get_full_schema_contains_per_verb_schemas() {
        let full = get_full_schema();
        let per_verb = full["per_verb_schemas"]
            .as_object()
            .expect("per_verb_schemas");
        for verb in ACTION_VERBS {
            let entry = per_verb
                .get(*verb)
                .unwrap_or_else(|| panic!("per_verb_schemas missing {verb}"));
            assert!(entry.is_object(), "{verb} is not an object: {entry:?}");
        }
    }

    #[test]
    fn typescript_sdk_types_contains_all_verbs() {
        let ts = typescript_sdk_types();
        for verb in ACTION_VERBS {
            assert!(
                ts.contains(&format!("\"{verb}\"")),
                "TS types missing verb {verb}"
            );
        }
    }

    #[test]
    fn typescript_sdk_types_starts_with_auto_generated_marker() {
        let ts = typescript_sdk_types();
        assert!(ts.starts_with("// AUTO-GENERATED by craft-schema"));
    }

    #[test]
    fn ts_sdk_output_parses_as_module() {
        let ts = typescript_sdk_types();
        let mut depth: i32 = 0;
        for c in ts.chars() {
            if c == '{' {
                depth += 1;
            }
            if c == '}' {
                depth -= 1;
            }
        }
        assert_eq!(depth, 0, "TS types must have balanced braces");
        assert!(ts.contains("export type Action"));
        assert!(ts.contains("export const ACTION_VERBS"));
    }

    #[test]
    fn typescript_sdk_types_defines_all_verb_aliases() {
        let ts = typescript_sdk_types();
        for alias in [
            "SetStateAction",
            "EmitAction",
            "DestroyAction",
            "SpawnAction",
            "IfAction",
            "MoveAction",
            "AnimateAction",
            "LogAction",
            "CallSystemAction",
        ] {
            assert!(ts.contains(alias), "TS types missing action alias {alias}");
        }
    }

    #[test]
    fn get_behavior_schema_description_is_nonempty() {
        let v = get_behavior_schema_description();
        assert!(v.is_object());
        assert_eq!(v["title"], "CraftBehavior");
        assert!(v["primitives"].is_array());
        let primitives: Vec<&str> = v["primitives"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.get("kind").and_then(|k| k.as_str()))
            .collect();
        assert!(primitives.contains(&"state_machine"));
        assert!(primitives.contains(&"on_tick"));
        assert!(primitives.contains(&"on_signal"));
    }

    #[test]
    fn get_target_schema_returns_schema() {
        let _ = get_target_schema();
    }
}
