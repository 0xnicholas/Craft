use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use craft_kernel::behavior::Action;
use craft_kernel::scene::Component;
use craft_kernel::{AutoFix, Engine, EngineConfig, EngineError, Scene, ValidationError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub id: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    pub fn internal(message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Value,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn err(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

pub fn parse_request(input: &str) -> Result<JsonRpcRequest, JsonRpcError> {
    let v: Value =
        serde_json::from_str(input).map_err(|e| JsonRpcError::parse_error(e.to_string()))?;
    let req: JsonRpcRequest =
        serde_json::from_value(v).map_err(|e| JsonRpcError::invalid_request(e.to_string()))?;
    if req.jsonrpc != "2.0" {
        return Err(JsonRpcError::invalid_request(format!(
            "jsonrpc must be \"2.0\", got {:?}",
            req.jsonrpc
        )));
    }
    Ok(req)
}

pub fn parse_batch(input: &str) -> Result<Vec<JsonRpcRequest>, JsonRpcError> {
    let v: Value =
        serde_json::from_str(input).map_err(|e| JsonRpcError::parse_error(e.to_string()))?;
    match v {
        Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let req: JsonRpcRequest = serde_json::from_value(item)
                    .map_err(|e| JsonRpcError::invalid_request(e.to_string()))?;
                out.push(req);
            }
            Ok(out)
        }
        _ => Ok(vec![parse_request(input)?]),
    }
}

pub fn serialize_response(resp: &JsonRpcResponse) -> String {
    serde_json::to_string(resp).expect("JsonRpcResponse serialization is infallible")
}

pub fn serialize_responses(responses: &[JsonRpcResponse]) -> String {
    serde_json::to_string(responses).expect("JsonRpcResponse array serialization is infallible")
}

pub fn engine_error_to_rpc(err: EngineError) -> (i32, String, Option<Value>) {
    match err {
        EngineError::Parse(p) => {
            let data = parse_error_to_data(&p);
            (-32000, p.message, Some(data))
        }
        EngineError::Validation { file, errors } => {
            let data = validation_errors_to_data(&file, &errors);
            (-32001, "validation failed".to_string(), Some(data))
        }
        EngineError::Io(io) => {
            let data = io_error_to_data(&io);
            (-32002, format!("io error: {}", io.message), Some(data))
        }
        EngineError::Internal(s) => (-32603, s, None),
    }
}

fn parse_error_to_data(p: &craft_kernel::ParseError) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("file".to_string(), Value::String(p.file.clone()));
    map.insert(
        "line".to_string(),
        p.line
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null),
    );
    map.insert(
        "column".to_string(),
        p.column
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null),
    );
    if let Some(s) = &p.snippet {
        map.insert("snippet".to_string(), Value::String(s.clone()));
    }
    Value::Object(map)
}

fn validation_errors_to_data(file: &str, errors: &[ValidationError]) -> Value {
    Value::Object(serde_json::Map::from_iter([
        ("file".to_string(), Value::String(file.to_string())),
        (
            "errors".to_string(),
            Value::Array(
                errors
                    .iter()
                    .map(|e| {
                        Value::Object(serde_json::Map::from_iter([
                            ("file".to_string(), Value::String(e.file.clone())),
                            ("json_path".to_string(), Value::String(e.json_path.clone())),
                            ("message".to_string(), Value::String(e.message.clone())),
                            (
                                "expected_type".to_string(),
                                Value::String(e.expected_type.clone()),
                            ),
                            (
                                "actual_value".to_string(),
                                e.actual_value.clone().unwrap_or(Value::Null),
                            ),
                            (
                                "suggestion".to_string(),
                                e.suggestion
                                    .as_ref()
                                    .map(|s| Value::String(s.clone()))
                                    .unwrap_or(Value::Null),
                            ),
                            (
                                "auto_fixable".to_string(),
                                Value::String(auto_fix_to_str(&e.auto_fixable).to_string()),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        ),
    ]))
}

fn auto_fix_to_str(f: &AutoFix) -> &'static str {
    match f {
        AutoFix::Safe => "safe",
        AutoFix::Suggested => "suggested",
        AutoFix::NeedsReview => "needs_review",
    }
}

fn io_error_to_data(io: &craft_kernel::IoError) -> Value {
    Value::Object(serde_json::Map::from_iter([
        ("file".to_string(), Value::String(io.file.clone())),
        (
            "kind".to_string(),
            Value::String(format!("{:?}", io.kind).to_lowercase()),
        ),
        ("message".to_string(), Value::String(io.message.clone())),
    ]))
}

pub struct Bridge {
    engine: Engine,
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Bridge {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    pub fn with_config(config: EngineConfig) -> Self {
        Self {
            engine: Engine::with_config(config),
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    pub fn dispatch(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        let method = request.method.clone();
        let result = self.call_method(&method, request.params);
        match result {
            Ok(value) => JsonRpcResponse::ok(id, value),
            Err(rpc_err) => JsonRpcResponse::err(id, rpc_err),
        }
    }

    pub fn dispatch_str(&mut self, input: &str) -> String {
        match parse_request(input) {
            Ok(req) => serialize_response(&self.dispatch(req)),
            Err(e) => serialize_response(&JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(e),
                id: Value::Null,
            }),
        }
    }

    pub fn dispatch_batch_str(&mut self, input: &str) -> Option<String> {
        match parse_batch(input) {
            Ok(reqs) => {
                let responses: Vec<JsonRpcResponse> =
                    reqs.into_iter().map(|r| self.dispatch(r)).collect();
                if responses.is_empty() {
                    None
                } else {
                    Some(serialize_responses(&responses))
                }
            }
            Err(e) => Some(serialize_response(&JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(e),
                id: Value::Null,
            })),
        }
    }

    fn call_method(&mut self, method: &str, params: Value) -> Result<Value, JsonRpcError> {
        match method {
            "engine.start" => self.rpc_start(params),
            "engine.loadScene" => self.rpc_load_scene(params),
            "engine.tick" => self.rpc_tick(),
            "engine.lint" => self.rpc_lint(params),
            "engine.dryRun" => self.rpc_dry_run(params),
            "engine.explain" => self.rpc_explain(params),
            "engine.getSchema" => Ok(self.rpc_get_schema()),
            "engine.getActionSchema" => self.rpc_get_action_schema(params),
            "engine.getComponent" => self.rpc_get_component(params),
            "engine.setComponent" => self.rpc_set_component(params),
            "engine.listSystems" => Ok(self.rpc_list_systems()),
            "engine.listNodes" => Ok(self.rpc_list_nodes()),
            "engine.applyHotReload" => self.rpc_apply_hot_reload(params),
            "engine.emit" => self.rpc_emit(params),
            "engine.stateHash" => Ok(self.rpc_state_hash()),
            "engine.version" => Ok(Value::String(env!("CARGO_PKG_VERSION").to_string())),
            _ => Err(JsonRpcError::method_not_found(method)),
        }
    }

    fn rpc_start(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let path = params
            .get("scene_path")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing scene_path"))?;
        let scene_json = std::fs::read_to_string(path)
            .map_err(|e| JsonRpcError::internal(format!("failed to read scene file: {e}"), None))?;
        let scene: Scene = serde_json::from_str(&scene_json)
            .map_err(|e| JsonRpcError::invalid_params(format!("invalid scene JSON: {e}")))?;
        let _ = scene;
        Ok(Value::Null)
    }

    fn rpc_load_scene(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let scene_value = params
            .get("scene")
            .cloned()
            .ok_or_else(|| JsonRpcError::invalid_params("missing scene"))?;
        let scene: Scene = serde_json::from_value(scene_value)
            .map_err(|e| JsonRpcError::invalid_params(format!("invalid scene JSON: {e}")))?;
        self.engine.load_scene(scene);
        Ok(Value::Null)
    }

    fn rpc_tick(&mut self) -> Result<Value, JsonRpcError> {
        self.engine.tick();
        let tick = self.engine.tick;
        Ok(serde_json::json!({ "tick": tick }))
    }

    fn rpc_lint(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let _ = params;
        let warnings = self.engine.lint();
        Ok(serde_json::to_value(&warnings).expect("LintWarning serialization is infallible"))
    }

    fn rpc_dry_run(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let node_id = params
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing node_id"))?
            .to_string();
        let actions: Vec<Action> = serde_json::from_value(
            params
                .get("actions")
                .cloned()
                .ok_or_else(|| JsonRpcError::invalid_params("missing actions"))?,
        )
        .map_err(|e| JsonRpcError::invalid_params(format!("invalid actions: {e}")))?;
        let scene = self
            .engine
            .scene
            .as_ref()
            .ok_or_else(|| JsonRpcError::invalid_request("no scene loaded"))?
            .clone();
        let registry = self.engine.nodes.clone();
        match craft_kernel::evaluator::evaluate_dry_run(&scene, &registry, &node_id, &actions) {
            Ok(changes) => {
                let diff = serde_json::to_value(&changes)
                    .map_err(|e| JsonRpcError::internal(format!("diff serialize: {e}"), None))?;
                Ok(serde_json::json!({ "ok": true, "diff": diff }))
            }
            Err(e) => {
                let (code, msg, data) = engine_error_to_rpc(e);
                Err(JsonRpcError {
                    code,
                    message: msg,
                    data,
                })
            }
        }
    }

    fn rpc_explain(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let node_id = params
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing node_id"))?;
        let scene = self
            .engine
            .scene
            .as_ref()
            .ok_or_else(|| JsonRpcError::invalid_request("no scene loaded"))?;
        let node = scene
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| JsonRpcError::invalid_params(format!("node not found: {node_id}")))?;

        let components: HashMap<&str, &Component> = node
            .components
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        let active_state = node.active_state.as_deref();

        let mut signal_subscriptions: Vec<String> = Vec::new();
        for b in &node.behaviors {
            if let craft_kernel::Behavior::OnSignal { signal, .. } = b {
                signal_subscriptions.push(signal.clone());
            }
        }

        Ok(serde_json::json!({
            "id": node.id,
            "type": node.type_name,
            "parent": node.parent,
            "components": components.keys().copied().collect::<Vec<_>>(),
            "active_state": active_state,
            "signal_subscriptions": signal_subscriptions,
        }))
    }

    fn rpc_get_schema(&self) -> Value {
        let nodes: Vec<Value> = self
            .engine
            .nodes
            .type_names()
            .map(|n| Value::String(n.to_string()))
            .collect();
        let systems: Vec<Value> = self
            .engine
            .list_systems()
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "phase": format!("{:?}", s.phase).to_lowercase(),
                })
            })
            .collect();
        let verbs = vec![
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
        serde_json::json!({
            "version": "0.1.0",
            "scene_kind": craft_kernel::SCENE_KIND,
            "node_types": nodes,
            "systems": systems,
            "verbs": verbs,
            "primitives": ["lint", "dryRun", "explain", "diff"],
            "schema_source": "craft-schema",
        })
    }

    fn rpc_get_action_schema(&self, params: Value) -> Result<Value, JsonRpcError> {
        let verb = params
            .get("verb")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing verb"))?;
        let schema = craft_schema::get_action_schema(verb);
        match schema {
            Some(s) => Ok(serde_json::json!({
                "verb": verb,
                "schema": serde_json::to_value(s).map_err(|e| {
                    JsonRpcError::internal(format!("schema serialization: {e}"), None)
                })?,
                "description": craft_schema::action_verb_descriptions()
                    .get(verb)
                    .copied()
                    .unwrap_or(""),
            })),
            None => Err(JsonRpcError::invalid_params(format!(
                "unknown verb: {verb}; valid verbs: {:?}",
                craft_schema::ACTION_VERBS
            ))),
        }
    }

    fn rpc_get_component(&self, params: Value) -> Result<Value, JsonRpcError> {
        let node_id = params
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing node_id"))?;
        let key = params
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing key"))?;
        let scene = self
            .engine
            .scene
            .as_ref()
            .ok_or_else(|| JsonRpcError::invalid_request("no scene loaded"))?;
        let node = scene
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .ok_or_else(|| JsonRpcError::invalid_params(format!("node not found: {node_id}")))?;
        let component = node
            .components
            .get(key)
            .ok_or_else(|| JsonRpcError::invalid_params(format!("component not found: {key}")))?;
        let v = craft_kernel::evaluator::component_value_to_json(&component.value);
        Ok(serde_json::json!({
            "node_id": node_id,
            "key": key,
            "value": v,
        }))
    }

    fn rpc_set_component(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let node_id = params
            .get("node_id")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing node_id"))?
            .to_string();
        let key = params
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing key"))?
            .to_string();
        let value = params
            .get("value")
            .cloned()
            .ok_or_else(|| JsonRpcError::invalid_params("missing value"))?;
        let component_value = craft_kernel::evaluator::json_to_component_value(value)
            .map_err(|e| JsonRpcError::invalid_params(format!("invalid value: {e}")))?;
        let scene = self
            .engine
            .scene
            .as_mut()
            .ok_or_else(|| JsonRpcError::invalid_request("no scene loaded"))?;
        let node = scene
            .nodes
            .iter_mut()
            .find(|n| n.id == node_id)
            .ok_or_else(|| JsonRpcError::invalid_params(format!("node not found: {node_id}")))?;
        node.components.insert(
            key,
            Component {
                value: component_value,
                kind: Default::default(),
            },
        );
        Ok(Value::Null)
    }

    fn rpc_list_systems(&self) -> Value {
        serde_json::to_value(self.engine.list_systems())
            .expect("SystemInfo serialization is infallible")
    }

    fn rpc_list_nodes(&self) -> Value {
        match &self.engine.scene {
            Some(scene) => {
                let nodes: Vec<Value> = scene
                    .nodes
                    .iter()
                    .map(|n| {
                        serde_json::json!({
                            "id": n.id,
                            "type": n.type_name,
                            "parent": n.parent,
                        })
                    })
                    .collect();
                Value::Array(nodes)
            }
            None => Value::Array(vec![]),
        }
    }

    fn rpc_apply_hot_reload(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let scene_value = params
            .get("scene")
            .cloned()
            .ok_or_else(|| JsonRpcError::invalid_params("missing scene"))?;
        let new_scene: Scene = serde_json::from_value(scene_value)
            .map_err(|e| JsonRpcError::invalid_params(format!("invalid scene JSON: {e}")))?;
        match self.engine.apply_hot_reload(&new_scene) {
            Ok(result) => {
                let v: Value = serde_json::to_value(&result)
                    .expect("HotReloadResult serialization is infallible");
                let applied = result.applied;
                let affected = result.affected_node_ids;
                Ok(serde_json::json!({
                    "applied": applied,
                    "affected": affected,
                    "diff": v,
                }))
            }
            Err(e) => {
                let (code, msg, data) = engine_error_to_rpc(e);
                Err(JsonRpcError {
                    code,
                    message: msg,
                    data,
                })
            }
        }
    }

    fn rpc_emit(&mut self, params: Value) -> Result<Value, JsonRpcError> {
        let signal_name = params
            .get("signal")
            .and_then(Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("missing signal"))?
            .to_string();
        let payload = params.get("payload").cloned().unwrap_or(Value::Null);
        let id = self.engine.bus.declare(&signal_name);
        self.engine.bus.emit(id, payload);
        Ok(serde_json::json!({ "signal_id": id.raw() }))
    }

    fn rpc_state_hash(&self) -> Value {
        serde_json::json!({ "hash": self.engine.state_hash() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_request() {
        let input = r#"{"jsonrpc":"2.0","method":"engine.tick","id":1}"#;
        let req = parse_request(input).expect("parse");
        assert_eq!(req.method, "engine.tick");
        assert_eq!(req.id, Value::Number(1.into()));
    }

    #[test]
    fn rejects_wrong_jsonrpc_version() {
        let input = r#"{"jsonrpc":"1.0","method":"x","id":1}"#;
        let err = parse_request(input).expect_err("must fail");
        assert_eq!(err.code, -32600);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = parse_request("not json").expect_err("must fail");
        assert_eq!(err.code, -32700);
    }

    #[test]
    fn dispatch_returns_method_not_found() {
        let mut bridge = Bridge::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "engine.bogus".to_string(),
            params: Value::Null,
            id: Value::Number(1.into()),
        };
        let resp = bridge.dispatch(req);
        assert!(resp.result.is_none());
        let err = resp.error.expect("error");
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn dispatch_engine_tick_advances() {
        let mut bridge = Bridge::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "engine.tick".to_string(),
            params: Value::Null,
            id: Value::Number(1.into()),
        };
        let resp = bridge.dispatch(req);
        let result = resp.result.expect("result");
        assert_eq!(result["tick"], Value::Number(1.into()));
    }

    #[test]
    fn dispatch_str_handles_parse_error() {
        let mut bridge = Bridge::new();
        let resp = bridge.dispatch_str("not json");
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert_eq!(v["error"]["code"], Value::Number((-32700).into()));
    }

    #[test]
    fn batch_request_returns_array() {
        let mut bridge = Bridge::new();
        let input = r#"[
            {"jsonrpc":"2.0","method":"engine.tick","id":1},
            {"jsonrpc":"2.0","method":"engine.stateHash","id":2}
        ]"#;
        let resp = bridge.dispatch_batch_str(input).expect("response");
        let v: Value = serde_json::from_str(&resp).unwrap();
        assert!(v.is_array());
        assert_eq!(v.as_array().unwrap().len(), 2);
    }

    #[test]
    fn empty_batch_returns_none() {
        let mut bridge = Bridge::new();
        let resp = bridge.dispatch_batch_str("[]");
        assert!(resp.is_none());
    }

    #[test]
    fn engine_version_returns_package_version() {
        let mut bridge = Bridge::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "engine.version".to_string(),
            params: Value::Null,
            id: Value::Number(1.into()),
        };
        let resp = bridge.dispatch(req);
        let result = resp.result.expect("result");
        assert!(result.is_string());
    }

    #[test]
    fn list_systems_returns_array() {
        let mut bridge = Bridge::new();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "engine.listSystems".to_string(),
            params: Value::Null,
            id: Value::Number(1.into()),
        };
        let resp = bridge.dispatch(req);
        let result = resp.result.expect("result");
        assert!(result.is_array());
    }
}
