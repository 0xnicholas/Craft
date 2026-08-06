use std::collections::BTreeMap;
use std::path::PathBuf;

use craft_audio::audio_request;
use craft_kernel::Engine;
use craft_kernel::scene::{Component, ComponentKind, ComponentValue, Node, Scene};
use craft_lua::{LuaEngineHook, LuaRuntime};

fn lua_emitter() -> &'static str {
    r#"
emitter = emitter or {}
emitter.__index = emitter

function emitter.new(node)
    return setmetatable({ node = node }, emitter)
end

function emitter:on_tick()
    engine.emit("jump", { volume = 0.5, loop = false })
end
"#
}

fn scene_with_lua_node() -> Scene {
    let mut components = BTreeMap::new();
    components.insert(
        "ticks".to_string(),
        Component {
            value: ComponentValue::Int(0),
            kind: ComponentKind::Regular,
        },
    );
    Scene {
        kind: "scene".to_string(),
        name: "audio_chain".to_string(),
        nodes: vec![Node {
            id: "e1".to_string(),
            type_name: "Emitter".to_string(),
            parent: None,
            components,
            behaviors: vec![],
            active_state: None,
            lua_class: Some("emitter".to_string()),
            destroyed: false,
        }],
        spawn_counter: 0,
    }
}

#[test]
fn lua_emit_reaches_the_audio_chain() {
    // SAFETY: test process, single-threaded setup before the engine runs.
    unsafe {
        std::env::set_var(
            "CRAFT_ASSET_ROOT",
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata"),
        );
    }
    let scene = scene_with_lua_node();
    let mut runtime = LuaRuntime::new(7).expect("lua");
    runtime.load_class("emitter", lua_emitter()).expect("class");
    let mut hook = LuaEngineHook::new(runtime);
    hook.runtime_mut()
        .bind_node(&scene, "e1", "emitter")
        .expect("bind");
    let mut engine = Engine::new();
    engine.load_scene(scene);
    engine.set_hook(Some(Box::new(hook)));

    // Tick 1: Lua emits "jump" onto the bus (delivered next tick).
    engine.tick();
    // Tick 2: delivery pass collects it; PostTick systems see it.
    engine.tick();

    let delivered = engine.bus.delivered_payloads();
    assert!(
        delivered.iter().any(|(name, _)| name == "jump"),
        "Lua-emitted signal must reach the bus delivery history; got {delivered:?}"
    );

    // The audio pipeline can now resolve it: payload volume/loop flow through.
    let req = delivered
        .iter()
        .find(|(name, _)| name == "jump")
        .and_then(|(_, payload)| {
            audio_request(
                "jump",
                payload,
                &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata"),
            )
        });
    let req = req.expect("jump.wav fixture exists");
    assert_eq!(req.volume, 0.5);
    assert!(!req.loop_);
}

#[test]
fn delivered_payloads_is_empty_before_any_emit() {
    let bus = craft_kernel::SignalBus::new();
    assert!(bus.delivered_payloads().is_empty());
}
