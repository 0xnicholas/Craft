use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::scene::ComponentValue;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Target {
    #[serde(rename = "self")]
    This,
    #[serde(rename = "node")]
    Node { id: String },
    #[serde(rename = "parent")]
    Parent,
    #[serde(rename = "children")]
    Children { filter: Option<String> },
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(deserializer)?;
        match v {
            serde_json::Value::String(s) => match s.as_str() {
                "self" => Ok(Self::This),
                "parent" => Ok(Self::Parent),
                "children" => Ok(Self::Children { filter: None }),
                other => Ok(Self::Node {
                    id: other.to_string(),
                }),
            },
            serde_json::Value::Object(map) => {
                if let Some(kind) = map.get("kind").and_then(Value::as_str) {
                    match kind {
                        "self" => Ok(Self::This),
                        "parent" => Ok(Self::Parent),
                        "children" => Ok(Self::Children {
                            filter: map.get("filter").and_then(|f| f.as_str().map(String::from)),
                        }),
                        "node" => {
                            let id = map
                                .get("id")
                                .and_then(|f| f.as_str())
                                .ok_or_else(|| serde::de::Error::custom("node target requires id"))?
                                .to_string();
                            Ok(Self::Node { id })
                        }
                        other => Err(serde::de::Error::custom(format!(
                            "unknown target kind: {other}"
                        ))),
                    }
                } else {
                    Err(serde::de::Error::custom(
                        "target object must have a `kind` field",
                    ))
                }
            }
            _ => Err(serde::de::Error::custom(
                "target must be a string or object",
            )),
        }
    }
}

impl Target {
    pub fn label(&self) -> String {
        match self {
            Self::This => "self".to_string(),
            Self::Node { id } => format!("node:{id}"),
            Self::Parent => "parent".to_string(),
            Self::Children { filter } => match filter {
                Some(f) => format!("children:{f}"),
                None => "children".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(untagged)]
pub enum Expression {
    Bare(String),
    Ref { r#ref: String },
    Eq { eq: Vec<Expression> },
    Neq { neq: Vec<Expression> },
    Lt { lt: Vec<Expression> },
    Gt { gt: Vec<Expression> },
    Add { add: Vec<Expression> },
    Sub { sub: Vec<Expression> },
    Literal(Value),
}

impl Expression {
    pub fn from_value(v: Value) -> Self {
        match v {
            Value::Null => Self::Literal(Value::Null),
            Value::Bool(b) => Self::Literal(Value::Bool(b)),
            Value::Number(n) => Self::Literal(Value::Number(n)),
            Value::String(s) => {
                if matches!(s.as_str(), "self" | "none" | "true" | "false") {
                    Self::Bare(s)
                } else if s.contains('.') && !s.starts_with('"') {
                    Self::Ref { r#ref: s }
                } else {
                    Self::Literal(Value::String(s))
                }
            }
            Value::Array(_) => Self::Literal(v),
            Value::Object(map) => {
                if let Some(Value::String(s)) = map.get("ref").cloned() {
                    return Self::Ref { r#ref: s };
                }
                for (op_key, op_name) in [
                    ("eq", "eq"),
                    ("neq", "neq"),
                    ("lt", "lt"),
                    ("gt", "gt"),
                    ("add", "add"),
                    ("sub", "sub"),
                ] {
                    if let Some(Value::Array(arr)) = map.get(op_key).cloned() {
                        let args = arr.into_iter().map(Self::from_value).collect::<Vec<_>>();
                        return match op_name {
                            "eq" => Self::Eq { eq: args },
                            "neq" => Self::Neq { neq: args },
                            "lt" => Self::Lt { lt: args },
                            "gt" => Self::Gt { gt: args },
                            "add" => Self::Add { add: args },
                            "sub" => Self::Sub { sub: args },
                            _ => unreachable!(),
                        };
                    }
                }
                Self::Literal(Value::Object(map))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    #[serde(rename = "set_state")]
    SetState {
        target: Target,
        key: String,
        value: Value,
    },
    #[serde(rename = "emit")]
    Emit {
        signal: String,
        #[serde(default)]
        args: BTreeMap<String, Value>,
    },
    #[serde(rename = "destroy")]
    Destroy { target: Target },
    #[serde(rename = "spawn")]
    Spawn {
        #[serde(rename = "type")]
        node_type: String,
        parent: Target,
        #[serde(default)]
        components: BTreeMap<String, Value>,
        #[serde(default)]
        behaviors: Vec<Behavior>,
    },
    #[serde(rename = "if")]
    If {
        cond: Expression,
        then: Vec<Action>,
        #[serde(rename = "else", default)]
        else_: Vec<Action>,
    },
    #[serde(rename = "move")]
    Move {
        target: Target,
        key: String,
        by: Value,
    },
    #[serde(rename = "animate")]
    Animate {
        target: Target,
        key: String,
        to: Value,
        duration: u32,
    },
    #[serde(rename = "log")]
    Log {
        level: LogLevel,
        message: String,
        #[serde(default)]
        fields: BTreeMap<String, Value>,
    },
    #[serde(rename = "call_system")]
    CallSystem {
        system: String,
        #[serde(default)]
        args: BTreeMap<String, Value>,
        #[serde(default)]
        result_in: Option<ResultTarget>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultTarget {
    pub key: String,
    pub on: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Behavior {
    #[serde(rename = "state_machine")]
    StateMachine {
        initial: String,
        states: BTreeMap<String, StateDef>,
    },
    #[serde(rename = "on_tick")]
    OnTick { actions: Vec<Action> },
    #[serde(rename = "on_signal")]
    OnSignal {
        signal: String,
        actions: Vec<Action>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StateDef {
    #[serde(default)]
    pub on_enter: Vec<Action>,
    #[serde(default)]
    pub on_tick: Vec<Action>,
    #[serde(default)]
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transition {
    pub to: String,
    pub when: Expression,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionCommand {
    #[serde(rename = "set_state")]
    SetState {
        target: Target,
        key: String,
        value: ComponentValue,
    },
    #[serde(rename = "emit")]
    Emit {
        signal: String,
        args: BTreeMap<String, Value>,
    },
    #[serde(rename = "destroy")]
    Destroy { target: Target },
    #[serde(rename = "spawn")]
    Spawn {
        node_type: String,
        parent: Target,
        components: BTreeMap<String, ComponentValue>,
        behaviors: Vec<Behavior>,
    },
    #[serde(rename = "animate")]
    StartAnimation {
        target: Target,
        key: String,
        to: ComponentValue,
        remaining: u32,
    },
    #[serde(rename = "log")]
    Log {
        level: LogLevel,
        message: String,
        fields: BTreeMap<String, Value>,
    },
    #[serde(rename = "call_system")]
    CallSystem {
        system: String,
        args: BTreeMap<String, Value>,
        result_target: Option<ResultTarget>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn expression_bare_string_is_ref() {
        let e = Expression::from_value(json!("self.position"));
        assert!(matches!(e, Expression::Ref { .. }));
    }

    #[test]
    fn expression_bare_self_reserved() {
        let e = Expression::from_value(json!("self"));
        assert!(matches!(e, Expression::Bare(ref s) if s == "self"));
    }

    #[test]
    fn expression_eq_form() {
        let e = Expression::from_value(json!({"eq": [{"ref": "a"}, 5]}));
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("\"eq\""));
    }

    #[test]
    fn expression_add_form() {
        let e = Expression::from_value(json!({"add": [1, 2]}));
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("\"add\""));
    }

    #[test]
    fn action_set_state_deserializes() {
        let a: Action = serde_json::from_value(json!({
            "kind": "set_state",
            "target": "self",
            "key": "health",
            "value": 50
        }))
        .unwrap();
        assert!(matches!(
            a,
            Action::SetState { ref key, .. } if key == "health"
        ));
    }

    #[test]
    fn action_if_deserializes() {
        let a: Action = serde_json::from_value(json!({
            "kind": "if",
            "cond": {"lt": [{"ref": "self.hp"}, 0]},
            "then": [{"kind": "emit", "signal": "died"}],
            "else": []
        }))
        .unwrap();
        match a {
            Action::If { then, else_, .. } => {
                assert_eq!(then.len(), 1);
                assert!(else_.is_empty());
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn action_call_system_with_result_target() {
        let a: Action = serde_json::from_value(json!({
            "kind": "call_system",
            "system": "compute_damage",
            "args": {"base": 10},
            "result_in": {"key": "damage", "on": "self"}
        }))
        .unwrap();
        match a {
            Action::CallSystem { result_in, .. } => {
                let r = result_in.unwrap();
                assert_eq!(r.key, "damage");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn behavior_state_machine_deserializes() {
        let b: Behavior = serde_json::from_value(json!({
            "kind": "state_machine",
            "initial": "idle",
            "states": {
                "idle": {"on_tick": [{"kind": "log", "level": "info", "message": "idle"}]},
                "attacking": {"transitions": [{"to": "idle", "when": "self.target"}]}
            }
        }))
        .unwrap();
        match b {
            Behavior::StateMachine { initial, states } => {
                assert_eq!(initial, "idle");
                assert!(states.contains_key("attacking"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn behavior_on_signal_deserializes() {
        let b: Behavior = serde_json::from_value(json!({
            "kind": "on_signal",
            "signal": "hit",
            "actions": [{"kind": "destroy", "target": "self"}]
        }))
        .unwrap();
        assert!(matches!(b, Behavior::OnSignal { ref signal, .. } if signal == "hit"));
    }
}
