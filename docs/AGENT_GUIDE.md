# Agent Guide — Building Games with Craft

Craft is an AI-native game engine. Every piece of game content is **structured JSON** — scenes, nodes, components, behaviors, and signals. No compilation. No binary formats. An LLM with the TypeScript SDK can build, modify, and reason about a game entirely through JSON.

## Quick Start

```typescript
import { Engine } from 'craft-sdk';

const engine = new Engine();
await engine.start('res://games/tower_defense/scene.json');

// Inspect
const schema = await engine.getSchema();
const playerHP = await engine.getComponent('player', 'health');

// Modify
await engine.setComponent('player', 'health', 150);

// Observe
engine.subscribe('enemy_killed', (e) => console.log('kill:', e));
```

## Core Concepts

### 1. Nodes = JSON Objects

A node is a typed entity with an ID, components, and behaviors. No inheritance — type differences are expressed through component keys.

```json
{
  "id": "player",
  "type": "Player",
  "components": {
    "position": [10, 10],
    "health": 100,
    "damage": 15,
    "hitbox": [0.5, 0.5]
  },
  "behaviors": [
    { "kind": "on_tick", "actions": [...] }
  ]
}
```

### 2. Components = Typed Key-Value Pairs

| Type | JSON | Example |
|------|------|---------|
| Int | number | `100` |
| Float | number | `0.5` |
| Bool | boolean | `true` |
| String | string | `"sprites/player.png"` |
| Vec2 | `[x, y]` | `[10, 20]` |
| Vec3 | `[r, g, b]` | `[1.0, 0.0, 0.0]` |
| Rect | `[x, y, w, h]` | `[0, 64, 32, 32]` |

### 3. Behaviors = Structured Rules

Three primitive types, all expressed as JSON:

**on_tick** — runs every frame:
```json
{
  "kind": "on_tick",
  "actions": [
    { "kind": "move", "target": "self", "key": "cooldown", "by": 1 },
    { "kind": "if", "cond": { "gt": [{"ref": "self.cooldown"}, 10] },
      "then": [{ "kind": "emit", "signal": "fire", "args": {} }]
    }
  ]
}
```

**on_signal** — reacts to events:
```json
{
  "kind": "on_signal",
  "signal": "collide",
  "actions": [
    { "kind": "move", "target": "self", "key": "health", "by": -5 },
    { "kind": "destroy", "target": "self" }
  ]
}
```

**state_machine** — finite states with guarded transitions:
```json
{
  "kind": "state_machine",
  "states": ["idle", "chasing", "dead"],
  "initial": "idle",
  "transitions": [
    { "from": "idle", "event": "player_spotted", "to": "chasing" },
    { "from": "*", "event": "collide", "to": "dead" }
  ]
}
```

### 4. Actions — 9 Closed Verbs

| Verb | Purpose | Example |
|------|---------|---------|
| `move` | Add delta to a numeric component | `move cooldown by 1` |
| `set_state` | Set component to a value | `set velocity to [1, 0]` |
| `emit` | Fire a signal (delivered next tick) | `emit "explosion"` |
| `destroy` | Remove node | `destroy self` |
| `spawn` | Create new node at runtime | `spawn Enemy at parent` |
| `if` | Conditional (then/else) | `if health < 0: destroy self` |
| `animate` | Interpolate component over N ticks | `animate position to [10,0] in 20 ticks` |
| `log` | Debug output | `log "hit!"` |
| `call_system` | Invoke registered Rust function | `call_system pathfinding::astar` |

### 5. Expressions — Structured Comparisons and Math

| Operator | JSON | Meaning |
|----------|------|---------|
| `ref` | `{"ref": "node.component"}` | Read component value |
| `eq` | `{"eq": [a, b]}` | `a == b` |
| `neq` | `{"neq": [a, b]}` | `a != b` |
| `lt` | `{"lt": [a, b]}` | `a < b` |
| `gt` | `{"gt": [a, b]}` | `a > b` |
| `add` | `{"add": [a, b]}` | `a + b` |
| `sub` | `{"sub": [a, b]}` | `a - b` |

### 6. Signals — Next-Tick Event Bus

Signals emitted in tick N fire handlers in tick N+1. This guarantees deterministic ordering and prevents reentrancy.

Reserved signals: `tick`, `collide` (physics), `hot_reload`

```json
// Emit
{ "kind": "emit", "signal": "game_over", "args": { "score": 100 } }

// Subscribe
{ "kind": "on_signal", "signal": "game_over", "actions": [...] }
```

Targeted dispatch: signals with `{a, b}` payload only fire handlers on those specific nodes (used by physics).

### 7. Spawning at Runtime

```json
{
  "kind": "spawn",
  "type": "Enemy",
  "parent": "self",
  "components": {
    "position": [10, 0],
    "health": 30,
    "velocity": [-0.5, 0],
    "hitbox": [0.5, 0.5]
  },
  "behaviors": [
    { "kind": "on_signal", "signal": "collide", "actions": [
      { "kind": "destroy", "target": "self" }
    ]}
  ]
}
```

### 8. Lua Scripting (for Complex Logic)

When the 9 verbs aren't enough, attach a Lua class:

```json
{
  "id": "boss",
  "type": "Enemy",
  "lua_class": "scripts.boss"
}
```

```lua
-- scripts/boss.lua
Boss = Boss or {}
function Boss:on_tick()
    -- full engine API: self.node.position, engine.find_node(...)
end
function Boss:on_signal(name, args)
    if name == "collide" then
        self.node.health = self.node.health - 10
    end
end
```

### 9. Physics Components

Add these to any node for automatic physics:

| Component | Type | Effect |
|-----------|------|--------|
| `velocity` | Vec2 | Per-tick displacement |
| `hitbox` | Vec2 | AABB half-extents for collision |
| `hitbox_radius` | Float | Circle collision radius |

Collisions emit `{"signal": "collide", "a": "node_a", "b": "node_b"}` which triggers targeted `on_signal` handlers.

### 10. Particle Emitters

Spawn a burst emitter:

```json
{
  "id": "explosion",
  "type": "ParticleEmitter",
  "components": {
    "position": [5, 3],
    "emit_rate": 1,
    "particles_per_burst": 12,
    "particle_lifetime": 15,
    "emitter_lifetime": 3,
    "modulate": [1.0, 0.5, 0.0]
  }
}
```

### 11. Audio

Place `.wav` files in `assets/` named after signals:

```
assets/collide.wav     → plays on every "collide" signal
assets/enemy_killed.wav → plays on every "enemy_killed" signal
```

The engine auto-discovers and plays them. No code changes needed.

## Best Practices

1. **Start with `engine.getSchema()`** — let the engine describe itself. Never hardcode API knowledge.
2. **Write scenes, not code** — behaviors are JSON data. Let the engine interpret them.
3. **Use `dryRun` before writing** — test actions hypothetically before committing.
4. **Check `lint` output** — static analysis catches unreachable states, missing references, broken paths.
5. **Leverage hot reload** — edit scene.json, see effect in <100ms. No restart.
6. **Use replay for debugging** — record a session, replay it, diff any two ticks.
