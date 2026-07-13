# ADR 0003: Behavior Runtime — Two-Tier Model with Command Buffer

**Date**: 2026-07-13
**Status**: Accepted
**Updated**: 2026-07-13 — Added two-tier model with Lua scripting (see ADR 0016)
**Supersedes**: Godot's `_process`/`_physics_process` direct-mutation callbacks

## Context

The PRD defines three behavior primitives: state machines, tick rules (`on_tick`), and signal handlers (`on_signal`). Behaviors compose a closed set of **9 action verbs** (PRD §6.5). The execution model must support both human-authored Lua scripts and agent-authored JSON rules.

Two-tier behavior model (see ADR 0016):
- **Tier 1**: Lua scripts — full power, direct mutation, GDScript parity
- **Tier 2**: JSON behaviors — 9 verbs, command buffer, deterministic guarantee

Both tiers coexist per-node. Execution order: Lua hooks → JSON rules.

## Decision

**Three-phase tick with Lua pre-pass: Lua hooks (direct mutation) → JSON evaluate (read-only) → flush signals → apply (write)**.

```
tick N:
  1. InputBus populates Input node's components
  2. Fire reserved "tick" signal
  3. LUA PRE-PASS (direct mutation):
     For each node with a lua_class, in declaration order:
       a. Call lua_instance:on_tick()    — reads/writes components directly
       b. Call lua_instance:on_signal()  — for signals queued before this tick
     Lua mutations are immediately visible to subsequent steps in the same tick.
  4. JSON EVALUATE phase (READ-ONLY):
     For each node in declaration order:
       a. State machine: evaluate transitions triggered by tick/queued signals
       b. on_tick rules: evaluate actions against current state (including Lua mutations)
       c. on_signal handlers: evaluate for signals queued before this tick
          (ordered by signal_name lexicographic, then by subscription registration order)
     All JSON actions produce ActionCommand values → pushed to command_buffer
  5. FLUSH signals emitted during JSON evaluation (enqueue for next tick)
  6. APPLY phase (WRITE):
     Drain command_buffer, apply mutation commands to SceneTree
     Note: Lua mutations were already applied in step 3. JSON mutations applied here.
  7. Transient component lifecycle: decrement counters, emit .done signals
  8. RENDER phase: read final state, produce output frame
```

### Action Vocabulary (per PRD §6.5)

```rust
/// The 9 closed-set action verbs. Agents cannot invent new verbs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Action {
    /// Write a component on the target node.
    /// Writing to a transient component restarts its lifetime.
    #[serde(rename = "set_state")]
    SetState {
        target: Target,
        key: String,
        value: serde_json::Value,     // literal or expression (see ADR 0011)
    },

    /// Fire a signal (queued; resolved next tick).
    #[serde(rename = "emit")]
    Emit {
        signal: String,
        args: HashMap<String, serde_json::Value>,
    },

    /// Remove a node. Always succeeds.
    /// References to the destroyed node resolve to `none` at access time.
    #[serde(rename = "destroy")]
    Destroy {
        target: Target,
    },

    /// Create a new node at runtime.
    #[serde(rename = "spawn")]
    Spawn {
        #[serde(rename = "type")]
        node_type: String,
        parent: Target,
        components: HashMap<String, serde_json::Value>,
    },

    /// Conditional execution. May be nested arbitrarily deep.
    #[serde(rename = "if")]
    If {
        cond: Expression,
        then: Vec<Action>,
        #[serde(rename = "else")]
        else_: Option<Vec<Action>>,
    },

    /// Add a delta to any numeric component (not position-specific).
    #[serde(rename = "move")]
    Move {
        target: Target,
        key: String,
        by: serde_json::Value,        // literal or expression
    },

    /// Interpolate any component to a target value over N ticks.
    #[serde(rename = "animate")]
    Animate {
        target: Target,
        key: String,
        to: serde_json::Value,
        duration: u32,                // ticks
    },

    /// Debug output. Surfaces in agent subscription stream.
    #[serde(rename = "log")]
    Log {
        level: LogLevel,
        message: String,
        fields: HashMap<String, serde_json::Value>,
    },

    /// Invoke a registered Rust system.
    #[serde(rename = "call_system")]
    CallSystem {
        system: String,
        args: HashMap<String, serde_json::Value>,
        result_in: Option<ResultTarget>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResultTarget {
    pub key: String,
    pub on: Target,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Target {
    #[serde(rename = "self")] This,
    #[serde(rename = "node")] Node { id: String },
    #[serde(rename = "parent")] Parent,
    #[serde(rename = "children")] Children { filter: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum LogLevel { Debug, Info, Warning, Error }
```

### Internal Action Commands (evaluate phase output)

```rust
/// Produced during evaluation, consumed during the apply phase.
pub enum ActionCommand {
    SetComponent   { node: NodeId, key: String, value: ComponentValue },
    EmitSignal     { signal: String, args: HashMap<String, Value>, source: NodeId },
    SpawnNode      { node_type: String, parent: Option<NodeId>, components: HashMap<String, ComponentValue> },
    DestroyNode    { node: NodeId },
    StartAnimation { node: NodeId, key: String, from: ComponentValue, to: ComponentValue, remaining: u32 },
    LogEntry       { level: LogLevel, message: String, fields: HashMap<String, Value> },
    CallSystemFn   { system: String, args: HashMap<String, Value>, result_target: Option<(NodeId, String)> },
}
```

## Cross-Node `set_state` Invariant

PRD §7.4 states: "Node A cannot directly `set_state` node B's component." The rationale is to force explicit dependencies via signals. However, PRD §6.4(c) shows a signal handler on a node setting `"target": "player"` (cross-node write within a signal handler).

**Resolution**: The invariant applies to `on_tick` rules only. Signal handlers may write to other nodes because the signal represents an explicit dependency contract (the emitting node "asked" the handler to react). The behavior runtime enforces this at validation time:

- `on_tick` rule actions: `target` must be `"self"` or `"children"` (never another named node)
- `on_signal` handler actions: `target` may reference any node
- State machine transition actions within `on_tick`-triggered transitions: same restriction as `on_tick`
- State machine transition actions within signal-triggered transitions: same freedom as `on_signal`

## Rationale

1. **Determinism**: No behavior can observe mutations from another behavior within the same tick. The evaluation phase sees a frozen state snapshot.

2. **No reentrancy**: Signals emitted during a tick are queued for the *next* tick. A signal handler cannot trigger a cascade that re-enters the evaluation loop.

3. **Agent reasoning**: The mental model is "state before → actions decided → state after." Agents can call `dryRun()` to predict component diffs without side effects.

4. **Hot reload safety**: The evaluate → apply boundary is a natural pause point. If a file change is detected during evaluation, the engine defers application until after reload.

5. **Tick ordering is explicit and documented**: Node order = declaration order in scene JSON. Within a node: state machine → on_tick rules → on_signal handlers (lexicographic by signal_name, then by subscription order). Within an action list: declaration order. Actions within the same list see prior actions' effects.

6. **`animate` is a first-class verb, not a composite**: Godot's Tween node requires multiple setup calls. Craft's `animate` is a single action — the engine manages interpolation internally across ticks. This matches the AI-native thesis: one declarative action, not multiple procedural steps.

## Godot Mapping

| Godot Pattern | Craft Replacement |
|---------------|-------------------|
| `_process(delta)` virtual function | Lua `on_tick()` (Tier 1) OR JSON `on_tick` rule (Tier 2) |
| `_on_signal_name(args)` per node | Lua `on_signal(signal_name, args)` (Tier 1) OR JSON `on_signal` handler (Tier 2) |
| `emit_signal("name", args...)` — synchronous | `engine.emit("name", args)` in Lua OR `emit` JSON action |
| `set_position()` — immediate mutation | Lua: `node.position = {10,20}` (direct). JSON: `set_state` → command buffer |
| `Tween` node (multi-step setup) | JSON `animate` action (declarative) |
| `print()` / `print_debug()` | `engine.log("info", msg)` in Lua OR JSON `log` action |
| Manual state machine (enum + switch) | JSON `state_machine` behavior primitive |
| `_input(event)` / `Input.get_vector()` | `InputBus` → `Input` node components |
| GDScript for game logic | Lua 5.5 (Tier 1) for humans, JSON 9 verbs (Tier 2) for agents |
| GDScript `await` / `yield` | Lua coroutines + `engine.wait_ticks(N)` + `engine.start_coroutine(fn)` |
| `extends Node2D` / `class_name Enemy` | `lua_class: "scripts.classes.enemy"` in scene JSON |
