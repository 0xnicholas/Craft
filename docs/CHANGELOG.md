# Changelog

Per-release notes for Craft. Versions follow the spirit of [Semantic Versioning](https://semver.org/) — major bumps mark architectural shifts (v0 → v1 = engine core shipped; v1 → v2 = editor shipped; etc.), minor bumps mark new milestones.

## [v0.2.0] — 2026-07-19

### GPU Rendering (v3.0-GPU)

- **craft-gpu** crate: wgpu + winit 2D sprite renderer with instanced batching, texture caching, and camera transforms.
- New `ComponentValue` variants: `Vec3`, `Rect`.
- Universal component allowlist: `velocity`, `hitbox`, `hitbox_radius` added alongside `position`, `sprite`, `modulate`, etc.
- `SpriteDrawCommand` extraction from `ComponentView`.
- `Camera2D` node type with position, zoom, follow.
- Editor F5 launches GPU game window as subprocess (macOS-safe).

### Editor (v2, E1–E4)

- **E1**: 7-panel egui editor shell: Scene Tree, Inspector, File Browser, Terminal Preview, Behavior Editor, Lua Editor, Agent Copilot. Dock layout persistence. Project manifest parsing. File watcher with hot reload.
- **E2**: Behavior JSON editor with inline validation. Lua editor with LuaLS LSP support, diagnostics, auto-complete. Engine type stubs generation.
- **E3**: Agent Copilot panel. LLM chat interface. Context injection (scene, selection, diff). Diff preview + accept/reject flow.
- **E4**: Undo/redo (command pattern). Keyboard shortcuts (Ctrl+1-5, Ctrl+Shift+A, Ctrl+Z). Context menus (scene tree + file browser). Drag-drop (reparent, Lua attach, file→editor). Dark theme with node type emoji icons.

### Physics (v3.1)

- **craft-physics** crate: `craft_system!(PhysicsSystem, PreTick)`. Velocity integration + AABB/circle collision detection. Collisions emit targeted `collide` signal `{a, b}`.
- `SystemContext` extended with `scene` and `pending_signals` access.
- Signal bus API: `emit_by_name`, `pending_signal_names`, `deliver_and_collect`.
- Signal bridge: bus→JSON handler dispatch. Targeted dispatch: signal payload `{a, b}` filters handlers to only involved nodes.

### Particles (v3.1)

- **craft-particles** crate: `craft_system!(ParticleSystem, PostTick)`. `ParticleEmitter` nodes spawn `Particle` children with radial velocity, lifetime fade, and auto-cleanup.

### Audio (v3.2)

- **craft-audio** crate: `craft_system!(AudioSystem, PostTick)`. Signal-triggered WAV/OGG playback via rodio. Auto-discovers `assets/<signal>.wav` on each signal.

### Games

- **Tower defense** upgraded with physics: enemies, projectiles, hitboxes, collide handlers. Enemies spawn from right, move left toward towers.
- **Dungeon crawler** (new): turn-based roguelike. Player auto-moves, Lua enemy AI chases, physics collision combat, health potions, exit stairs. 6 integration tests.

### Agent Benchmarks

- 4/4 benchmark tasks passing (stub backend). LLM prompt fix for `[]` return when scene behaviors suffice.

### Documentation

- `AGENT_GUIDE.md`: how LLMs use Craft SDK (nodes, behaviors, actions, expressions, signals, Lua, physics, particles, audio).
- `SCHEMA.md`: JSON Schema reference for scenes, nodes, components, behaviors, actions, signals, systems.

### Quality

- 15 crates, all tests passing, clippy clean.

## [v0.1.0] — 2026-07-16

First stable release of the Craft engine. Implements all PRD §12 milestones (M1–M10) and the v1.5 Lua scripting sequence (L1–L3).

### Highlights

- **9 crates**: `craft-kernel`, `craft-macros`, `craft-schema`, `craft-replay`, `craft-bridge`, `craft-terminal`, `craft-lua`, `craft-eval`, plus the reference `tower_defense` game.
- **346 tests** across 14 integration test files (4-layer pyramid per ADR 0010).
- Single-threaded engine (ADR 0015) with an 8 ms tick budget, verified at 1000 ticks for the reference game.
- Deterministic replay (ADR 0006) with per-tick state hash verification.
- Structured JSON errors throughout (ADR 0008), with `file` / `json_path` / `expected_type` / `actual_value` / `suggestion` fields.
- Hot reload as the default authoring loop (ADR 0009) — file change → diff → apply in <100 ms.
- Property-bag Node component model with no class inheritance (ADR 0002) and 9 closed-set action verbs (ADR 0003).
- Compile-time JSON Schema generation from Rust types (ADR 0005).
- Lua 5.4 scripting (`craft-lua`):
    - **L1**: Node userdata with `__index` / `__newindex` field syntax; `engine.*` global API (`emit`, `spawn`, `call_system`, `rng`); sandbox enforced (no `io`/`os`/`debug`/`package.loadlib`; no `math.random`).
    - **L2**: Lua class lifecycle (`on_tick`, `on_signal`, `on_spawn`) integrated into the engine tick loop via the new `EngineHook` extension point (`craft-kernel/src/hook.rs`).
    - **L3**: three-switch determinism (`rng`, `float`, `order`) actually enforced at the `NodeRef` and global `pairs()` boundaries; workspace module loader with TOML `luarocks.lock` validation; `craft-replay` `Recorder::start_validated` for lockfile + RNG-locked recording.

### Coverage gate

`scripts/coverage.sh` measures craft-kernel production line coverage (currently **71.90%**), filtering out `#[cfg(test)]` blocks so adding tests does not lower the gate. Default threshold: 65%. Bump when adding coverage.

### Known gap

Reproducible 3/4 benchmark runs (ROADMAP v1 exit criterion) requires an LLM API key to exercise the `craft-eval` agent harness against the four benchmark tasks in `benchmarks/`. Tracked but deferred from this cut.

### Notes

- The original ADR 0016 specified Lua 5.5; we shipped on **Lua 5.4** (mlua 0.12 with feature `lua54`) because Lua 5.5 was not yet packaged by Homebrew as of the cut date. ADR updated with rationale.
- `craft-eval` is implemented but its agent-loop is tested by direct harness invocation only, not by running actual LLM agents.

[Compare with prior commits](https://github.com/0xnicholas/Craft/compare/61f0f0a...v0.1.0)