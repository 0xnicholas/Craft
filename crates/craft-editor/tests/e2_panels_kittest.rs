use craft_editor::panels::{BehaviorEditorPanel, LuaEditorPanel, Panel};
use craft_editor::state::EditorState;
use egui_kittest::Harness;

#[test]
fn behavior_panel_renders_empty_state() {
    let mut state = EditorState::default();
    let mut harness = Harness::new_ui(|ui| {
        BehaviorEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn lua_panel_renders_empty_state() {
    let mut state = EditorState::default();
    let mut harness = Harness::new_ui(|ui| {
        LuaEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn lua_panel_renders_with_runtime_error() {
    let mut state = EditorState::default();
    state.engine.lua_init_error = Some("test init failure".to_string());
    let mut harness = Harness::new_ui(|ui| {
        LuaEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn lua_panel_renders_fallback_mode() {
    let mut state = EditorState::default();
    state.lua_editor.fallback_mode = true;
    state.lua_editor.current_path = Some(std::path::PathBuf::from("/tmp/x.lua"));
    let mut harness = Harness::new_ui(|ui| {
        LuaEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn behavior_panel_renders_with_file_loaded() {
    let mut state = EditorState::default();
    state.standalone_behavior.path = Some(std::path::PathBuf::from("/tmp/test.behavior.json"));
    state.standalone_behavior.buffer = r#"{"kind": "set_state"}"#.to_string();
    let mut harness = Harness::new_ui(|ui| {
        BehaviorEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}

#[test]
fn lua_panel_renders_with_file_loaded() {
    let mut state = EditorState::default();
    state.lua_editor.current_path = Some(std::path::PathBuf::from("/tmp/x.lua"));
    state.lua_editor.buffer = "function f() return 1 end".to_string();
    state.lua_editor.fallback_mode = true;
    let mut harness = Harness::new_ui(|ui| {
        LuaEditorPanel.show(ui, &mut state);
    });
    harness.run();
    harness.fit_contents();
}
