use craft_bridge::Bridge;
use craft_kernel::Scene;
use craft_kernel::craft_node;
use craft_kernel::craft_system;
use craft_kernel::serde_json::json;

craft_node!(BRPlayer, {
    components: {
        health: Int = 100,
        position: Vec2 = [0.0, 0.0],
    },
});

craft_system!(M6BridgeTestSystem, phase: Tick, {
    let _ = ctx;
});

fn scene_json() -> String {
    r#"{
        "kind": "scene",
        "name": "test",
        "nodes": [
            {
                "id": "p1",
                "type": "BRPlayer",
                "components": { "health": 100, "position": [0.0, 0.0] },
                "behaviors": [
                    {
                        "kind": "on_tick",
                        "actions": [
                            { "kind": "move", "target": "self", "key": "health", "by": 1 }
                        ]
                    }
                ]
            }
        ]
    }"#
    .to_string()
}

fn loaded_bridge() -> Bridge {
    let mut bridge = Bridge::new();
    let scene_value: serde_json::Value =
        serde_json::from_str(&scene_json()).expect("parse scene json");
    let registry = bridge.engine().nodes.clone();
    let scene = Scene::from_value(scene_value, "scene.json", &registry).expect("scene from value");
    bridge.engine_mut().load_scene(scene);
    bridge
}

fn make_request(method: &str, params: serde_json::Value, id: u32) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    })
    .to_string()
}

#[test]
fn ts_style_engine_tick_via_json_rpc() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.tick", serde_json::json!({}), 1);
    let resp_str = bridge.dispatch_str(&req);
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["tick"], 1);
    assert!(resp.get("error").is_none() || resp["error"].is_null());
}

#[test]
fn ts_style_engine_get_schema() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.getSchema", serde_json::json!({}), 2);
    let resp_str = bridge.dispatch_str(&req);
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["result"]["version"], "0.1.0");
    let verbs = resp["result"]["verbs"].as_array().expect("verbs array");
    assert!(verbs.iter().any(|v| v == "set_state"));
    assert!(verbs.iter().any(|v| v == "emit"));
    let primitives = resp["result"]["primitives"].as_array().expect("primitives");
    assert!(primitives.iter().any(|p| p == "lint"));
    assert!(primitives.iter().any(|p| p == "dryRun"));
    assert!(primitives.iter().any(|p| p == "explain"));
    assert!(primitives.iter().any(|p| p == "diff"));
}

#[test]
fn ts_style_engine_get_component() {
    let mut bridge = loaded_bridge();
    let req = make_request(
        "engine.getComponent",
        serde_json::json!({"node_id": "p1", "key": "health"}),
        3,
    );
    let resp_str = bridge.dispatch_str(&req);
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert_eq!(resp["result"]["node_id"], "p1");
    assert_eq!(resp["result"]["key"], "health");
    assert_eq!(resp["result"]["value"], 100);
}

#[test]
fn ts_style_engine_set_component() {
    let mut bridge = loaded_bridge();
    let req = make_request(
        "engine.setComponent",
        serde_json::json!({"node_id": "p1", "key": "health", "value": 75}),
        4,
    );
    let resp_str = bridge.dispatch_str(&req);
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    assert!(resp["error"].is_null() || resp.get("error").is_none());

    let get_req = make_request(
        "engine.getComponent",
        serde_json::json!({"node_id": "p1", "key": "health"}),
        5,
    );
    let get_resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&get_req)).unwrap();
    assert_eq!(get_resp["result"]["value"], 75);
}

#[test]
fn ts_style_engine_lint_returns_warnings() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.lint", serde_json::json!({}), 6);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    let warnings = resp["result"].as_array().expect("warnings array");
    assert!(
        warnings.iter().any(|w| w["code"] == "unused_component"),
        "should report unused component warnings"
    );
}

#[test]
fn ts_style_engine_explain_returns_node_summary() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.explain", serde_json::json!({"node_id": "p1"}), 7);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert_eq!(resp["result"]["id"], "p1");
    assert_eq!(resp["result"]["type"], "BRPlayer");
    let components = resp["result"]["components"].as_array().expect("components");
    assert!(components.iter().any(|c| c == "health"));
    assert!(components.iter().any(|c| c == "position"));
}

#[test]
fn ts_style_engine_dry_run() {
    let mut bridge = loaded_bridge();
    let req = make_request(
        "engine.dryRun",
        serde_json::json!({
            "node_id": "p1",
            "actions": [
                { "kind": "set_state", "target": "self", "key": "health", "value": 50 }
            ]
        }),
        8,
    );
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert_eq!(resp["result"]["ok"], true);
    let diff = resp["result"]["diff"].as_array().expect("diff array");
    assert!(
        !diff.is_empty(),
        "dry-run should report the would-apply delta"
    );
    let _ = diff
        .iter()
        .find(|c| c.get("key").and_then(serde_json::Value::as_str) == Some("health"))
        .unwrap_or_else(|| panic!("expected health diff in {diff:?}"));
    assert_eq!(
        diff[0]["to"]["value"], 50,
        "dry-run should show new value 50"
    );
    let get_req = make_request(
        "engine.getComponent",
        serde_json::json!({"node_id": "p1", "key": "health"}),
        9,
    );
    let get_resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&get_req)).unwrap();
    assert_eq!(
        get_resp["result"]["value"], 100,
        "dryRun must not mutate state"
    );
}

#[test]
fn ts_style_engine_state_hash() {
    let mut bridge = loaded_bridge();
    let hash_req = make_request("engine.stateHash", serde_json::json!({}), 10);
    let hash_resp: serde_json::Value =
        serde_json::from_str(&bridge.dispatch_str(&hash_req)).unwrap();
    let h1 = hash_resp["result"]["hash"].as_u64().expect("u64");

    let tick_req = make_request("engine.tick", serde_json::json!({}), 11);
    bridge.dispatch_str(&tick_req);
    let hash_resp2: serde_json::Value =
        serde_json::from_str(&bridge.dispatch_str(&hash_req)).unwrap();
    let h2 = hash_resp2["result"]["hash"].as_u64().expect("u64");
    assert_ne!(h1, h2, "state hash must change after tick");
}

#[test]
fn ts_style_engine_list_systems() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.listSystems", serde_json::json!({}), 12);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    let systems = resp["result"].as_array().expect("systems array");
    assert!(
        !systems.is_empty(),
        "at least the inventory-registered systems should be present"
    );
}

#[test]
fn ts_style_engine_list_nodes() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.listNodes", serde_json::json!({}), 13);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    let nodes = resp["result"].as_array().expect("nodes array");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["id"], "p1");
}

#[test]
fn ts_style_engine_emit_signal() {
    let mut bridge = loaded_bridge();
    let req = make_request(
        "engine.emit",
        serde_json::json!({"signal": "user_action", "payload": {"action": "click"}}),
        14,
    );
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert!(resp["result"]["signal_id"].is_number());
}

#[test]
fn ts_style_engine_apply_hot_reload() {
    let mut bridge = loaded_bridge();
    let new_scene = json!({
        "kind": "scene",
        "name": "test",
        "nodes": [
            {
                "id": "p1",
                "type": "BRPlayer",
                "components": { "health": 999, "position": [0.0, 0.0] }
            }
        ]
    });
    let req = make_request("engine.applyHotReload", json!({"scene": new_scene}), 15);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert_eq!(resp["result"]["applied"], true);
    let affected = resp["result"]["affected"].as_array().expect("affected");
    assert!(affected.iter().any(|n| n == "p1"));
}

#[test]
fn ts_style_method_not_found() {
    let mut bridge = loaded_bridge();
    let req = make_request("engine.nope", serde_json::json!({}), 16);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert_eq!(resp["error"]["code"], -32601);
    assert!(resp["error"]["message"].as_str().unwrap().contains("nope"));
}

#[test]
fn ts_style_invalid_params() {
    let mut bridge = loaded_bridge();
    let req = make_request(
        "engine.tick",
        serde_json::json!({"unexpected": "field"}),
        17,
    );
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert!(
        resp["result"].get("tick").is_some(),
        "tick with unexpected extra params still works"
    );
}

#[test]
fn ts_style_batch_request() {
    let mut bridge = loaded_bridge();
    let batch = json!([
        {"jsonrpc": "2.0", "method": "engine.tick", "id": 1},
        {"jsonrpc": "2.0", "method": "engine.tick", "id": 2},
        {"jsonrpc": "2.0", "method": "engine.tick", "id": 3}
    ]);
    let resp_str = bridge
        .dispatch_batch_str(&batch.to_string())
        .expect("response");
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();
    let responses = resp.as_array().expect("array");
    assert_eq!(responses.len(), 3);
    for (i, r) in responses.iter().enumerate() {
        assert_eq!(r["id"], (i + 1) as u32);
        assert_eq!(r["result"]["tick"], (i + 1) as u64);
    }
}

#[test]
fn ts_style_engine_load_scene() {
    let mut bridge = Bridge::new();
    let scene_value: serde_json::Value =
        serde_json::from_str(&scene_json()).expect("parse scene json");
    let req = make_request("engine.loadScene", json!({"scene": scene_value}), 18);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert!(resp["error"].is_null() || resp.get("error").is_none());
    assert_eq!(bridge.engine().scene.as_ref().unwrap().nodes.len(), 1);
}

#[test]
fn ts_style_engine_version() {
    let mut bridge = Bridge::new();
    let req = make_request("engine.version", serde_json::json!({}), 19);
    let resp: serde_json::Value = serde_json::from_str(&bridge.dispatch_str(&req)).unwrap();
    assert!(resp["result"].is_string());
    assert!(!resp["result"].as_str().unwrap().is_empty());
}
