# ADR 0012: System Registry — `craft_system!` Macro

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: PRD §6.4(d), §8.1 — Rust escape hatch for behaviors; `craft_system!` macro

## Context

The PRD's 9 action verbs cover common game logic, but some operations (pathfinding, complex math, external algorithms) require Rust-level implementation. The `call_system` action invokes a registered Rust function with typed arguments and an optional `result_in` destination.

Godot handles this via GDScript/C# scripting or GDExtension C ABI plugins. Craft's approach is simpler: Rust functions registered at compile time via a macro, surfaced through schema generation, callable from JSON behaviors.

## Decision

**A `craft_system!` proc-macro that registers a Rust function into a global `SystemRegistry`. Systems are pure by default; impure systems are explicitly marked and excluded from `dryRun`.**

### System Signature

```rust
// craft-kernel/src/system/mod.rs

/// Trait implemented by all registered systems
pub trait System: Send + Sync {
    /// Unique identifier (e.g., "pathfinding::astar")
    fn name(&self) -> &str;

    /// Human-readable description (surfaced in schema)
    fn description(&self) -> &str;

    /// Whether this system mutates engine state beyond its `result_in` target.
    /// Pure systems can run in dryRun; impure systems cannot.
    fn is_pure(&self) -> bool;

    /// JSON Schema for the system's arguments
    fn args_schema(&self) -> serde_json::Value;

    /// JSON Schema for the system's return type
    fn return_schema(&self) -> serde_json::Value;

    /// Execute the system. Arguments are already deserialized and validated.
    /// Pure systems: called during evaluate phase, have read-only tree access.
    /// Impure systems: called during apply phase, can push commands via ctx.commands.
    fn call(&self, args: serde_json::Value, ctx: &mut SystemContext) -> SystemResult;
}

pub struct SystemContext<'a> {
    pub tree: &'a SceneTree,
    pub rng: &'a mut RngState,
    pub resources: &'a ResourceRegistry,

    /// Only available for impure systems (None during evaluate phase).
    /// Impure systems push mutations here instead of mutating the tree directly.
    pub commands: Option<&'a mut Vec<ActionCommand>>,
}

pub type SystemResult = Result<serde_json::Value, SystemError>;

pub struct SystemError {
    pub code: String,
    pub message: String,
    pub suggestion: Option<String>,
}
```

### The `craft_system!` Macro

```rust
// Usage in project code (e.g., games/tower_defense/systems.rs)
use craft_kernel::craft_system;

craft_system!(
    /// Euclidean distance between two nodes.
    /// Pure: only reads components, no side effects.
    #[pure]
    fn distance_between(ctx, args: { from: NodeId, to: NodeId }) -> f64 {
        let from_pos = ctx.tree.get(args.from)
            .and_then(|n| n.get_vec2("position"))
            .unwrap_or_default();
        let to_pos = ctx.tree.get(args.to)
            .and_then(|n| n.get_vec2("position"))
            .unwrap_or_default();
        let dx = to_pos[0] - from_pos[0];
        let dy = to_pos[1] - from_pos[1];
        (dx * dx + dy * dy).sqrt()
    }
);

craft_system!(
    /// Spawn a wave of enemies along a path.
    /// Impure: creates new nodes.
    #[impure]
    fn spawn_wave(ctx, args: { template: String, count: u32, path: Vec<[f64; 2]> }) -> Vec<String> {
        let mut spawned = Vec::new();
        let commands = ctx.commands.as_mut().expect("impure system must have command buffer");
        for i in 0..args.count {
            let pos = args.path[i as usize % args.path.len()];
            let node_id = NodeId::new();
            spawned.push(node_id.to_string());
            commands.push(ActionCommand::SpawnNode {
                node_type: args.template.clone(),
                parent: None,   // root
                components: {
                    let mut comps = HashMap::new();
                    comps.insert("position".into(), ComponentValue::Vec2(pos));
                    comps
                },
            });
        }
        spawned.into()
    }
);
```

The macro generates:
1. A `System` trait implementation
2. JSON Schema for the args struct and return type (via `schemars`)
3. Registration into the global `SystemRegistry` (via `inventory` or `ctor` crate)
4. The function body with argument deserialization and return serialization

### System Registry

```rust
// craft-kernel/src/system/registry.rs
use std::collections::HashMap;

pub struct SystemRegistry {
    systems: HashMap<String, Box<dyn System>>,
}

impl SystemRegistry {
    pub fn register(&mut self, system: Box<dyn System>);
    pub fn get(&self, name: &str) -> Option<&dyn System>;
    pub fn list(&self) -> Vec<SystemInfo>;
    pub fn is_pure(&self, name: &str) -> bool;

    /// For schema generation: all systems accessible to the agent
    pub fn all_schemas(&self) -> serde_json::Value {
        // Returns { "system_name": { "args": {...}, "returns": {...}, "pure": bool } }
    }
}

pub struct SystemInfo {
    pub name: String,
    pub description: String,
    pub is_pure: bool,
    pub args_schema: serde_json::Value,
    pub return_schema: serde_json::Value,
}
```

### Integration with `call_system` Action

The behavior runtime handles `call_system` differently for pure vs. impure systems:

**Pure systems — evaluate phase (read-only, current tick)**:
1. Look up `system` name in `SystemRegistry`
2. Validate `args` against the system's args schema
3. Evaluate expression-based args (refs, arithmetic) against current state
4. Call `system.call(deserialized_args, ctx)` with `ctx.commands = None`
5. If `result_in` is specified, push `ActionCommand::SetComponent` into the command buffer
6. Return value is available for subsequent actions in the same tick (via `result_in`)

**Impure systems — apply phase (deferred to after evaluation)**:
1. During evaluate phase: validate args + resolve expressions → push `ActionCommand::CallSystemFn { system, resolved_args, result_target }` into command buffer
2. During apply phase: drain `CallSystemFn` from buffer → call `system.call(resolved_args, ctx)` with `ctx.commands = Some(&mut command_buffer)`
3. The system pushes its mutations (SpawnNode, DestroyNode, SetComponent) into the same command buffer
4. The apply loop continues draining — newly pushed commands are executed in the same apply pass
5. If `result_in` is specified, the return value is written after the system completes

**System calls during dry run**: Pure systems execute normally and their results are included in the dry-run diff. Impure systems are rejected with a `dryRun:system_must_be_pure` error — the agent must call pure systems only in hypothetical execution.

### Schema Exposure

`engine.getSchema()` returns under `systems`:

```json
{
  "systems": {
    "pathfinding::astar": {
      "description": "A* pathfinding between two nodes",
      "pure": true,
      "args": { "from": "vec2", "to": "vec2" },
      "returns": "array<vec2>"
    },
    "combat::spawn_wave": {
      "description": "Spawn a wave of enemies",
      "pure": false,
      "args": { "template": "string", "count": "u32", "path": "array<vec2>" },
      "returns": "array<string>"
    }
  }
}
```

## Rationale

1. **Closed vocabulary with escape hatch**: The 9 action verbs are a hard boundary. Systems are the only extension point. This keeps the agent's reasoning surface bounded — new verbs cannot be invented, but new systems can be added.

2. **Pure/impure distinction enables dryRun**: The agent can safely call `dryRun` with `call_system` actions only if the system is pure. This is enforced at registration time, not guessed at runtime.

3. **Schema is auto-generated from Rust types**: The `craft_system!` macro uses `schemars` to derive JSON Schema from the args struct. No manual schema writing — the Rust function signature IS the schema.

4. **No cross-language ABI**: Unlike GDExtension's C ABI, Craft's systems are Rust functions in the same binary. This eliminates ABI stability concerns, compatibility hashes, and FFI overhead.

## Godot Mapping

| Godot | Craft System Registry |
|-------|----------------------|
| GDExtension C ABI + compatibility hash | `craft_system!` macro (compile-time registration, same binary) |
| `ClassDB::bind_method()` + `MethodBind` | `SystemRegistry::register()` (auto-generated by macro) |
| GDScript `call()` → dynamic dispatch | `call_system` action → registry lookup → typed invocation |
| No pure/impure distinction | `#[pure]` / `#[impure]` attribute — enables dryRun safety |
| Manual method documentation | `/// doc comments` → `description` field in schema |
| Manual class registration in `.cpp` | `inventory` crate auto-collects all `craft_system!` registrations |
