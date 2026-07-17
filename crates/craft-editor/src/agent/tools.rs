use craft_kernel::scene::NodeRegistry;
use craft_kernel::{Action, Scene, evaluate_dry_run, explain_node, lint};

use super::types::{ToolDef, ToolFunction};

#[derive(Default)]
pub struct ToolRegistry;

impl ToolRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn all_defs(&self) -> Vec<ToolDef> {
        vec![
            make_tool(
                "lint",
                "Analyze the current scene for issues",
                serde_json::json!({
                    "type": "object", "properties": {}, "required": []
                }),
            ),
            make_tool(
                "dry_run",
                "Simulate actions on a node without side effects",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "node_id": {"type": "string"},
                        "actions": {"type": "array"}
                    },
                    "required": ["node_id", "actions"]
                }),
            ),
            make_tool(
                "explain",
                "Get a node's structured JSON summary",
                serde_json::json!({
                    "type": "object",
                    "properties": {"node_id": {"type": "string"}},
                    "required": ["node_id"]
                }),
            ),
            make_tool(
                "read_scene",
                "Get the current scene's summary",
                serde_json::json!({
                    "type": "object", "properties": {}, "required": []
                }),
            ),
            make_tool(
                "read_node",
                "Get a node's full JSON",
                serde_json::json!({
                    "type": "object",
                    "properties": {"node_id": {"type": "string"}},
                    "required": ["node_id"]
                }),
            ),
            make_tool(
                "propose_diff",
                "Propose scene changes (not a real tool — returns inline diffs)",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "description": {"type": "string"},
                        "diff": {"type": "object"}
                    },
                    "required": ["description", "diff"]
                }),
            ),
        ]
    }

    pub fn execute(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        scene: &Scene,
        registry: &NodeRegistry,
    ) -> Result<String, String> {
        match tool_name {
            "lint" => Ok(execute_lint(scene, registry)),
            "dry_run" => execute_dry_run_tool(arguments, scene, registry),
            "explain" => execute_explain(arguments, scene),
            "read_scene" => Ok(serde_json::to_string(&make_scene_info(scene)).unwrap_or_default()),
            "read_node" => execute_read_node(arguments, scene),
            "propose_diff" => Err("propose_diff is inline — should not be called as tool".into()),
            _ => Err(format!("Unknown tool: {tool_name}")),
        }
    }
}

fn make_tool(name: &str, description: &str, parameters: serde_json::Value) -> ToolDef {
    ToolDef {
        def_type: "function".into(),
        function: ToolFunction {
            name: name.into(),
            description: description.into(),
            parameters,
        },
    }
}

fn execute_lint(scene: &Scene, registry: &NodeRegistry) -> String {
    let warnings = lint(scene, registry);
    if warnings.is_empty() {
        "lint completed: 0 issues found".into()
    } else {
        let mut out = format!("{} issue(s) found:\n", warnings.len());
        for w in &warnings {
            out.push_str(&format!(
                "  - {}: {}\n",
                w.node.as_deref().unwrap_or("?"),
                w.message
            ));
        }
        out
    }
}

fn execute_dry_run_tool(
    args: &serde_json::Value,
    scene: &Scene,
    registry: &NodeRegistry,
) -> Result<String, String> {
    let node_id = args["node_id"].as_str().ok_or("missing node_id")?;
    let actions: Vec<Action> = serde_json::from_value(args["actions"].clone())
        .map_err(|e| format!("invalid actions: {e}"))?;
    let results = evaluate_dry_run(scene, registry, node_id, &actions)
        .map_err(|e| format!("dry_run error: {e}"))?;
    if results.is_empty() {
        Ok("dry_run: no changes".into())
    } else {
        let mut out = String::new();
        for r in &results {
            out.push_str(&format!("  {r:?}\n"));
        }
        Ok(out)
    }
}

fn execute_explain(args: &serde_json::Value, scene: &Scene) -> Result<String, String> {
    let node_id = args["node_id"].as_str().ok_or("missing node_id")?;
    let node = scene
        .find_node(node_id)
        .ok_or(format!("node '{node_id}' not found"))?;
    let json = explain_node(node, scene);
    Ok(serde_json::to_string(&json).unwrap_or_default())
}

fn make_scene_info(scene: &Scene) -> serde_json::Value {
    let mut node_types: Vec<String> = Vec::new();
    let mut root_id: Option<String> = None;
    for node in &scene.nodes {
        if !node_types.contains(&node.type_name) {
            node_types.push(node.type_name.clone());
        }
        if node.parent.is_none() && root_id.is_none() {
            root_id = Some(node.id.clone());
        }
    }
    serde_json::json!({
        "name": scene.name,
        "node_count": scene.nodes.len(),
        "node_types": node_types,
        "root_id": root_id,
    })
}

fn execute_read_node(args: &serde_json::Value, scene: &Scene) -> Result<String, String> {
    let node_id = args["node_id"].as_str().ok_or("missing node_id")?;
    let node = scene
        .find_node(node_id)
        .ok_or(format!("node '{node_id}' not found"))?;
    Ok(serde_json::to_string(&node).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use craft_kernel::scene::{NodeRegistry, Scene};

    fn make_scene() -> (Scene, NodeRegistry) {
        let registry = NodeRegistry::new();
        let scene = Scene {
            kind: "scene".into(),
            name: "test_scene".into(),
            nodes: vec![],
            spawn_counter: 0,
        };
        (scene, registry)
    }

    #[test]
    fn lint_returns_string() {
        let (scene, registry) = make_scene();
        let result = execute_lint(&scene, &registry);
        assert!(result.contains("lint completed") || result.contains("0 issues"));
    }

    #[test]
    fn read_scene_returns_scene_info() {
        let (scene, _registry) = make_scene();
        let info = make_scene_info(&scene);
        assert_eq!(info["name"], "test_scene");
        assert!(info["node_count"].as_u64().is_some());
    }

    #[test]
    fn tool_registry_has_six_tools() {
        let tools = ToolRegistry::new();
        let defs = tools.all_defs();
        assert_eq!(defs.len(), 6);
        let names: Vec<&str> = defs.iter().map(|d| d.function.name.as_str()).collect();
        assert!(names.contains(&"lint"));
        assert!(names.contains(&"explain"));
        assert!(names.contains(&"read_scene"));
        assert!(names.contains(&"read_node"));
        assert!(names.contains(&"dry_run"));
        assert!(names.contains(&"propose_diff"));
    }

    #[test]
    fn execute_returns_error_for_missing_node() {
        let (scene, _registry) = make_scene();
        let result = execute_read_node(&serde_json::json!({"node_id": "nope"}), &scene);
        assert!(result.is_err());
    }
}
