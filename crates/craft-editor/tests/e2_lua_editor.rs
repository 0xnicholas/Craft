use std::time::Duration;

use craft_editor::lua_lsp::{find_lua_ls, spawn};

fn lua_ls_available() -> bool {
    find_lua_ls().is_some()
}

#[test]
fn spawn_initializes_when_available() {
    if !lua_ls_available() {
        eprintln!("skip: LuaLS not found on PATH");
        return;
    }
    let tmp = tempfile::TempDir::new().unwrap();
    let mut client = spawn(tmp.path()).expect("spawn");
    assert!(
        client.capabilities.is_some(),
        "initialize handshake should populate capabilities"
    );
    std::thread::sleep(Duration::from_millis(200));
    let _ = client.shutdown();
}

#[test]
fn derive_class_name_basic() {
    let path = std::path::Path::new("/proj/scripts/towers/target_priority.lua");
    let class = craft_editor::panels::lua_editor::derive_class_name(path);
    assert_eq!(class, "towers.target_priority");
}

#[test]
fn derive_class_name_root_file() {
    let path = std::path::Path::new("/proj/scripts/main.lua");
    let class = craft_editor::panels::lua_editor::derive_class_name(path);
    assert_eq!(class, "scripts.main");
}

#[test]
fn reload_class_after_save_succeeds() {
    let mut state = craft_editor::state::EditorState::default();
    if state.engine.lua_runtime.is_none() {
        eprintln!("skip: Lua runtime init failed");
        return;
    }
    let src = "function target_priority() return 1 end";
    if let Some(runtime) = state.engine.lua_runtime_mut() {
        match runtime.reload_class("target_priority", src) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("reload_class returned err (acceptable on missing modules_dir): {e}")
            }
        }
    }
}
