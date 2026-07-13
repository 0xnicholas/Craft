# ADR 0013: Input Model

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: PRD §6.1, §7.1 — built-in `Input` node + `InputBus`

## Context

The PRD defines an `Input` node as a built-in singleton that exposes the current input frame as components (`input.direction`, `input.action`, etc.). The engine's input layer populates this each tick before behavior evaluation. This is the bridge between the external world (player keyboard, AI agent actions) and the game state.

Godot's input system uses `Input` singleton + `_input(event)` callbacks + `InputMap` for action binding. Each node can independently poll or subscribe to input events. This distributed model creates non-deterministic ordering and makes replay difficult.

## Decision

**A single `Input` node populated by `InputBus` at the start of each tick. All input is available as components on this node. Behaviors read input via `ref` expressions or `on_signal` handlers.**

### Input Node Schema

```json
{
  "type": "Input",
  "id": "_input",
  "components": {
    "direction": { "value": [0, 0], "default": [0, 0] },
    "action":    { "value": "none", "default": "none" },
    "actions":   { "value": [], "default": [] },
    "mouse_pos": { "value": [0, 0], "default": [0, 0] },
    "mouse_down": { "value": false, "default": false }
  }
}
```

### InputBus

```rust
// craft-kernel/src/input.rs

pub struct InputBus {
    direction: (i8, i8),         // normalized: (-1,0,1) × (-1,0,1)
    primary_action: String,      // the "current" action ("click", "pause", etc.)
    active_actions: HashSet<String>,  // all actions held this frame
    mouse_pos: (i32, i32),
    mouse_clicked: bool,
}

impl InputBus {
    /// Called by the bridge layer (agent) or terminal (keyboard).
    pub fn set_direction(&mut self, dx: i8, dy: i8);
    pub fn press_action(&mut self, action: &str);
    pub fn release_action(&mut self, action: &str);
    pub fn set_mouse(&mut self, x: i32, y: i32, clicked: bool);

    /// Called at the start of each tick to populate the Input node.
    pub fn populate(&self, tree: &mut SceneTree) {
        if let Some(input_node) = tree.get_mut(INPUT_NODE_ID) {
            input_node.set_component("direction", ComponentValue::Vec2Int([self.direction.0 as i64, self.direction.1 as i64]));
            input_node.set_component("action", ComponentValue::String(self.primary_action.clone()));
            input_node.set_component("actions", ComponentValue::StringArray(self.active_actions.iter().cloned().collect()));
            input_node.set_component("mouse_pos", ComponentValue::Vec2Int([self.mouse_pos.0 as i64, self.mouse_pos.1 as i64]));
            input_node.set_component("mouse_down", ComponentValue::Bool(self.mouse_clicked));
        }
    }

    /// Clear per-frame state after tick (actions are frame-latched)
    pub fn end_frame(&mut self);
}
```

### Behavior Integration

Agents reference input in behaviors using ref expressions:

```json
{
  "kind": "on_tick",
  "actions": [
    {
      "kind": "if",
      "cond": { "neq": ["_input.direction", [0, 0]] },
      "then": [
        { "kind": "move", "target": "self", "key": "position", "by": "_input.direction" }
      ]
    },
    {
      "kind": "if",
      "cond": { "eq": ["_input.action", "click"] },
      "then": [
        { "kind": "emit", "signal": "player_click" }
      ]
    }
  ]
}
```

Or via signal handlers (for discrete actions):

```json
{
  "kind": "on_signal",
  "signal": "player_click",
  "actions": [
    { "kind": "spawn", "type": "Projectile", "parent": "world", "components": { "position": "_input.mouse_pos" } }
  ]
}
```

### Input Sources

| Source | How InputBus is fed | Used for |
|--------|---------------------|----------|
| Terminal keyboard | `craft-terminal` calls `InputBus::set_direction()` / `press_action()` on key events | Reference game, manual play |
| Agent (RPC) | `engine.setInput({ direction, action })` via NAPI | Agent-driven testing, benchmarks |
| Replay | `ReplayRunner` feeds recorded `InputFrame` into `InputBus` each tick | Deterministic replay |

## Rationale

1. **Single source of truth**: All input lives on one node. Behaviors reference it uniformly. No distributed `_input(event)` callbacks with unknown execution order.

2. **Deterministic replay**: Recording captures the populated `InputFrame` per tick. Replay feeds the exact same input into `InputBus` before the tick. Input is never "lost" or reordered.

3. **Agent-accessible input**: The agent can call `engine.setInput(...)` to simulate keyboard/mouse. This is how benchmarks drive the engine without a terminal.

4. **Frame-latched**: Input state persists for the entire tick. All behaviors see the same input frame. No "first behavior processes the key event, second behavior misses it."

## Godot Mapping

| Godot | Craft Input |
|-------|------------|
| `Input` singleton (global access) | `InputBus` → `Input` node components (same data, tree-accessible) |
| `_input(event)` callback on each node | `on_tick` reading `_input.action` or `on_signal` handlers |
| `InputMap` (action → key binding) | Terminal key → `InputBus::press_action()` mapping (hardcoded for v1) |
| `Input.get_vector()` polling | `{ "ref": "_input.direction" }` expression |
| `Input.is_action_just_pressed()` | `{ "eq": ["_input.action", "click"] }` + frame-latched |
| No replay of input | `InputFrame` per tick in recording → deterministic replay |
