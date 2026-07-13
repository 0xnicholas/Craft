# ADR 0016: Lua Scripting — First-Class Scripting Language

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: ADR 0003's single-tier behavior model (JSON-only). Introduces two-tier model.
**Reference**: Godot GDScript — equivalent capability with industry-standard language

## Context

ADR 0003 defined a closed-set JSON behavior system with 9 action verbs and 3 behavior primitives (state machine, on_tick, on_signal). This system is AI-native — structured, schema-validated, deterministic. But it is not a human-friendly scripting language. It lacks loops, variables, functions, closures, coroutines, metaprogramming, and module systems.

The human editor (ADR 0017+) needs a scripting experience equivalent to Godot's Script Editor. Godot solved this with GDScript, a custom language. Craft uses **Lua 5.5** — an industry-standard language already used in game development (World of Warcraft, Roblox/Luau, Factorio, Balatro, Core Keeper).

## Decision

**Lua 5.5 embedded via `mlua` v0.12 as a first-class scripting language, forming a two-tier behavior model with JSON behaviors.**

### Two-Tier Behavior Model

```
┌──────────────────────────────────────────────────┐
│  Tier 1: Lua scripts (full power, human path)    │
│  - Direct mutation (no command buffer)            │
│  - Complete engine API (any node, any system)     │
│  - Table-prototype OOP (class/enemy.lua)          │
│  - Coroutines (wait_ticks, parallel flows)        │
│  - LuaRocks packages (require "pathfinding")      │
│  - Executes before JSON behaviors in each tick    │
│  - Determinism: optional lock-on-record           │
├──────────────────────────────────────────────────┤
│  Tier 2: JSON behaviors (restricted, agent path)  │
│  - 9 closed verbs + command buffer                │
│  - Determinism guaranteed always                  │
│  - Schema-validated, agent-friendly               │
└──────────────────────────────────────────────────┘
```

A node can have **both** Lua and JSON behaviors. Execution order: Lua hooks fire first, then JSON rules. Lua's mutations are immediately visible to subsequent JSON rules in the same tick (same as GDScript `_process` → `_physics_process` ordering within a node).

### Lua Hook System

A single Lua file can handle multiple lifecycle hooks:

```lua
-- scripts/classes/enemy.lua
local Enemy = {}
Enemy.__index = Enemy

function Enemy.new(node)
    return setmetatable({ node = node }, Enemy)
end

function Enemy:on_tick()
    -- Direct mutation — no buffer, no serialization
    local player = engine.get_node("player")
    local dir = engine.call_system("pathfinding::direction_to", {
        from = self.node.position,
        to = player.position
    })
    self.node.position = self.node.position + dir * self.node.speed

    if vec2.distance(self.node.position, player.position) < 10 then
        engine.emit("enemy_nearby", { id = self.node.id, pos = self.node.position })
    end
end

function Enemy:on_signal(signal_name, args)
    if signal_name == "damage" then
        self.node.hp = self.node.hp - args.amount
        if self.node.hp <= 0 then
            engine.emit("enemy_died", { id = self.node.id })
            self.node:destroy()
        end
    end
end

function Enemy:on_spawn()
    -- Initialize state on node creation
    self.node.state = "spawning"
end
```

### Scene Integration

Nodes bind to Lua classes via a `lua_class` field:

```json
{
  "id": "enemy_1",
  "type": "Enemy",
  "lua_class": "scripts.classes.enemy",
  "components": {
    "position": [10, 20],
    "hp": 100,
    "speed": 2.0
  }
}
```

`lua_class` is optional. Without it, the node only runs JSON behaviors. With it, the engine:
1. Requires `scripts/classes/enemy.lua` into the Lua VM
2. Calls `Enemy.new(engine_node)` to create the Lua-side instance
3. Registers `on_tick`, `on_signal`, `on_spawn` hooks if the table has those functions
4. Calls hooks each tick alongside (before) JSON behavior evaluation

### Engine API (Lua Side)

Node properties use field syntax (not get/set methods), matching GDScript ergonomics:

```lua
-- Component access — direct field syntax
local pos = self.node.position          -- read
self.node.position = { x = 10, y = 20 } -- write
self.node.hp = self.node.hp - 10        -- read-modify-write

-- Cross-node access
local player = engine.get_node("player")
if player then
    local dist = vec2.distance(self.node.position, player.position)
end

-- Signal emission
engine.emit("damage_taken", { amount = 10, source = self.node.id })

-- Node lifecycle
engine.spawn("Enemy", nil, { position = { x = 0, y = 0 }, hp = 50 })
self.node:destroy()  -- marks for despawn (deferred to end of tick)

-- Rust system calls
local path = engine.call_system("pathfinding::astar", {
    from = self.node.position,
    to = target.position
})

-- Deterministic RNG (not math.random)
local x = engine.rng(0, 100)

-- Coroutines — non-blocking wait
engine.start_coroutine(function()
    self.node.state = "retreating"
    engine.wait_ticks(30)
    self.node.state = "patrolling"
end)

-- Module system
local pathfinding = require("lib.lua_pathfinding")
local vec2 = require("lib.vec2")
```

### Rust-Side Implementation (`craft-lua` crate)

```rust
// crates/craft-lua/src/lib.rs

use mlua::{Lua, Function, Table, UserData};

pub struct LuaRuntime {
    vm: Lua,
    scripts: HashMap<String, CompiledScript>,
    class_registry: HashMap<String, LuaClassDef>,
}

struct CompiledScript {
    bytecode: Vec<u8>,
    hooks: Vec<LuaHook>,
    source_path: String,
}

enum LuaHook { OnTick, OnSignal(Option<String>), OnSpawn }

pub struct LuaNodeInstance {
    pub lua_table: mlua::Table,     // the Enemy {} table, kept alive in VM
    pub hooks: Vec<LuaHook>,
}

impl LuaRuntime {
    pub fn new() -> Self {
        let vm = Lua::new();
        // Sandbox: disable dangerous stdlib
        vm.sandbox(true)?;
        // Register engine binding globals
        register_engine_api(&vm)?;
        Ok(Self { vm, scripts: HashMap::new(), class_registry: HashMap::new() })
    }

    pub fn load_class(&mut self, path: &str) -> EngineResult<LuaClassDef>;

    pub fn instantiate(&self, class_name: &str, node_id: NodeId)
        -> EngineResult<LuaNodeInstance>;

    pub fn call_hook(
        &self,
        instance: &LuaNodeInstance,
        hook: LuaHook,
        node: &Node,              // read-only access
        ctx: &TickContext,
    );
}
```

### Node Binding — `__index` / `__newindex` for Component Access

```rust
// crates/craft-lua/src/bindings/node.rs
// Node exposed as Lua userdata with metatable for field access

impl UserData for NodeRef {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, node| Ok(node.id.to_string()));
        fields.add_field_method_get("type", |_, node| Ok(node.type_name.clone()));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // __index: node.position → reads component
        methods.add_meta_method(MetaMethod::Index, |lua, node, key: String| {
            node.get_component_as_lua(lua, &key)
        });

        // __newindex: node.position = {10, 20} → writes component
        methods.add_meta_method(MetaMethod::NewIndex, |_, node, (key, value): (String, mlua::Value)| {
            node.set_component_from_lua(&key, value)
        });

        methods.add_method("destroy", |_, node, ()| {
            node.mark_destroyed();
            Ok(())
        });
    }
}
```

### Sandboxing

| Restriction | Mechanism |
|-------------|-----------|
| No filesystem access | Disable `io.*`, `os.execute`, `os.rename`, `os.remove`, `os.tmpname` |
| No process spawning | Disable `os.execute` |
| No loading external .dll/.so | Disable `package.loadlib` |
| No loading untrusted Lua files | Disable `dofile`, `loadfile`. Only `require` allowed for known paths |
| No debug API | Disable `debug.*` |
| Memory limit | `lua_setmemorylimit()` — per-tick allocation cap |
| Instruction limit | `lua_sethook` with instruction count hook — per-tick VM instruction cap |
| RNG | `math.random` replaced with engine-managed `engine.rng()` |

### Module Organization

```
games/tower_defense/
├── scene.json
├── scripts/
│   ├── classes/           # Node-bound Lua classes
│   │   ├── enemy.lua
│   │   ├── tower.lua
│   │   └── projectile.lua
│   ├── systems/           # Pure Lua computation (no engine dependency)
│   │   └── pathfinding.lua
│   └── lib/               # Shared libraries
│       ├── vec2.lua
│       └── table_utils.lua
├── behaviors/             # JSON behaviors (agent path)
│   └── spawner.json
└── craft.toml
```

### Determinism Mode

| Mode | Behavior | Use Case |
|------|----------|----------|
| **Development** | Lua full power: direct mutation, `math.random`, coroutines, `require` any package | Authoring, debugging |
| **Recording** | Optional lock: `engine.set_determinism(true)` → replaces `math.random` with `engine.rng()`, disables `os.clock()`, logs all engine API calls | Generate valid replay |
| **Replay** | Lua output ignored. Only recorded `ActionCommands` (from JSON + locked Lua) are replayed. | CI, benchmarks, agent evaluation |

This mirrors GDScript's philosophy — development speed > mathematical purity — but adds Craft's unique capability: **optional determinism when you need it for agents and CI**.

## PRD Deviation

PRD §3.2 lists "Lua / Python / GDScript-style embedded scripting" as a v1 non-goal. This ADR defines Lua for v1/v2 scope — the two-tier behavior model allows Lua to ship alongside JSON behaviors. The PRD's exclusion was based on the original single-tier thesis (JSON-only for AI agents). Adding Lua as a second tier preserves the agent path while expanding to human authors. See ADR 0003 for the updated two-tier tick loop.

## Rationale

1. **Lua is proven in gamedev**: World of Warcraft (Lua since 2004), Roblox (Luau), Factorio, Balatro, Core Keeper — all ship Lua scripting for game logic. It is the industry standard for embeddable game scripting.

2. **mlua supports Lua 5.5**: The latest Lua release (2025-12-15) with improved interpreter performance and GC. mlua v0.12 (2026-07-05) has first-class support via `features = ["lua55"]`.

3. **Field syntax → GDScript parity**: `node.position` instead of `node:get("position")` is not cosmetic. It makes Lua scripts read like GDScript, which is Godot's primary UX advantage. Achieved via `__index`/`__newindex` metatables on Node userdata.

4. **Two tiers serve two audiences**: JSON behaviors for AI agents (deterministic, schema-validated), Lua for humans (expressive, familiar). Both coexist per-node, same tick.

5. **LuaRocks ecosystem**: Unlike GDScript's closed ecosystem, Lua has 30 years of open-source packages via LuaRocks. Pathfinding, noise, serialization, ECS frameworks — all available via `require`.

6. **Coroutines > GDScript await**: Lua's native `coroutine.yield` + `engine.wait_ticks(N)` is more flexible than GDScript's `await get_tree().create_timer(N)`. Coroutines can be composed, nested, and inspected.

## Godot Mapping

| Godot | Craft Lua |
|-------|-----------|
| GDScript (custom language, 30+ files) | Lua 5.5 (standard language, `mlua` crate) |
| `_process(delta)` per node | `Enemy:on_tick()` per Lua instance |
| `_on_signal_name(args)` per node | `Enemy:on_signal(signal_name, args)` unified handler |
| `_ready()` / `_enter_tree()` | `Enemy:on_spawn()` |
| `extends Node2D` / `class_name Enemy` | `lua_class: "scripts.classes.enemy"` in JSON |
| `$Player.position` / `get_node("Player")` | `engine.get_node("player").position` |
| `emit_signal("hit", damage)` | `engine.emit("hit", { damage = 10 })` |
| `await get_tree().create_timer(1.0)` | `engine.wait_ticks(60)` inside coroutine |
| `Input.get_vector()` | `engine.get_node("_input").direction` |
| `print("debug")` | `engine.log("info", "debug", {})` |
| No package manager | LuaRocks (`require "pathfinding"`) |

## LuaRocks Dependency Locking

ADR 0010 Layer 3 requires "same seed → byte-identical" for agent benchmarks. LuaRocks packages resolved at different times can produce different versions, breaking reproducibility. The engine enforces a lockfile:

**`games/<name>/luarocks.lock`** — pinned dependency manifest:

```toml
# Auto-generated by `engine.lock_dependencies()`. Do not edit manually.
[packages]
lua-pathfinding = { version = "2.1.0", hash = "sha256:abc123..." }
vec2 = { version = "1.0.0", hash = "sha256:def456..." }
```

| Scenario | Behavior |
|----------|----------|
| Development (`engine.start()`) | Loads latest compatible version. Emits lint warning if lockfile exists but is stale (version mismatch). |
| Recording (`engine.start_recording()`) | **Requires lockfile**. If missing or stale → `EngineError::Validation` with suggestion: "Run engine.lock_dependencies() to pin versions." The lockfile is embedded in `RecordingMeta.resource_snapshots`. |
| Replay | Uses the lockfile embedded in the recording. Ignores the current filesystem lockfile. |
| Agent benchmark | Same as recording — lockfile required, embedded in benchmark session. |

**`engine.lock_dependencies()`**: Scans all Lua scripts in the project, resolves `require()` calls against LuaRocks, writes pinned versions + content hashes to `luarocks.lock`. Called once during project setup, re-run when dependencies are added/upgraded. Analogous to `cargo update` writing `Cargo.lock`.
| No determinism option | `engine.set_determinism(true)` for recording |

## Hot Reload & Lua Instance State

Hot reload (ADR 0009) preserves NodeId and component state across reloads. However, Lua instances hold **pure Lua-side state** — fields on the `self` table that are not engine components (e.g., `self.custom_timer = 0`, `self.cached_target = nil`, coroutine state, closure captures). These live in the Lua VM's heap, not in `node.components`.

| Hot reload operation | Component state | Pure Lua state | Behavior |
|----------------------|----------------|----------------|----------|
| Component value change | Preserved | Preserved | Lua `self` table untouched |
| Behavior rule change | Preserved | Preserved | Lua class file re-required; `self` table untouched |
| Lua class file edit (Ctrl+S) | Preserved | **Lost** — `Enemy.new(node)` called, new `self` table | The edited class file is re-required and all instances of that class are re-instantiated. Pure Lua state resets to constructor defaults. |
| Node type change | Preserved | **Lost** — in-place respawn (ADR 0002) re-initializes type | Type change is structural — the old Lua class is detached, new class attached. Old `self` table is garbage-collected. |

**Mitigation for Lua class file edits**: The hot reload diff should distinguish "Lua file content changed" from "node type changed." Standard practice:

1. If the `lua_class` field value is the same (still `"scripts.classes.enemy"`) but the file content changed → re-require the module, but **don't** re-instantiate existing instances. Lua's `package.loaded` cache is cleared so the next `require` loads new code, but existing `self` tables are kept.
2. If the `lua_class` field value changed (e.g., `"scripts.classes.enemy"` → `"scripts.classes.enemy_v2"`) → detach old class, attach new class, call `EnemyV2.new(node)`. Pure Lua state is legitimately reset because the type changed.

This means human authors can edit `enemy.lua`, save, and see their new logic take effect **without losing** `self.custom_timer` or active coroutines — matching Godot's GDScript hot reload behavior where `@export` variables survive script reload.
