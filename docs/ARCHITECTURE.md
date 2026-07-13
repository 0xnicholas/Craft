# Craft Architecture

**Status**: Draft
**Date**: 2026-07-13
**Reference**: Godot 4.x source (_references/godot-master/)

## Overview

Craft is an AI-native game engine implemented in Rust. The core thesis: a game engine whose architecture is designed so that AI agents (LLMs) can read, modify, and reason about game logic as first-class operations. Godot's architecture (scene tree, signals, resources, server abstraction) serves as the primary design reference, reimagined through Rust idioms and the AI-native constraint.

Three subsystems, built in dependency order:

| Subsystem | Milestone | Crate(s) | ADRs |
|-----------|----------|----------|------|
| **Engine Core** | v1 | `craft-kernel`, `craft-schema`, `craft-replay`, `craft-bridge`, `craft-terminal` | 0001-0015 |
| **Lua Scripting** | v1 | `craft-lua` | 0016 |
| **Editor** | v2 | `craft-editor` | 0017-0021 |

## System Architecture

```
┌──────────────────────────────────────────────────────────┐
│  Agent (LLM)                    │  Human (v2 editor)     │
│  TypeScript SDK                  │  egui desktop app      │
│  sync NAPI ← JSON-RPC            │  direct fn calls       │
└──────────────┬───────────────────┴──────────┬─────────────┘
               │                              │
┌──────────────▼──────────────────────────────▼─────────────┐
│  craft-bridge                     craft-editor (v2)       │
│  NAPI bindings                    egui + eframe           │
│  JSON-RPC dispatcher              embedded engine         │
└──────────────┬───────────────────────────────────────────┘
               │
┌──────────────▼───────────────────────────────────────────┐
│  craft-kernel (engine core)                               │
│  ┌─────────┐ ┌────────┐ ┌──────────┐ ┌───────────────┐  │
│  │ scene   │ │ signal │ │ behavior │ │ resource      │  │
│  │ (graph) │ │ (bus)  │ │ (runtime)│ │ (registry)    │  │
│  └─────────┘ └────────┘ └──────────┘ └───────────────┘  │
│  ┌─────────┐ ┌────────────┐ ┌────────┐ ┌────────────┐   │
│  │ system  │ │ hot_reload │ │ lint   │ │ input      │   │
│  │ (macro) │ │ (diff)     │ │ (stat.)│ │ (bus)      │   │
│  └─────────┘ └────────────┘ └────────┘ └────────────┘   │
└──────────────┬───────────────────────────────────────────┘
               │
┌──────────────▼───────────────────────────────────────────┐
│  craft-lua (Lua 5.5 via mlua)                             │
│  Two-tier behavior runtime: Lua scripts + JSON behaviors  │
└──────────────────────────────────────────────────────────┘
               │
┌──────────────▼──────────────┐ ┌──────────────────────────┐
│  craft-terminal             │ │  craft-replay            │
│  ANSI terminal renderer     │ │  recording + replay      │
│  impl Render trait          │ │  deterministic hash      │
└─────────────────────────────┘ └──────────────────────────┘
               │
┌──────────────▼──────────────┐
│  craft-schema               │
│  JSON Schema from Rust types│
│  schemars + Craft extensions│
└─────────────────────────────┘
```

## Crate Dependency DAG

```
craft-terminal ──→ craft-kernel   (impl Render trait)
craft-replay   ──→ craft-kernel   (reads scene, feeds ticks)
craft-lua      ──→ craft-kernel   (Lua VM + engine bindings)
craft-bridge   ──→ craft-kernel + craft-schema + craft-lua
craft-editor   ──→ craft-kernel + craft-lua + craft-schema
craft-schema   ──→ (no kernel dep; schemars + proc-macros only)
```

All crates are `Send + Sync` safe. v1 is single-threaded by design (determinism invariant).

## Key Architectural Patterns

### 1. Property-Bag Node Model (no inheritance)

Godot uses deep OOP inheritance (`Node → CanvasItem → Node2D → Sprite2D`) with ClassDB reflection. Craft uses a single `Node` struct where type differences are expressed as component key sets. Schema validation at load time replaces ClassDB runtime reflection.

```rust
pub struct Node {
    pub id: NodeId,                    // generational index (SlotMap)
    pub type_name: String,             // "Player", "Enemy", "Tower"
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub components: HashMap<String, Component>,
    pub behaviors: Vec<Behavior>,
    pub lua_class: Option<String>,     // "scripts.classes.enemy"
}
```

### 2. Two-Tier Behavior Model

| Tier | Author | Execution | Determinism |
|------|--------|-----------|-------------|
| **Lua scripts** (Tier 1) | Human | Direct mutation, coroutines, full engine API | Optional lock-on-record |
| **JSON behaviors** (Tier 2) | Agent | 9 closed verbs → command buffer → apply | Always guaranteed |

Tick order: Lua hooks fire first (direct mutation), then JSON rules evaluate (read-only → command buffer → apply). Lua mutations are visible to JSON rules in the same tick.

### 3. Command Buffer Pipeline (JSON behaviors only)

```
tick N:
  1. InputBus → populate Input node
  2. Fire "tick" signal
  3. LUA PRE-PASS (direct mutation)
  4. JSON EVALUATE phase (READ-ONLY) → produce ActionCommands
  5. FLUSH signals → next-tick queue
  6. APPLY phase (WRITE) → drain buffer, apply mutations
  7. Transient component lifecycle
  8. RENDER phase
```

Signals emitted during tick N do not fire handlers until tick N+1. This prevents reentrancy and guarantees deterministic ordering.

### 4. Minimal Render Trait (4 methods)

Godot's `RenderingServer` is 100+ methods across 180K lines. Craft v1 has a single ANSI terminal backend. The trait is deliberately minimal:

```rust
pub trait Render: Send {
    fn render(&mut self, components: &[ComponentView], tick: u64);
    fn viewport(&self) -> Viewport;
    fn resize(&mut self, viewport: Viewport);
    fn shutdown(&mut self);
}
```

Extends when v2 adds 2D/3D backends. `NullRenderer` enables headless testing.

### 5. Schema as First-Class Product

Every Rust type (actions, node definitions, component schemas, system signatures) emits JSON Schema via `schemars` + `craft-schema` extensions. TypeScript SDK types are auto-generated. The agent's API view cannot drift from engine reality.

### 6. Deterministic Replay

`craft-replay` records per-tick state hashes + input frames. Replay re-runs with the same seed + scene snapshot and verifies hash equality. Any mismatch is a determinism bug — tracked and fixed before v1 ships.

### 7. Structured Errors (agent-consumable)

All engine errors are JSON with `file`, `json_path`, `expected_type`, `actual_value`, and `suggestion` fields. Validation collects all errors before returning (bulk error, not first-error-abort). Errors are machine-readable — the agent can auto-correct.

## Godot Architecture Comparison

| Godot | Craft | Lines (Godot → Craft) |
|-------|-------|----------------------|
| `core/` — Variant, Object, ClassDB, containers, IO | `craft-kernel` — no Variant/Object; math inline; property-bag Node | ~170K → ~15K |
| `scene/` — Node hierarchy, 2D/3D, GUI, animation | `craft-kernel::scene/` — unified Node, component-based | ~333K → ~10K |
| `servers/rendering/` — RenderingServer (100+ methods) | `Render` trait (4 methods) | ~180K → ~0.3K |
| `drivers/` — Vulkan, Metal, D3D12, GLES3 | `craft-terminal/` — ANSI only | ~87K → ~0.5K |
| `editor/` — Godot Editor IDE | `craft-editor/` — egui desktop app (v2) | ~333K → ~15K |
| `modules/gdscript/` — custom scripting language | `craft-lua/` — Lua 5.5 via mlua | ~30K → ~5K |
| `modules/mono/` — C# embedding | None | ~25K → 0 |
| `platform/` — Windows, macOS, Linux, Android, iOS, Web | Node.js (NAPI host) | ~93K → 0 |
| `servers/physics*/`, `audio/`, `navigation*/`, `xr/` | v1 excluded (non-goals) | ~60K → 0 |
| `thirdparty/` — 69 C/C++ libraries | ~6 cargo deps | ~673 cpp + 2364 h → 6 toml |
| No replay system | `craft-replay/` | 0 → ~3K |
| No agent bridge | `craft-bridge/` | 0 → ~3K |
| No schema pipeline | `craft-schema/` | 0 → ~2K |

## ADR Index

| ADR | Topic | Key Decision |
|-----|-------|-------------|
| 0001 | Crate Structure | 6-crate workspace: kernel, lua, schema, replay, bridge, terminal |
| 0002 | Node Model | Property-bag + tree; no inheritance; SlotMap generational indices |
| 0003 | Behavior Runtime | Two-tier: Lua direct + JSON command buffer; evaluate → apply pipeline |
| 0004 | Render Trait | 4-method trait; ComponentView iterator; NullRenderer for testing |
| 0005 | Schema Pipeline | schemars + craft-schema extensions; JSON Schema → TS types |
| 0006 | Replay System | Per-tick hash recording; deterministic re-run; resource snapshots |
| 0007 | Bridge Layer | Sync NAPI + JSON-RPC dispatcher; transport trait for v2 WebSocket |
| 0008 | Error Handling | Structured JSON errors; bulk collection; actionable suggestions |
| 0009 | Hot Reload | File watcher → diff → hot-patch; stale resource semantics |
| 0010 | Testing | Four-layer pyramid: unit, replay regression, agent benchmarks, integration |
| 0011 | Expression Vocabulary | 7 operators: ref, eq, neq, lt, gt, add, sub |
| 0012 | System Registry | craft_system! macro; pure/impure distinction; schema auto-generation |
| 0013 | Input Model | InputBus → Input node; frame-latched; replay-compatible |
| 0014 | AI-Native Primitives | lint, dryRun, explain, diff; schema-exposed |
| 0015 | Performance Budgets | 8 committed targets: ≤8ms tick, ≤100ms hot reload, ≤5s replay |
| 0016 | Lua Scripting | Lua 5.5 via mlua; first-class scripting; GDScript parity |
| 0017 | Editor Architecture | egui + eframe embedded; file-based editing; PRD v2 deviation |
| 0018 | Editor Panels | Scene Tree, Inspector, Behavior Editor, Terminal Preview, File Browser |
| 0019 | Agent Copilot | Sidebar panel; context injection; diff review flow |
| 0020 | Lua Script Editor | LuaLS via LSP; engine type stubs; hot reload on save |
| 0021 | Editor UX | Visual language, shortcuts, drag-drop, 5 authoring workflows |

## Key Design Principles

1. **Agent and human share the same engine API** — the bridge (NAPI) and editor (direct call) both call the same `Engine` methods. No bifurcated API surface.

2. **Determinism is optional but testable** — JSON behaviors are always deterministic. Lua scripts are optionally lockable. Replay hash verification catches any divergence.

3. **Schema is the source of truth** — Rust types define the schema; schema generates TypeScript types. The agent's API view cannot drift.

4. **Minimal abstractions until proven** — 4-method Render trait, single Node struct, 9 action verbs. YAGNI applied at every layer. Extend when a second backend/user requires it.

5. **Godot is the reference, not the blueprint** — We copy what works (scene tree, signals, server pattern, editor docking) and replace what doesn't (deep OOP, ClassDB, Variant, custom build system, multi-platform windowing).
