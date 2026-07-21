use std::path::PathBuf;

use egui::Ui;

use crate::agent::SYSTEM_PROMPT;
use crate::state::{EditorError, EditorState};

#[derive(Debug, Clone)]
pub enum PanelAction {
    OpenScene(PathBuf),
    SaveScene,
    RunScene,
    StopScene,
    StepTick,
    ReloadScene,
    SetStatus(String),
    ReportError(EditorError),
    RequestQuit,
    OpenBehaviorFile(PathBuf),
    OpenLuaFile(PathBuf),
    AgentSendMessage(String),
    AddChildNode,
    AddChildNodeAt(String),
    DuplicateNode(String),
    DeleteNode(String),
    ReparentNode(String, String),
    SetLuaClass(String, String),
    RenameNode(String, String),
    NewFile(PathBuf, String),
    NewFolder(PathBuf),
    DeleteFile(PathBuf),
}

pub trait Panel {
    fn id(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn show(&mut self, ui: &mut Ui, state: &mut EditorState) -> Vec<PanelAction>;
}

pub mod agent_panel;
pub mod behavior_editor;
pub mod file_browser;
pub mod inspector;
pub mod lua_editor;
pub mod scene_tree;
pub mod terminal_preview;

pub use agent_panel::AgentPanel;
pub use behavior_editor::BehaviorEditorPanel;
pub use file_browser::FileBrowserPanel;
pub use inspector::InspectorPanel;
pub use lua_editor::LuaEditorPanel;
pub use scene_tree::SceneTreePanel;
pub use terminal_preview::TerminalPreviewPanel;

pub fn dispatch(actions: Vec<PanelAction>, state: &mut EditorState) {
    for action in actions {
        match action {
            PanelAction::SaveScene => {
                let _ = state.save_dirty();
            }
            PanelAction::SetStatus(msg) => state.ui.status_message = msg,
            PanelAction::ReportError(err) => state.errors.push(err),
            PanelAction::RequestQuit => {}
            PanelAction::OpenScene(p) => {
                let _ = state.open_scene(&p);
            }
            PanelAction::RunScene => {
                let _ = state.save_dirty();
                launch_gpu_subprocess(state);
                state.engine.is_running = true;
            }
            PanelAction::StopScene => {
                kill_gpu_subprocess(state);
                state.engine.stop();
            }
            PanelAction::StepTick => state.engine.step(),
            PanelAction::ReloadScene => {
                let _ = state.engine.reload();
            }
            PanelAction::OpenBehaviorFile(path) => {
                crate::panels::behavior_editor::open_file(state, path);
            }
            PanelAction::OpenLuaFile(path) => {
                crate::panels::lua_editor::open_file(state, path);
            }
            PanelAction::AddChildNode => {
                let parent_id = state.panels.scene_tree.selected_node.clone();
                if let Some(pid) = parent_id {
                    dispatch_add_child_node(state, &pid);
                }
            }
            PanelAction::AddChildNodeAt(parent_id) => {
                dispatch_add_child_node(state, &parent_id);
            }
            PanelAction::DeleteNode(node_id) => {
                dispatch_delete_node(state, &node_id);
            }
            PanelAction::DuplicateNode(node_id) => {
                dispatch_duplicate_node(state, &node_id);
            }
            PanelAction::ReparentNode(child_id, new_parent_id) => {
                dispatch_reparent_node(state, &child_id, &new_parent_id);
            }
            PanelAction::SetLuaClass(node_id, lua_path) => {
                dispatch_set_lua_class(state, &node_id, &lua_path);
            }
            PanelAction::RenameNode(node_id, new_name) => {
                dispatch_rename_node(state, &node_id, &new_name);
            }
            PanelAction::NewFile(parent_dir, name) => {
                let path = parent_dir.join(&name);
                if let Err(e) = std::fs::write(&path, "") {
                    state.ui.status_message = format!("failed to create file: {e}");
                } else {
                    state.ui.status_message = format!("created {}", path.display());
                }
            }
            PanelAction::NewFolder(parent_dir) => {
                let new_dir = parent_dir.join("new_folder");
                let mut path = new_dir.clone();
                let mut i = 1;
                while path.exists() {
                    path = parent_dir.join(format!("new_folder_{}", i));
                    i += 1;
                }
                if let Err(e) = std::fs::create_dir(&path) {
                    state.ui.status_message = format!("failed to create folder: {e}");
                } else {
                    state.ui.status_message = format!("created {}", path.display());
                }
            }
            PanelAction::DeleteFile(path) => {
                if path.is_dir() {
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        state.ui.status_message = format!("failed to delete dir: {e}");
                    } else {
                        state.ui.status_message = format!("deleted {}", path.display());
                    }
                } else if let Err(e) = std::fs::remove_file(&path) {
                    state.ui.status_message = format!("failed to delete file: {e}");
                } else {
                    state.ui.status_message = format!("deleted {}", path.display());
                }
            }
            PanelAction::AgentSendMessage(text) => {
                if state.agent_rx.is_some() {
                    state
                        .panels
                        .agent_panel
                        .messages
                        .push(crate::state::AgentMessage::System {
                            text: "Already processing a request".into(),
                        });
                    return;
                }
                if let Some(ref client) = state.agent_client {
                    let context = crate::agent::context::ContextBuilder::build_from_state(state);
                    let context_msg =
                        crate::agent::context::ContextBuilder::build_system_message(&context);
                    let tools = state.agent_tool_registry.all_defs();

                    let messages = vec![
                        crate::agent::ChatMessage {
                            role: "system".into(),
                            content: SYSTEM_PROMPT.into(),
                            tool_calls: None,
                            tool_call_id: None,
                        },
                        context_msg,
                        crate::agent::ChatMessage {
                            role: "user".into(),
                            content: text,
                            tool_calls: None,
                            tool_call_id: None,
                        },
                    ];

                    let (tx, rx) = std::sync::mpsc::channel();
                    state.agent_rx = Some(rx);
                    state.panels.agent_panel.is_streaming = true;
                    state.panels.agent_panel.streaming_text.clear();
                    state.panels.agent_panel.tool_round = 0;

                    if let Some(handle) = client.chat(messages, &tools, false, tx) {
                        state.agent_handle = Some(handle);
                    }
                }
            }
        }
    }
}

fn dispatch_add_child_node(state: &mut EditorState, parent_id: &str) {
    let Some(ref mut scene_state) = state.scene else {
        return;
    };
    let parent_id = parent_id.to_string();
    let new_id = format!("__editor_{}", scene_state.def.spawn_counter);
    scene_state.def.spawn_counter += 1;
    let node = craft_kernel::Node {
        id: new_id.clone(),
        type_name: "Node".to_string(),
        parent: Some(parent_id.clone()),
        components: std::collections::BTreeMap::new(),
        behaviors: Vec::new(),
        active_state: None,
        lua_class: None,
        destroyed: false,
    };
    state.undo_redo.begin_action("add child node");
    let nid = new_id.clone();
    state.undo_redo.add_undo(move |s| {
        if let Some(ref mut ss) = s.scene {
            ss.def.nodes.retain(|n| n.id != nid);
        }
    });
    let nid2 = new_id.clone();
    let pid = parent_id.clone();
    state.undo_redo.add_do(move |s| {
        if let Some(ref mut ss) = s.scene {
            if !ss.def.nodes.iter().any(|n| n.id == nid2) {
                ss.def.nodes.push(craft_kernel::Node {
                    id: nid2.clone(),
                    type_name: "Node".to_string(),
                    parent: Some(pid.clone()),
                    components: std::collections::BTreeMap::new(),
                    behaviors: Vec::new(),
                    active_state: None,
                    lua_class: None,
                    destroyed: false,
                });
            }
        }
    });
    scene_state.def.nodes.push(node);
    state.ui.status_message = format!("added child node {new_id}");
    state.undo_redo.commit_action();
}

fn dispatch_delete_node(state: &mut EditorState, node_id: &str) {
    let node_id = node_id.to_string();
    let snapshot = state
        .scene
        .as_ref()
        .and_then(|s| s.def.nodes.iter().find(|n| n.id == node_id).cloned());
    let Some(snapshot) = snapshot else {
        return;
    };
    {
        let Some(ref mut scene_state) = state.scene else {
            return;
        };
        if let Some(node) = scene_state.def.nodes.iter_mut().find(|n| n.id == node_id) {
            node.destroyed = true;
        }
    }
    purge_destroyed(state);

    state.undo_redo.begin_action("delete node");
    let nid = node_id.clone();
    let restore = snapshot;
    state.undo_redo.add_undo(move |s| {
        if let Some(ref mut ss) = s.scene {
            if !ss.def.nodes.iter().any(|n| n.id == nid) {
                let mut n = restore.clone();
                n.destroyed = false;
                ss.def.nodes.push(n);
            }
        }
    });
    let nid2 = node_id.clone();
    state.undo_redo.add_do(move |s| {
        if let Some(ref mut ss) = s.scene {
            if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == nid2) {
                n.destroyed = true;
            }
        }
        purge_destroyed(s);
    });
    state.ui.status_message = format!("deleted node {node_id}");
    state.undo_redo.commit_action();
}

fn purge_destroyed(state: &mut EditorState) {
    let Some(ref mut scene_state) = state.scene else {
        return;
    };
    let destroyed_ids: Vec<String> = scene_state
        .def
        .nodes
        .iter()
        .filter(|n| n.destroyed)
        .map(|n| n.id.clone())
        .collect();
    for id in &destroyed_ids {
        for node in &mut scene_state.def.nodes {
            if node.parent.as_deref() == Some(id.as_str()) {
                node.destroyed = true;
            }
        }
    }
    scene_state.def.nodes.retain(|n| !n.destroyed);
    let selected_is_destroyed = state
        .panels
        .scene_tree
        .selected_node
        .as_ref()
        .is_some_and(|sid| destroyed_ids.contains(sid));
    if selected_is_destroyed {
        state.panels.scene_tree.selected_node = None;
    }
}

fn dispatch_duplicate_node(state: &mut EditorState, node_id: &str) {
    let Some(ref mut scene_state) = state.scene else {
        return;
    };
    let node_id = node_id.to_string();
    let Some(original) = scene_state.def.nodes.iter().find(|n| n.id == node_id) else {
        return;
    };
    let parent = original.parent.clone();
    let new_id = format!("__editor_{}", scene_state.def.spawn_counter);
    scene_state.def.spawn_counter += 1;
    let clone = craft_kernel::Node {
        id: new_id.clone(),
        type_name: original.type_name.clone(),
        parent: parent.clone(),
        components: original.components.clone(),
        behaviors: original.behaviors.clone(),
        active_state: original.active_state.clone(),
        lua_class: original.lua_class.clone(),
        destroyed: false,
    };
    state.undo_redo.begin_action("duplicate node");
    let nid = new_id.clone();
    state.undo_redo.add_undo(move |s| {
        if let Some(ref mut ss) = s.scene {
            ss.def.nodes.retain(|n| n.id != nid);
        }
    });
    let nid2 = new_id.clone();
    let clone_for_do = clone.clone();
    state.undo_redo.add_do(move |s| {
        if let Some(ref mut ss) = s.scene {
            if !ss.def.nodes.iter().any(|n| n.id == nid2) {
                let mut c = clone_for_do.clone();
                c.id = nid2.clone();
                ss.def.nodes.push(c);
            }
        }
    });
    scene_state.def.nodes.push(clone);
    state.ui.status_message = format!("duplicated node {new_id}");
    state.undo_redo.commit_action();
}

fn dispatch_reparent_node(state: &mut EditorState, child_id: &str, new_parent_id: &str) {
    let Some(ref mut scene_state) = state.scene else {
        return;
    };
    if child_id == new_parent_id {
        return;
    }
    let is_descendant = scene_state
        .def
        .nodes
        .iter()
        .any(|n| n.parent.as_deref() == Some(child_id) && n.id == new_parent_id);
    if is_descendant {
        state.ui.status_message = "cannot reparent to a descendant".to_string();
        return;
    }
    let Some(node) = scene_state.def.nodes.iter_mut().find(|n| n.id == child_id) else {
        return;
    };
    let old_parent = node.parent.clone();
    let new_parent = Some(new_parent_id.to_string());
    node.parent = new_parent.clone();
    state.ui.status_message = format!("reparented {child_id}");
    state.undo_redo.begin_action("reparent node");
    let cid = child_id.to_string();
    let op = old_parent.clone();
    state.undo_redo.add_undo(move |s| {
        if let Some(ref mut ss) = s.scene {
            if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == cid) {
                n.parent = op.clone();
            }
        }
    });
    let cid2 = child_id.to_string();
    let np = new_parent.clone();
    state.undo_redo.add_do(move |s| {
        if let Some(ref mut ss) = s.scene {
            if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == cid2) {
                n.parent = np.clone();
            }
        }
    });
    state.undo_redo.commit_action();
}

fn dispatch_set_lua_class(state: &mut EditorState, node_id: &str, lua_path: &str) {
    let Some(ref mut scene_state) = state.scene else {
        return;
    };
    let Some(node) = scene_state.def.nodes.iter_mut().find(|n| n.id == node_id) else {
        return;
    };
    let old = node.lua_class.clone();
    let new = Some(lua_path.to_string());
    node.lua_class = new.clone();
    state.ui.status_message = format!("set lua_class on {node_id}");
    state.undo_redo.begin_action("set lua class");
    let nid = node_id.to_string();
    let o = old.clone();
    state.undo_redo.add_undo(move |s| {
        if let Some(ref mut ss) = s.scene {
            if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == nid) {
                n.lua_class = o.clone();
            }
        }
    });
    let nid2 = node_id.to_string();
    let n = new.clone();
    state.undo_redo.add_do(move |s| {
        if let Some(ref mut ss) = s.scene {
            if let Some(n2) = ss.def.nodes.iter_mut().find(|n| n.id == nid2) {
                n2.lua_class = n.clone();
            }
        }
    });
    state.undo_redo.commit_action();
}

fn dispatch_rename_node(state: &mut EditorState, node_id: &str, new_name: &str) {
    let new_id = new_name.to_string();
    {
        let Some(ref mut scene_state) = state.scene else {
            return;
        };
        let old_id = scene_state
            .def
            .nodes
            .iter()
            .find(|n| n.id == node_id)
            .map(|n| n.id.clone());
        let Some(old_id) = old_id else {
            return;
        };
        if old_id == new_id {
            return;
        }
        if scene_state.def.nodes.iter().any(|n| n.id == new_id) {
            state.ui.status_message = format!("node id {new_id} already exists");
            return;
        }
        if let Some(node) = scene_state.def.nodes.iter_mut().find(|n| n.id == node_id) {
            node.id = new_id.clone();
        }
        for n in &mut scene_state.def.nodes {
            if n.parent.as_deref() == Some(old_id.as_str()) {
                n.parent = Some(new_id.clone());
            }
        }
        if state.panels.scene_tree.selected_node.as_deref() == Some(old_id.as_str()) {
            state.panels.scene_tree.selected_node = Some(new_id.clone());
        }
        state.undo_redo.begin_action("rename node");
        let oid = old_id.clone();
        let nid = new_id.clone();
        state.undo_redo.add_undo(move |s| {
            if let Some(ref mut ss) = s.scene {
                if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == nid) {
                    n.id = oid.clone();
                }
                for n in &mut ss.def.nodes {
                    if n.parent.as_deref() == Some(nid.as_str()) {
                        n.parent = Some(oid.clone());
                    }
                }
            }
            if s.panels.scene_tree.selected_node.as_deref() == Some(nid.as_str()) {
                s.panels.scene_tree.selected_node = Some(oid.clone());
            }
        });
        let oid2 = old_id.clone();
        let nid2 = new_id.clone();
        state.undo_redo.add_do(move |s| {
            if let Some(ref mut ss) = s.scene {
                if let Some(n) = ss.def.nodes.iter_mut().find(|n| n.id == oid2) {
                    n.id = nid2.clone();
                }
                for n in &mut ss.def.nodes {
                    if n.parent.as_deref() == Some(oid2.as_str()) {
                        n.parent = Some(nid2.clone());
                    }
                }
            }
            if s.panels.scene_tree.selected_node.as_deref() == Some(oid2.as_str()) {
                s.panels.scene_tree.selected_node = Some(nid2.clone());
            }
        });
        state.ui.status_message = format!("renamed {old_id} -> {new_id}");
        state.undo_redo.commit_action();
    }
}

fn launch_gpu_subprocess(state: &mut EditorState) {
    kill_gpu_subprocess(state);

    if let Some(ref scene_state) = state.scene {
        let scene_path = &scene_state.path;
        let asset_root = scene_path
            .parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .join("assets");

        let scene_json = match serde_json::to_string(&scene_state.def) {
            Ok(j) => j,
            Err(e) => {
                state.ui.status_message = format!("serialize error: {e}");
                return;
            }
        };
        let tmp = std::env::temp_dir().join("craft_scene.json");
        if let Err(e) = std::fs::write(&tmp, &scene_json) {
            state.ui.status_message = format!("write temp scene: {e}");
            return;
        }

        let exe = std::env::current_exe().unwrap_or_default();
        let gpu_bin = exe
            .parent()
            .unwrap_or(&std::path::PathBuf::from("."))
            .join(format!("craft-gpu{}", std::env::consts::EXE_SUFFIX));

        match std::process::Command::new(&gpu_bin)
            .arg(&tmp)
            .arg("--asset-root")
            .arg(&asset_root)
            .spawn()
        {
            Ok(child) => {
                state.ui.status_message = "GPU game window launched".into();
                state.game_child = Some(child);
            }
            Err(e) => {
                state.ui.status_message = format!(
                    "failed to launch GPU window: {e}. Dev: cargo run -p craft-gpu -- {} --asset-root {}",
                    tmp.display(),
                    asset_root.display()
                );
            }
        }
    }
}

fn kill_gpu_subprocess(state: &mut EditorState) {
    if let Some(mut child) = state.game_child.take() {
        let _ = child.kill();
        let _ = child.wait();
        state.ui.status_message = "GPU game window closed".into();
    }
}
