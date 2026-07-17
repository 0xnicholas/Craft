use std::path::PathBuf;
use std::time::Instant;

use egui_dock::{DockState, TabViewer};

use crate::agent::SYSTEM_PROMPT;
use crate::io::recent;
use crate::panels::{
    AgentPanel, BehaviorEditorPanel, FileBrowserPanel, InspectorPanel, LuaEditorPanel, Panel,
    PanelAction, SceneTreePanel, TerminalPreviewPanel,
};
use crate::persist;
use crate::state::{EditorState, FileChangeKind, FileChangePending, ProjectState};
use crate::watcher::{Watcher, WatcherEvent};

pub struct EditorApp {
    pub state: EditorState,
    pub dock: DockState<String>,
    pub watcher: Option<Watcher>,
    pub last_watcher_poll: Instant,
    pub pending_actions: Vec<PanelAction>,
    pub scene_tree_panel: SceneTreePanel,
    pub inspector_panel: InspectorPanel,
    pub file_browser_panel: FileBrowserPanel,
    pub terminal_panel: TerminalPreviewPanel,
    pub behavior_panel: BehaviorEditorPanel,
    pub lua_panel: LuaEditorPanel,
    pub agent_panel: AgentPanel,
}

impl Default for EditorApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl EditorApp {
    pub fn new(initial_project: Option<PathBuf>) -> Self {
        let mut state = EditorState::default();

        let mut recorded_recent = recent::load();
        let watcher = initial_project.as_ref().and_then(|p| Watcher::new(p).ok());
        if let Some(p) = initial_project.as_ref() {
            recorded_recent.add_or_bump(p);
            let _ = recent::save(&recorded_recent);
        }

        state.project = initial_project.map(|p| ProjectState { root: p });

        Self {
            dock: persist::load_dock().unwrap_or_else(persist::build_default_dock),
            state,
            watcher,
            last_watcher_poll: Instant::now(),
            pending_actions: Vec::new(),
            scene_tree_panel: SceneTreePanel,
            inspector_panel: InspectorPanel,
            file_browser_panel: FileBrowserPanel,
            terminal_panel: TerminalPreviewPanel,
            behavior_panel: BehaviorEditorPanel,
            lua_panel: LuaEditorPanel,
            agent_panel: AgentPanel,
        }
    }

    fn on_welcome_choice(&mut self, root: PathBuf) {
        let mut r = recent::load();
        r.add_or_bump(&root);
        let _ = recent::save(&r);
        self.state.project = Some(ProjectState { root: root.clone() });
        self.watcher = Watcher::new(&root).ok();
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::theme::apply(ctx);
        crate::menu::draw(ctx, self);

        if self.state.project.is_none() {
            self.draw_welcome(ctx);
            return;
        }

        let mut do_undo = false;
        let mut do_redo = false;
        let mut focus_tab_title: Option<String> = None;
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                let unapplied = self
                    .state
                    .panels
                    .inspector
                    .behavior_edits
                    .values()
                    .any(|e| e.dirty && e.parsed.is_none());
                if unapplied {
                    self.state.ui.status_message =
                        "behavior modified — press Apply first".to_string();
                } else {
                    self.pending_actions.push(PanelAction::SaveScene);
                }
            }
            if i.key_pressed(egui::Key::F5) {
                self.pending_actions.push(PanelAction::RunScene);
            }
            if i.key_pressed(egui::Key::F8) {
                self.pending_actions.push(PanelAction::StopScene);
            }
            if i.key_pressed(egui::Key::F10) {
                self.pending_actions.push(PanelAction::StepTick);
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                if i.modifiers.shift {
                    do_redo = true;
                } else {
                    do_undo = true;
                }
            }
            let num_keys = [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
                egui::Key::Num5,
            ];
            for panel_num in 1..=5 {
                if i.modifiers.ctrl && i.key_pressed(num_keys[panel_num - 1]) {
                    let titles: &[&str] = &[
                        "Scene Tree",
                        "Inspector",
                        "Files",
                        "Terminal Preview",
                        "Agent Copilot",
                    ];
                    focus_tab_title = Some(titles[panel_num - 1].to_string());
                }
            }
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::A) {
                self.pending_actions.push(PanelAction::AddChildNode);
            }
        });
        if let Some(title) = focus_tab_title {
            if let Some((si, ni, _)) = self.dock.find_tab(&title) {
                self.dock.set_focused_node_and_surface((si, ni));
            }
        }
        if do_undo {
            let mut ur =
                std::mem::replace(&mut self.state.undo_redo, crate::undo::UndoRedo::new(100));
            ur.undo(&mut self.state);
            self.state.undo_redo = ur;
        }
        if do_redo {
            let mut ur =
                std::mem::replace(&mut self.state.undo_redo, crate::undo::UndoRedo::new(100));
            ur.redo(&mut self.state);
            self.state.undo_redo = ur;
        }

        if let Some(w) = &self.watcher {
            for ev in w.drain_debounced() {
                if let WatcherEvent::Changed(p) = ev {
                    let kind = {
                        let path_str = p.to_string_lossy();
                        if path_str.ends_with(".behavior.json") {
                            Some(FileChangeKind::Behavior)
                        } else if p.extension().and_then(|e| e.to_str()) == Some("lua") {
                            Some(FileChangeKind::Lua)
                        } else {
                            let is_current_scene = self
                                .state
                                .scene
                                .as_ref()
                                .map(|s| s.path == p)
                                .unwrap_or(false);
                            if is_current_scene {
                                Some(FileChangeKind::SceneJson)
                            } else {
                                None
                            }
                        }
                    };
                    if let Some(kind) = kind {
                        self.state.ui.file_change_pending =
                            Some(FileChangePending { path: p, kind });
                    }
                }
            }
        }

        self.state.engine.tick_if_due();

        let mut dock = std::mem::replace(&mut self.dock, persist::build_default_dock());
        let pending_from_panels = {
            let mut viewer = PanelsViewer { app: self };
            egui_dock::DockArea::new(&mut dock).show(ctx, &mut viewer);
            std::mem::take(&mut viewer.app.pending_actions)
        };
        self.dock = dock;
        crate::panels::dispatch(pending_from_panels, &mut self.state);

        // Drain agent stream events
        if self.state.agent_rx.is_some() {
            let rx = self.state.agent_rx.take().unwrap();
            let panel = &mut self.state.panels.agent_panel;
            let mut tool_loop_needed = false;
            let mut tool_loop_messages: Vec<crate::agent::ChatMessage> = vec![];
            let mut tool_loop_tools: Vec<crate::agent::ToolDef> = vec![];

            while let Ok(event) = rx.try_recv() {
                match event {
                    crate::agent::AgentStreamEvent::Token(t) => {
                        panel.streaming_text.push_str(&t);
                    }
                    crate::agent::AgentStreamEvent::Done {
                        full_text,
                        tool_calls,
                    } => {
                        if !tool_calls.is_empty() && panel.tool_round < 3 {
                            panel.messages.push(crate::state::AgentMessage::Agent {
                                text: full_text,
                                suggestions: vec![],
                            });

                            if let Some(ref scene_state) = self.state.scene {
                                let mut loop_msgs = vec![crate::agent::ChatMessage {
                                    role: "system".into(),
                                    content: SYSTEM_PROMPT.into(),
                                    tool_calls: None,
                                    tool_call_id: None,
                                }];

                                for tc in &tool_calls {
                                    let args: serde_json::Value =
                                        serde_json::from_str(&tc.function.arguments)
                                            .unwrap_or_default();
                                    let result = self.state.agent_tool_registry.execute(
                                        &tc.function.name,
                                        &args,
                                        &scene_state.def,
                                        self.state.engine.engine.node_registry_mut(),
                                    );
                                    let result_text =
                                        result.unwrap_or_else(|e| format!("Error: {e}"));
                                    loop_msgs.push(crate::agent::ChatMessage {
                                        role: "tool".into(),
                                        content: result_text,
                                        tool_calls: None,
                                        tool_call_id: Some(tc.id.clone()),
                                    });
                                }

                                panel.tool_round += 1;
                                tool_loop_needed = true;
                                tool_loop_messages = loop_msgs;
                                tool_loop_tools = self.state.agent_tool_registry.all_defs();
                            }
                        } else {
                            panel.is_streaming = false;
                            panel.tool_round = 0;

                            let parsed: serde_json::Value = serde_json::from_str(&full_text)
                                .unwrap_or(serde_json::json!({"reply": full_text}));
                            let reply = parsed["reply"].as_str().unwrap_or(&full_text).to_string();
                            let diffs: Vec<crate::state::AgentSuggestion> = parsed["diffs"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .enumerate()
                                        .map(|(i, d)| crate::state::AgentSuggestion {
                                            id: format!(
                                                "suggestion-{}",
                                                panel.suggestion_counter + i as u32 + 1
                                            ),
                                            description: d["description"]
                                                .as_str()
                                                .unwrap_or("")
                                                .into(),
                                            diff: serde_json::from_value(d["diff"].clone())
                                                .unwrap_or_default(),
                                            status: crate::state::SuggestionStatus::Pending,
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            panel.suggestion_counter += diffs.len() as u32;
                            panel.messages.push(crate::state::AgentMessage::Agent {
                                text: reply,
                                suggestions: diffs,
                            });
                            self.state.agent_handle = None;
                        }
                    }
                    crate::agent::AgentStreamEvent::Error(e) => {
                        panel.is_streaming = false;
                        panel.tool_round = 0;
                        panel.messages.push(crate::state::AgentMessage::System {
                            text: format!("Error: {e}"),
                        });
                        self.state.agent_handle = None;
                    }
                }
            }

            if tool_loop_needed {
                if let Some(ref client) = self.state.agent_client {
                    let (tx, rx) = std::sync::mpsc::channel();
                    self.state.agent_rx = Some(rx);
                    panel.is_streaming = true;
                    panel.streaming_text.clear();
                    if let Some(h) = client.chat(tool_loop_messages, &tool_loop_tools, true, tx) {
                        self.state.agent_handle = Some(h);
                    }
                }
            }
        }

        if let Some(pending) = self.state.ui.file_change_pending.clone() {
            egui::TopBottomPanel::bottom("file_change_prompt").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} changed externally", pending.path.display()));
                    if ui.button("Reload").clicked() {
                        match pending.kind {
                            FileChangeKind::SceneJson => {
                                let _ = self.state.open_scene(&pending.path);
                            }
                            FileChangeKind::Lua => {
                                if let Ok(content) = std::fs::read_to_string(&pending.path) {
                                    self.state.lua_editor.buffer = content;
                                    self.state.lua_editor.dirty = false;
                                    self.state.ui.status_message =
                                        format!("reloaded {}", pending.path.display());
                                }
                            }
                            FileChangeKind::Behavior => {
                                if let Ok(content) = std::fs::read_to_string(&pending.path) {
                                    self.state.standalone_behavior.buffer = content;
                                    self.state.standalone_behavior.dirty = false;
                                    self.state.ui.status_message =
                                        format!("reloaded {}", pending.path.display());
                                }
                            }
                        }
                        self.state.ui.file_change_pending = None;
                    }
                    if ui.button("Keep mine").clicked() {
                        self.state.ui.file_change_pending = None;
                    }
                });
            });
        }

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.label(&self.state.ui.status_message);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(ref client) = self.state.agent_client {
            client.shutdown();
        }
        let _ = persist::save_dock(&self.dock);
    }
}

impl EditorApp {
    fn draw_welcome(&mut self, ctx: &egui::Context) {
        let recent_projects = recent::load();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("craft-editor");
                ui.add_space(20.0);

                ui.label("Recent projects");
                let mut chosen: Option<PathBuf> = None;
                ui.group(|ui| {
                    if recent_projects.entries.is_empty() {
                        ui.label("(none)");
                    } else {
                        for entry in recent_projects.entries.iter().take(5) {
                            let label = entry.root.display().to_string();
                            if ui.button(&label).clicked() {
                                chosen = Some(entry.root.clone());
                            }
                        }
                    }
                });

                ui.add_space(20.0);
                if ui.button("Open Project…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        chosen = Some(dir);
                    }
                }
                if let Some(p) = chosen {
                    self.on_welcome_choice(p);
                }
            });
        });
    }
}

struct PanelsViewer<'a> {
    app: &'a mut EditorApp,
}

impl<'a> TabViewer for PanelsViewer<'a> {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.clone().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        let actions = match tab.as_str() {
            "Scene Tree" => self.app.scene_tree_panel.show(ui, &mut self.app.state),
            "Inspector" => self.app.inspector_panel.show(ui, &mut self.app.state),
            "Files" => self.app.file_browser_panel.show(ui, &mut self.app.state),
            "Terminal Preview" => self.app.terminal_panel.show(ui, &mut self.app.state),
            "Behavior Editor" => self.app.behavior_panel.show(ui, &mut self.app.state),
            "Lua Editor" => self.app.lua_panel.show(ui, &mut self.app.state),
            "Agent Copilot" => self.app.agent_panel.show(ui, &mut self.app.state),
            _ => {
                ui.label(format!("Unknown tab: {tab}"));
                Vec::new()
            }
        };
        if !actions.is_empty() {
            self.app.pending_actions.extend(actions);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist;
    use egui_dock::DockState;

    #[test]
    fn editor_app_constructs_without_project() {
        let app = EditorApp::new(None);
        assert!(app.state.project.is_none());
        assert!(app.watcher.is_none());
    }

    #[test]
    fn editor_app_constructs_with_project_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let app = EditorApp::new(Some(dir.path().to_path_buf()));
        assert_eq!(
            app.state.project.as_ref().map(|p| p.root.clone()),
            Some(dir.path().to_path_buf())
        );
        assert!(
            app.watcher.is_some(),
            "watcher should be created for valid project root"
        );
    }

    #[test]
    fn panel_dispatch_via_pending_actions_runs_save() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = EditorApp::new(Some(dir.path().to_path_buf()));
        app.pending_actions.push(PanelAction::SaveScene);
        let actions = std::mem::take(&mut app.pending_actions);
        crate::panels::dispatch(actions, &mut app.state);
        assert!(app.state.scene.is_none());
        assert!(!app.state.engine.is_running);
    }

    #[test]
    fn run_and_stop_actions_toggle_engine() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut app = EditorApp::new(Some(dir.path().to_path_buf()));
        app.pending_actions.push(PanelAction::RunScene);
        let actions = std::mem::take(&mut app.pending_actions);
        crate::panels::dispatch(actions, &mut app.state);
        assert!(app.state.engine.is_running);
        app.pending_actions.push(PanelAction::StopScene);
        let actions = std::mem::take(&mut app.pending_actions);
        crate::panels::dispatch(actions, &mut app.state);
        assert!(!app.state.engine.is_running);
    }

    #[test]
    fn default_dock_includes_all_seven_tab_titles() {
        let dock = persist::build_default_dock();
        let titles: Vec<String> = dock.main_surface().tabs().map(|t| (*t).clone()).collect();
        let expected = [
            "Scene Tree",
            "Inspector",
            "Files",
            "Terminal Preview",
            "Behavior Editor",
            "Lua Editor",
            "Agent Copilot",
        ];
        for key in expected {
            assert!(titles.iter().any(|t| t == key), "missing tab: {key}");
        }
    }

    #[test]
    fn dock_round_trips_through_bincode() {
        let dock: DockState<String> = DockState::new(vec!["A".into(), "B".into(), "C".into()]);
        let persisted = persist::PersistedDock {
            tab_titles: dock.main_surface().tabs().map(|t| (*t).clone()).collect(),
        };
        let bytes = bincode::serialize(&persisted).expect("serialize");
        let restored: persist::PersistedDock = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(restored.tab_titles.len(), 3);
    }
}
