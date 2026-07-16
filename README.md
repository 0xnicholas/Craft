# Craft

An AI-native game engine in Rust. Designed so that both AI agents and humans can author games as first-class operations — not as a wrapper around a human-oriented engine.

Godot's architecture (scene tree, signals, resources, server abstraction) serves as the design reference, reimagined through Rust idioms and the AI-native constraint.

## What Makes It Different

| Property | Why |
|----------|-----|
| **Game logic = structured data** | Scenes, behaviors, and resources are JSON. Schema-validated. Agents write data reliably. |
| **Lua 5.4 as first-class scripting** | Human authors get GDScript-parity expressiveness. Agents get the JSON behavior path. |
| **Closed-set action vocabulary** | 9 verbs. Every action is enumerable, schema-validated, and deterministic. |
| **Schema is the source of truth** | Rust types → JSON Schema → TypeScript types. The agent's API view cannot drift. |
| **Deterministic replay** | Same scene + same seed + same input = byte-identical state every tick. Testable in CI. |
| **Hot reload as default** | Edit scene.json or .lua → see effect in <100ms. No restart. |
| **Errors are structured data** | Every error has `file`, `json_path`, `expected_type`, `actual_value`, and `suggestion`. Agents can auto-correct. |
| **10x less code than Godot** | No deep OOP class hierarchy, no custom GUI toolkit, no multi-platform windowing, no Variant/ClassDB system. |

## Architecture

```
Agent (LLM)                       Human (v2)
TypeScript SDK + NAPI              egui desktop app + embedded engine
         │                              │
         ▼                              ▼
    craft-bridge                    craft-editor
         │                              │
         └──────────┬───────────────────┘
                    ▼
              craft-kernel
      scene · signal · behavior · resource · system · hot_reload · lint
                    │
         ┌──────────┼──────────┐
         ▼          ▼          ▼
   craft-lua   craft-replay  craft-terminal
   (Lua 5.4)   (recording)   (ANSI render)
                    │
                    ▼
              craft-schema
         (JSON Schema from Rust types)
```

## Project Structure

```
docs/
├── ARCHITECTURE.md              # Overarching system design
├── CHANGELOG.md                 # Per-release notes (v0.1.0+)
├── superpowers/specs/
│   └── 2026-07-09-craft-prd.md  # Product requirements
└── adr/
    ├── 0001-0010.md + 0015.md   # Engine core architecture (v1)
    ├── 0016.md                  # Lua scripting
    └── 0017-0019.md             # Editor architecture + panels + Copilot (v2)

crates/
├── craft-kernel/                # Engine core (scene, signal, behavior, ...)
├── craft-macros/                # #\[craft_node!\], #\[craft_system!\] proc macros
├── craft-schema/                # JSON Schema from Rust types
├── craft-replay/                # Recording + replay runner + drift checks
├── craft-bridge/                # NAPI bindings + JSON-RPC dispatcher
├── craft-terminal/              # ANSI renderer (NullRenderer for tests)
├── craft-lua/                   # Lua 5.4 via mlua (VM, bindings, hooks)
├── craft-eval/                  # Benchmark harness (LLM-driven agent tasks)
└── (craft-editor/ planned v2)

games/
└── tower_defense/               # Reference game (v1)
```

Compared to Godot
----------------

| Godot (~800K lines C++) | Craft (~50-80K lines Rust) |
|--------------------------|---------------------------|
| Deep OOP class hierarchy (1,811 GDCLASS registrations) | Single `Node` struct; types differ by component keys |
| ClassDB + Variant + MethodBind runtime reflection | Compile-time JSON Schema generation |
| GDScript (custom language, 30 files) | Lua 5.4 via mlua (standard, 30yr ecosystem) |
| RenderingServer (100+ methods, Vulkan/Metal/D3D12) | Render trait (4 methods, ANSI terminal) |
| Editor (333K lines, custom GUI toolkit) | egui desktop app (v2, ~15K lines) |
| Multi-platform windowing (93K lines) | Node.js process (NAPI host) |
| 69 third-party C/C++ libraries | ~10 Rust crate dependencies (mlua, schemars, serde, ...) |
| No built-in replay | Deterministic recording + hash-verified replay |
| No AI agent interface | sync NAPI bridge + TypeScript SDK + schema introspection |

Documentation
-------------

| Document | Purpose |
|----------|---------|
| `docs/ARCHITECTURE.md` | System overview, crate DAG, key patterns, Godot comparison |
| `docs/superpowers/specs/...-prd.md` | Product requirements, v1 goals, success criteria |
| `docs/CHANGELOG.md` | Per-release notes |
| `docs/adr/0001-0010.md + 0015.md` | Engine core: node model, behavior runtime, schema, replay, bridge, error handling, testing, performance |
| `docs/adr/0016.md` | Lua scripting: two-tier behavior model, mlua, sandbox, determinism |
| `docs/adr/0017-0019.md` | Editor: egui architecture, panels + UX, Copilot collaborative editing |

Status
------

**`v0.1.0` shipped — v1 engine + v1.5 Lua scripting complete.** See `docs/CHANGELOG.md`.

- [x] Product requirements (PRD)
- [x] Architecture decision records (16 ADRs)
- [x] Overarching system design (`docs/ARCHITECTURE.md`)
- [x] Engine core implementation: 10 milestones (M1–M10)
- [x] Reference game: tower defense (1000 ticks, replayable)
- [x] Lua scripting: L1 (VM + bindings), L2 (class lifecycle), L3 (determinism + modules)
- [x] Test infrastructure (346 tests; 4-layer pyramid per ADR 0010)
- [x] Coverage gate (`scripts/coverage.sh`)
- [ ] Editor (v2)
- [ ] LLM-driven agent benchmarks — reproducible 3/4 runs deferred (requires API key)

### Quick Start

Build and test:

```bash
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/coverage.sh
```

For Lua to compile, the system needs Lua 5.4:

```bash
brew install lua@5.4
export PKG_CONFIG_PATH="$(brew --prefix lua@5.4)/lib/pkgconfig"
cargo build -p craft-lua
```

License
-------

MIT
