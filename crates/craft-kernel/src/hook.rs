use std::any::Any;

use crate::Engine;

/// Extension point that lets external runtimes (e.g. craft-lua) participate
/// in the engine tick loop without kernel taking a dependency on them.
///
/// The host (project code) installs at most one hook via
/// `Engine::set_hook`. The kernel calls the hook at three points:
/// 1. `before_behaviors` — once per tick, after signal delivery and systems
///    but before JSON behaviors are evaluated. This is the L2 "Lua pre-pass"
///    window.
/// 2. `on_signal` — once per dispatched signal, after JSON on_signal handlers
///    fire (so a signal arriving in tick N reaches both JSON and Lua hooks
///    in tick N, before tick N+1).
/// 3. `on_spawn` — once per newly-spawned node, intended to fire `on_spawn`
///    Lua hooks for late-bound instances.
///
/// Hook implementations must be `'static` because the engine stores them
/// as `Box<dyn EngineHook>`. Trait objects are not `Send`-bounded because
/// the engine is single-threaded (ADR 0015).
pub trait EngineHook: Any {
    fn before_behaviors(&mut self, engine: &mut Engine);

    fn on_signal(&mut self, engine: &mut Engine, signal_name: &str, args: &serde_json::Value);

    fn on_spawn(&mut self, engine: &mut Engine, node_id: &str);
}
