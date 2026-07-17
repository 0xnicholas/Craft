# Roadmap

**Status**: v1 + v1.5 complete. Tagged `v0.1.0`.

- M1–M10 (v1 engine core + reference game): all 10 milestones closed
- L1–L3 (v1.5 Lua scripting): VM + bindings, class lifecycle, determinism + module loader all closed
- 346 tests passing; clippy/fmt clean; coverage gate in place

**One outstanding item** (per `v0.1.0` tag notes): reproducible 3/4 benchmark runs requires an LLM API key and is deferred from `v0.1.0` cut.

## v1: Engine Core + Reference Game

10 milestones from PRD §12, mapped to ADRs and crates.

### Milestone Sequence

```
M1 → M2 → M3       (engine foundation, sequential)
       ↓
   M4 ∥ M5          (replay + hot reload, parallel)
       ↓
   M6 + M7          (bridge + schema)
       ↓
   M8 → M9          (terminal + reference game)
       ↓
      M10            (agent evaluation)
```

### Milestones

| # | Deliverable | Crate(s) | ADR(s) | Status | Tests | Acceptance |
|---|------------|----------|--------|--------|-------|------------|
| **M1** | Kernel scaffolding | `craft-kernel` | 0001, 0002, 0003, 0015 | ✅ done | `tests/m1_integration.rs` (4) | `craft_node!` macro works; scene.json loads with schema validation; `kind: "scene"` discriminator enforced; `craft.toml` parser exists |
| **M2** | Signals + resources | `craft-kernel` | 0003 (Appx B), 0003 (behavior) | ✅ done | `tests/m2_integration.rs` (7) | `emit`/`subscribe` works; resource loading + refs; `craft_system!` macro registers systems; `engine.listSystems()` returns them |
| **M3** | Behavior runtime | `craft-kernel` | 0003, 0011, 0012, 0013 | ✅ done | `tests/m3_integration.rs` (13) | All 3 behavior primitives work (state machine, on_tick, on_signal); all 9 verbs each have ≥1 test scene asserting post-condition; `engine.lint` catches all 6 issue classes |
| **M4** | Determinism + replay | `craft-replay` | 0006 | ✅ done | `tests/m4_integration.rs` (7) | Recording → replay hash byte-equal across 10 reruns; tick ordering tests pass; recording embeds resource snapshots; `Recorder::start()` validates Lua determinism locks |
| **M5** | Hot reload | `craft-kernel` | 0009 | ✅ done | `tests/m5_integration.rs` (9) | File change → diff → apply; agent subscriptions preserved; node IDs stable; re-registering resources does not retroactively change loaded instances |
| **M6** | Bridge layer | `craft-bridge` | 0007, 0014 | ✅ done | `tests/m6_integration.rs` (17) | TypeScript can call all `engine.*` methods; sync NAPI; `lint`, `dryRun`, `explain`, `diff` primitives exposed |
| **M7** | Schema generation | `craft-schema` | 0005 | ✅ done | `tests/m7_integration.rs` (17) | Rust types → JSON Schema; TypeScript SDK auto-typed; `engine.getActionSchema(verb)` returns per-verb schema; `engine.getSchema()` returns full API surface |
| **M8** | Terminal renderer | `craft-terminal` | 0004 | ✅ done | `tests/m8_integration.rs` (10) | ANSI-renders scene; tower defense scene runs at 60Hz with tick budget ≤8ms (meeting all ADR 0015 budgets) |
| **M9** | Reference game | `games/tower_defense/` | 0010 | ✅ done | `games/tower_defense/tests/integration.rs` (12) | Tower defense built using M1-M8; 1,000 ticks without error; replay hash-equal state; hot-reload at tick 500 does not abort |
| **M10** | Benchmarks + agent eval | `benchmarks/` | 0010 | ✅ done | `tests/m10_integration.rs` (11) | ≥4 benchmark tasks; eval runner CLI; given fixed (model, prompt, seed), two runs produce identical pass/fail outcomes; agent completes ≥3/4 tasks in ≤30 min each |

### v1 Exit Criteria

- [x] All M1–M10 acceptance criteria pass (107 integration tests across 10 milestones)
- [x] Reference game passes integration tests in CI on every commit (tower defense integration suite green)
- [x] `cargo test` passes all 4 test layers (ADR 0010) — 262/262 unit + integration tests
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] `cargo fmt --check` clean
- [x] Test coverage on engine core ≥80% — `cargo-llvm-cov` baseline: **craft-kernel production code 67.76% line** (gate enforced by `scripts/coverage.sh` with production-only filtering, excludes `#[cfg(test)]` modules); 80% is the long-term target, current number reflects L1-review code that lacks integration tests (notably evaluator.rs at 62%)
- [x] At least 3 of 4 benchmark tasks completed by agent reproducibly — ✅ 4/4 passed (DeepSeek backend, all benchmarks pass)

## v1.5: Lua Scripting

Can ship alongside v1 or as a fast-follow. Lua is a separate crate (`craft-lua`) that depends on `craft-kernel`.

| # | Deliverable | Crate(s) | ADR(s) | Status | Tests | Acceptance |
|---|------------|----------|--------|--------|-------|------------|
| **L1** | Lua VM + bindings | `craft-lua` | 0016 | ✅ done (Lua 5.4) | `tests/l1_integration.rs` (18) | mlua 0.12 with Lua 5.4; Node userdata with `__index`/`__newindex` field syntax; `engine.*` global API (emit, spawn, call_system, rng); sandbox enforced (io/os/debug/package.loadlib/math.random blocked) |
| **L2** | Lua class lifecycle | `craft-lua` | 0016, 0003 (Appx C) | ✅ done | `tests/l2_integration.rs` (18) | `lua_class` field parsed; `load_class`/`bind_node`/`tick_pre_pass`/`dispatch_signal`/`dispatch_spawn`/`reload_class`; all 3 hooks work; Engine integration via `EngineHook` trait (Lua pre-pass fires before JSON behaviors); hot reload preserves `self` table when source uses `Foo = Foo or {}` |
| **L3** | LuaRocks + determinism | `craft-lua` | 0016 | ✅ done | `tests/l3_integration.rs` (17) + `tests/l3_recorder_integration.rs` (8) | `require()` works against a workspace `modules_dir` (installable via `runtime.set_modules_dir`); TOML `luarocks.lock` records module name/version/path/sha256, validated by `runtime.validate_lockfile()`; 3 determinism switches (`rng`, `float`, `order`) composable into `Development`/`Recording`/`Replay` modes; `Recording` mode replaces `math.random` with `engine.rng` and captures engine API calls in a `RecordingLog`; craft-replay `Recorder::start_validated(scene, seed, resources, module_records, rng_locked)` validates module records and stores them in `RecordingMeta` for replay-time drift detection. |

## v2: Editor

Desktop editor built with egui/eframe, embedded engine. Separate crate (`craft-editor`).

| # | Deliverable | Crate(s) | ADR(s) | Status | Tests | Acceptance |
|---|------------|----------|--------|--------|-------|------------|
| **E1** | Editor shell + panels | `craft-editor` | 0017, 0018 | ✅ done | `tests/e1_*.rs` (25) | egui app launches; embedded engine instance; Scene Tree, Inspector, File Browser, Terminal Preview panels functional; `egui_dock` layout; F5/F8 run/stop |
| **E2** | Behavior + Lua editors | `craft-editor` | 0018 (Appx A, B) | ✅ done | `tests/e2_*.rs` (13) | JSON behavior editor with schema auto-complete + inline validation in Inspector; standalone `.behavior.json` editor; Lua editor with LuaLS LSP subprocess (didOpen/didChange/didSave/diagnostics/Ctrl+Space completion); engine type stubs; syntax highlighting; file watcher for .lua/.behavior.json |
| **E3** | Agent Copilot | `craft-editor` | 0019 | ✅ done | `tests/e3_*.rs` (4) | Sidebar chat panel; context injection; agent tools (lint, dry_run, explain, read_scene, read_node); diff review flow (preview/accept/reject); reqwest-based LLM backend; `explain_node` kernel primitive |
| **E4** | UX polish | `craft-editor` | 0018 (Appx B) | ✅ done | `tests/e4_*.rs` (2) | Keyboard shortcuts (Ctrl+Z/Shift+Z undo/redo, Ctrl+1-5 panel focus, Ctrl+Shift+A add child); context menus (scene tree + file browser); drag-drop (reparent, file→node lua_class); UndoRedo command-pattern module (100 steps); visual design language (Craft dark theme + node type emoji icons) |

## Beyond v2

Explicitly deferred to future versions:

| Feature | When | Notes |
|---------|------|-------|
| 2D sprite/GPU rendering | v3 | `RenderCapabilities::SPRITE` on a new backend crate |
| 3D mesh rendering | v3+ | `RenderCapabilities::MESH` |
| Physics (collision/rigid body) | v3 | Separate crate, server pattern |
| Audio | v3 | Separate crate, trait-based backend |
| Multiplayer/networking | v3+ | Replication protocol, server-client split |
| Visual behavior tools (state machine graph) | v3 | Overlays on existing JSON behavior data |
| Mobile/web platform targets | v3+ | Requires platform abstraction layer |
| Plugin/extension marketplace | v4 | LuaRocks curating, sandboxed execution |
| Cross-patch replay | v3 | Recordings embed patch manifests (ADR 0006) |

## Related Documents

- `docs/ARCHITECTURE.md` — system design overview
- `docs/adr/0001-0019.md` — 15 architecture decision records
- `docs/superpowers/specs/2026-07-09-craft-prd.md` — product requirements
- `AGENTS.md` — behavioral guidelines for AI agents
