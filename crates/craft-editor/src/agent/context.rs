use std::path::PathBuf;

use craft_kernel::ComponentValue;

use crate::agent::types::ChatMessage;
use crate::state::EditorState;

pub struct AgentContext {
    pub active_file: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub node_count: usize,
    pub selected_node: Option<NodeSummary>,
    pub visible_components: Vec<String>,
    pub recent_changes: Vec<ChangeRecord>,
    pub engine_schema: serde_json::Value,
}

pub struct NodeSummary {
    pub id: String,
    pub type_name: String,
    pub component_keys: Vec<String>,
    pub component_types: Vec<(String, String)>,
    pub behavior_count: usize,
    pub lua_class: Option<String>,
}

pub struct ChangeRecord {
    pub timestamp: u64,
    pub description: String,
}

pub struct ContextBuilder;

impl ContextBuilder {
    pub fn build_from_state(state: &EditorState) -> AgentContext {
        let scene_state = state.scene.as_ref();
        let scene = scene_state.map(|s| &s.def);
        let selected_node = scene_state.and_then(|ss| {
            state
                .panels
                .scene_tree
                .selected_node
                .as_ref()
                .and_then(|id| ss.def.find_node(id))
                .map(|node| {
                    let keys: Vec<String> = node.components.keys().cloned().collect();
                    let types: Vec<(String, String)> = node
                        .components
                        .iter()
                        .map(|(k, c)| (k.clone(), component_type_name(&c.value).to_string()))
                        .collect();
                    NodeSummary {
                        id: node.id.clone(),
                        type_name: node.type_name.clone(),
                        component_keys: keys,
                        component_types: types,
                        behavior_count: node.behaviors.len(),
                        lua_class: node.lua_class.clone(),
                    }
                })
        });

        AgentContext {
            active_file: state.engine.scene_path.clone(),
            scene_name: scene.map(|s| s.name.clone()),
            node_count: scene.map(|s| s.nodes.len()).unwrap_or(0),
            selected_node,
            visible_components: vec![],
            recent_changes: vec![],
            engine_schema: serde_json::json!({"version": "1.0"}),
        }
    }

    pub fn build_system_message(ctx: &AgentContext) -> ChatMessage {
        let mut content = String::new();
        content.push_str("[EDITOR CONTEXT]\n");

        if let Some(ref file) = ctx.active_file {
            let file_name = file.file_name().unwrap_or_default().to_string_lossy();
            content.push_str(&format!("Active file: {file_name}"));
            if let Some(ref name) = ctx.scene_name {
                content.push_str(&format!(" ({name}, {} nodes)", ctx.node_count));
            }
            content.push('\n');
        }

        if let Some(ref node) = ctx.selected_node {
            content.push_str(&format!(
                "Selected node: {} ({})\n",
                node.id, node.type_name
            ));
            content.push_str(&format!(
                "  Components: {}\n",
                node.component_keys.join(", ")
            ));
            if node.behavior_count > 0 {
                content.push_str(&format!("  Behaviors: {}\n", node.behavior_count));
            }
            if let Some(ref lua) = node.lua_class {
                content.push_str(&format!("  Lua class: {lua}\n"));
            }
        }

        if !ctx.recent_changes.is_empty() {
            content.push_str("Recent changes:\n");
            for change in &ctx.recent_changes {
                content.push_str(&format!("  - {}\n", change.description));
            }
        }

        content.push_str(&format!(
            "Engine schema version: {}\n",
            ctx.engine_schema
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
        ));

        ChatMessage {
            role: "system".into(),
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

fn component_type_name(v: &ComponentValue) -> &'static str {
    match v {
        ComponentValue::Nil => "nil",
        ComponentValue::Bool(_) => "bool",
        ComponentValue::Int(_) => "int",
        ComponentValue::Float(_) => "float",
        ComponentValue::String(_) => "string",
        ComponentValue::Vec2(_) => "vec2",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_context() -> AgentContext {
        AgentContext {
            active_file: Some(PathBuf::from("scene.json")),
            scene_name: Some("tower_defense".into()),
            node_count: 14,
            selected_node: Some(NodeSummary {
                id: "tower_1".into(),
                type_name: "Tower".into(),
                component_keys: vec!["cooldown".into(), "range".into()],
                component_types: vec![
                    ("cooldown".into(), "int".into()),
                    ("range".into(), "float".into()),
                ],
                behavior_count: 1,
                lua_class: Some("towers.target_priority".into()),
            }),
            visible_components: vec![],
            recent_changes: vec![],
            engine_schema: serde_json::json!({"version": "1.0"}),
        }
    }

    #[test]
    fn formats_context_with_selected_node() {
        let ctx = make_context();
        let msg = ContextBuilder::build_system_message(&ctx);
        assert!(msg.content.contains("tower_defense"));
        assert!(msg.content.contains("14 nodes"));
        assert!(msg.content.contains("tower_1"));
        assert!(msg.content.contains("Tower"));
        assert!(msg.content.contains("cooldown"));
        assert!(msg.content.contains("towers.target_priority"));
        assert_eq!(msg.role, "system");
    }

    #[test]
    fn formats_context_without_selected_node() {
        let mut ctx = make_context();
        ctx.selected_node = None;
        let msg = ContextBuilder::build_system_message(&ctx);
        assert!(msg.content.contains("tower_defense"));
        assert!(!msg.content.contains("tower_1"));
    }
}
