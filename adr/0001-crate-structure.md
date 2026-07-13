# ADR 0001: Crate Structure & Workspace Layout

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: Godot directory structure (`_references/godot-master/`)

## Context

Craft is an AI-native game engine implemented in Rust, using Godot's architecture as a design reference. The engine is split across multiple concerns: a kernel, recording/replay, schema generation, an agent bridge, and a terminal renderer. We need to decide crate boundaries and the dependency direction.

Godot's equivalent is a monolithic C++ tree (`core/`, `scene/`, `servers/`, `editor/`, `modules/`, `platform/`, `drivers/`) compiled by SCons with per-directory `SCsub` files.

## Decision

**5 Cargo workspace crates** with internal `pub mod` boundaries within `craft-kernel`:

```
craft/
├── Cargo.toml              # workspace root
├── crates/
│   ├── craft-kernel/       # engine core: scene, signal, behavior, resource, system, hot_reload, lint
│   │   └── src/
│   │       ├── scene/      # Node, SceneGraph, Component storage
│   │       ├── signal/     # Signal bus, subscriptions
│   │       ├── behavior/   # Command buffer, scheduler, action interpreter
│   │       ├── resource/   # Resource registry, URI resolution
│   │       ├── system/     # craft_system! macro, system registry
│   │       ├── hot_reload/ # File watcher, diff, hot-patch
│   │       └── lint/       # Static scene analysis
│   ├── craft-schema/       # schemars-based JSON Schema generation + Craft extension attributes
│   ├── craft-replay/       # recording codec, seed management, replay runner
│   ├── craft-bridge/       # NAPI bindings + JSON-RPC adapter
│   └── craft-terminal/     # ANSI terminal renderer (implements Render trait)
├── sdk/                    # TypeScript SDK (npm package)
└── games/
    └── tower_defense/      # reference game
```

**Dependency DAG**:
```
craft-terminal ──→ craft-kernel  (Render trait defined here)
craft-replay   ──→ craft-kernel  (reads scene, feeds ticks)
craft-bridge   ──→ craft-kernel + craft-schema  (wraps kernel, exports schemas)
craft-schema   ──→ (no kernel dep; only schemars + syn/quote for proc macros)
```

## Rationale

1. **Separate crates for real boundaries**: `craft-replay` has its own storage/serialization concerns. `craft-bridge` has NAPI/FFI linking concerns. `craft-terminal` is a swappable render backend. `craft-schema` is code-generation tooling with no runtime dependency on the kernel.

2. **Kernel stays as one crate for now**: The internal modules (`scene`, `signal`, `behavior`, `resource`, `system`, `hot_reload`, `lint`) are highly interdependent. Extracting them into sub-crates before the interfaces stabilize is premature abstraction. Rust's `pub mod` system provides sufficient isolation.

3. **Trait definitions live in `craft-kernel`**: The `Render` trait and `Replayable` trait are defined in the kernel crate so backends and tools depend on the kernel, not vice versa.

## Godot Mapping

| Godot Directory | Craft Equivalent | Notes |
|-----------------|-----------------|-------|
| `core/` | No standalone crate | Math types inline in kernel; no Variant/Object/ClassDB system |
| `scene/` | `craft-kernel::scene/` | Unified property-bag Node model (no deep inheritance) |
| `servers/rendering/` | `Render` trait in `craft-kernel` | 4 methods vs Godot's 100+ method RenderingServer |
| `drivers/vulkan/` etc. | `craft-terminal/` | Only v1 render backend |
| `modules/gdscript/` | `craft-kernel::behavior/` + `::system/` | JSON interpreter + Rust system registry, no scripting VM |
| `editor/` (33万行) | None | PRD explicitly excludes GUI editor |
| `platform/` (9万行) | None | v1 runs in Node.js process; platform handled by NAPI host |
| `core/extension/` | `craft-kernel::system/` `craft_system!` macro | Rust-internal extension, not cross-language C ABI |
| SCsub build system | Standard `Cargo.toml` workspace | No custom build system needed |
| `thirdparty/` (69 libs) | ~5 cargo dependencies | schemars, serde, napi-rs, notify, rand |

## Scale Comparison

- Godot (excluding thirdparty): ~800K lines C++
- Craft v1 (projected): ~50-80K lines Rust
- Godot's largest eliminated subsystems: editor (333K), Variant/ClassDB/reflection system (~170K core), platform abstraction (93K), physics servers (~30K), audio server (~14K)
