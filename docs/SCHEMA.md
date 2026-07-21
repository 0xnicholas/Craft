# Schema Reference

Craft generates JSON Schema from Rust types at compile time. Every data structure the engine consumes or produces has a corresponding schema. `engine.getSchema()` returns the full schema; `engine.getActionSchema(verb)` returns per-verb schemas.

## Scene

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["kind", "name", "nodes"],
  "properties": {
    "kind": { "const": "scene" },
    "name": { "type": "string" },
    "nodes": {
      "type": "array",
      "items": { "$ref": "#/$defs/Node" }
    },
    "spawn_counter": { "type": "integer", "default": 0 }
  }
}
```

## Node

```json
{
  "$defs": {
    "Node": {
      "type": "object",
      "required": ["id", "type"],
      "properties": {
        "id": { "type": "string" },
        "type": { "type": "string" },
        "parent": { "type": "string" },
        "components": {
          "type": "object",
          "additionalProperties": { "$ref": "#/$defs/Component" }
        },
        "behaviors": {
          "type": "array",
          "items": { "$ref": "#/$defs/Behavior" }
        },
        "lua_class": { "type": "string" },
        "active_state": { "type": "string" }
      }
    }
  }
}
```

## Component

| JSON Type | ComponentType | Example |
|-----------|---------------|---------|
| `null` | `Nil` | `null` |
| `boolean` | `Bool` | `true` |
| integer | `Int` | `100` |
| number | `Float` | `0.5` |
| string | `String` | `"player.png"` |
| `[x, y]` | `Vec2` | `[10.0, 20.0]` |
| `[r, g, b]` | `Vec3` | `[1.0, 0.0, 0.0]` |
| `[x, y, w, h]` | `Rect` | `[0, 64, 32, 32]` |

### Structured Component (for transient)

```json
{
  "type": "object",
  "required": ["value"],
  "properties": {
    "value": { "$ref": "#/$defs/ComponentValue" },
    "kind": { "enum": ["regular", "transient"] },
    "default": { "$ref": "#/$defs/ComponentValue" }
  }
}
```

## Universal Components

These bypass per-node-type validation. Any node can use them:

`position`, `velocity`, `hitbox`, `hitbox_radius`, `sprite`, `sprite_rect`, `modulate`, `alpha`, `scale`, `rotation`, `z_index`, `visible`

## Behaviors

### on_tick

```json
{
  "kind": "on_tick",
  "actions": [{ "$ref": "#/$defs/Action" }]
}
```

### on_signal

```json
{
  "kind": "on_signal",
  "signal": "collide",
  "actions": [{ "$ref": "#/$defs/Action" }]
}
```

### state_machine

```json
{
  "kind": "state_machine",
  "states": ["idle", "moving"],
  "initial": "idle",
  "transitions": [
    {
      "from": "idle",
      "event": "tick",
      "to": "moving",
      "guard": { "$ref": "#/$defs/Expression" }
    }
  ]
}
```

## Actions (9 verbs)

| Verb | Required Fields | Optional |
|------|----------------|----------|
| `move` | `target`, `key`, `by` | — |
| `set_state` | `target`, `key`, `value` | — |
| `emit` | `signal` | `args` |
| `destroy` | `target` | — |
| `spawn` | `type`, `components` | `parent`, `behaviors` |
| `if` | `cond`, `then` | `else` |
| `animate` | `target`, `key`, `to`, `duration` | — |
| `log` | `message` | `level`, `fields` |
| `call_system` | `system`, `args` | `result_in` |

### Target

| Value | Meaning |
|-------|---------|
| `"self"` | Current node |
| `"parent"` | Parent node |
| `{"kind": "node", "id": "..."}` | Specific node by ID |
| `{"kind": "children", "filter": "type"}` | All children, optionally filtered |

## Expressions

| Operator | Schema |
|----------|--------|
| `ref` | `{"ref": "node.component"}` |
| `eq` | `{"eq": [expr, expr]}` |
| `neq` | `{"neq": [expr, expr]}` |
| `lt` | `{"lt": [expr, expr]}` |
| `gt` | `{"gt": [expr, expr]}` |
| `add` | `{"add": [expr, expr]}` |
| `sub` | `{"sub": [expr, expr]}` |
| literal | a JSON number, string, or bool |

Bare strings in expression position are shorthand for `ref`: `"self.health"` ≡ `{"ref": "self.health"}`.

## Signals

| Signal | Source | When |
|--------|--------|------|
| `tick` | Kernel | Every tick, before behaviors |
| `collide` | PhysicsSystem | Two hitboxes overlap |
| `hot_reload` | Kernel | After successful hot reload |
| `replay_start` | Replay | Replay begins |
| `replay_end` | Replay | Replay ends |

User signals (e.g., `tower_fire`, `enemy_killed`, `level_complete`) are defined in scene JSON and emitted via `emit` actions.

## Systems

Registered Rust functions callable via `call_system`. Discovered via `engine.listSystems()`.

| System | Phase | Purpose |
|--------|-------|---------|
| `PhysicsSystem` | PreTick | Velocity integration + collision detection |
| `ParticleSystem` | PostTick | Emitter lifecycle + particle update |
| `AudioSystem` | PostTick | Signal-triggered audio playback |

## SDK Schema Access

```typescript
// Full schema (all types, verbs, systems)
const schema = await engine.getSchema();

// Per-type
await engine.getNodeTypeSchema('Player');

// Per-verb
await engine.getActionSchema('spawn');

// Per-system  
await engine.getSystemSchema('pathfinding::astar');
```

## Schema Generation

Schema is generated from Rust types via `schemars` + `craft-schema`. Adding a new node type via `craft_node!` automatically registers it and generates its JSON Schema fragment. The agent's view of the API can never drift from the engine implementation.
