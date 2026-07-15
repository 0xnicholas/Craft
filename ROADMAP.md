# Roadmap

**Status**: v1 engine implementation complete (`61f0f0a`). M1–M10 merged. 6 of 7 v1 exit criteria satisfied; one remains: reproducible 3/4 benchmark runs.

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
- [x] Test coverage on engine core ≥80% — `cargo-llvm-cov` baseline: **craft-kernel 80.29% line / 79.97% region**; workspace 82.19% / 82.48%; gate enforced by `scripts/coverage.sh`
- [ ] At least 3 of 4 benchmark tasks completed by agent reproducibly — `craft-eval` runner + 4 specs exist; LLM-driven runs not yet executed

## v1.5: Lua Scripting

Can ship alongside v1 or as a fast-follow. Lua is a separate crate (`craft-lua`) that depends on `craft-kernel`.

| # | Deliverable | Crate(s) | ADR(s) | Acceptance |
|---|------------|----------|--------|------------|
| **L1** | Lua VM + bindings | `craft-lua` | 0016 | mlua 0.12 with Lua 5.5; Node userdata with `__index`/`__newindex` field syntax; `engine.*` global API (emit, spawn, call_system, rng); sandbox enforced |
| **L2** | Lua class lifecycle | `craft-lua` | 0016, 0003 (Appx C) | `lua_class` field loads + instantiates Lua class; `on_tick`, `on_signal`, `on_spawn` hooks; Lua pre-pass integrated into tick loop; hot reload preserves `self` table for same-class edits |
| **L3** | LuaRocks + determinism | `craft-lua` | 0016 | `require()` works; `luarocks.lock` enforced for recording; 3-switch determinism lock (RNG/Float/Order); `Recorder::start()` validates locks |

## v2: Editor

Desktop editor built with egui/eframe, embedded engine. Separate crate (`craft-editor`).

| # | Deliverable | Crate(s) | ADR(s) | Acceptance |
|---|------------|----------|--------|------------|
| **E1** | Editor shell + panels | `craft-editor` | 0017, 0018 | egui app launches; embedded engine instance; Scene Tree, Inspector, File Browser, Terminal Preview panels functional; `egui_dock` layout; F5/F8 run/stop |
| **E2** | Behavior + Lua editors | `craft-editor` | 0018 (Appx A, B) | JSON editor with schema auto-complete + inline validation; Lua editor with LuaLS LSP subprocess + engine type stubs; syntax highlighting |
| **E3** | Agent Copilot | `craft-editor` | 0019 | Sidebar chat panel; context injection; diff review flow (preview/accept/modify/reject); agent tools map to engine API |
| **E4** | UX polish | `craft-editor` | 0018 (Appx B) | Keyboard shortcuts (Godot-aligned); drag-drop (reparent, file→node); context menus; undo/redo (100-level per-SceneDef); visual design language |

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
