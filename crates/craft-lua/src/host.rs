use craft_kernel::{Engine, EngineHook};

use crate::runtime::LuaRuntime;

/// Adapter that wraps a [`LuaRuntime`] and implements kernel's
/// [`EngineHook`] so that class hooks fire as part of the engine tick
/// loop. Install on an `Engine` via `engine.set_hook(Some(Box::new(
/// LuaEngineHook::new(runtime))))`.
///
/// Errors from Lua hooks are silently swallowed at the engine boundary.
/// Inspect them via the runtime directly during development:
/// `runtime.borrow_pending_errors()` (not exposed in L2; added in a
/// follow-up if logging becomes a concern).
pub struct LuaEngineHook {
    runtime: LuaRuntime,
}

impl LuaEngineHook {
    pub fn new(runtime: LuaRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime_mut(&mut self) -> &mut LuaRuntime {
        &mut self.runtime
    }
}

impl EngineHook for LuaEngineHook {
    fn before_behaviors(&mut self, engine: &mut Engine) {
        if let Err(e) = self.runtime.tick_pre_pass(engine) {
            eprintln!("[craft-lua] before_behaviors error: {e}");
        }
    }

    fn on_signal(&mut self, engine: &mut Engine, signal_name: &str, args: &serde_json::Value) {
        if let Err(e) = self.runtime.dispatch_signal(engine, signal_name, args) {
            eprintln!("[craft-lua] on_signal({signal_name}) error: {e}");
        }
    }

    fn on_spawn(&mut self, engine: &mut Engine, node_id: &str) {
        if let Err(e) = self.runtime.dispatch_spawn(engine, node_id) {
            eprintln!("[craft-lua] on_spawn({node_id}) error: {e}");
        }
    }
}
