# Craft

An AI-native game engine in Rust. Designed so that both AI agents and humans can author games as first-class operations — not as a wrapper around a human-oriented engine.

Godot's architecture (scene tree, signals, resources, server abstraction) serves as the design reference, reimagined through Rust idioms and the AI-native constraint.

## What Makes It Different

| Property | Why |
|----------|-----|
| **Game logic = structured data** | Scenes, behaviors, and resources are JSON. Schema-validated. Agents write data reliably. |
| **Lua 5.5 as first-class scripting** | Human authors get GDScript-parity expressiveness. Agents get the JSON behavior path. |
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
   (Lua 5.5)   (recording)   (ANSI render)
                    │
                    ▼
              craft-schema
         (JSON Schema from Rust types)
```

## Project Structure

```
docs/
├── ARCHITECTURE.md              # Overarching system design
├── superpowers/specs/
│   └── 2026-07-09-craft-prd.md  # Product requirements
└── adr/
    ├── 0001-0015.md             # Engine core architecture (v1)
    ├── 0016.md                  # Lua scripting
    └── 0017-0021.md             # Editor architecture + UX (v2)

crates/                          # (planned, not yet scaffolded)
├── craft-kernel/                # Engine core
├── craft-lua/                   # Lua 5.5 runtime
├── craft-schema/                # Schema generation
├── craft-replay/                # Recording + replay
├── craft-bridge/                # NAPI bindings
├── craft-terminal/              # ANSI renderer
└── craft-editor/                # egui desktop editor (v2)

games/
└── tower_defense/               # Reference game (v1)
```

## Compared to Godot

| Godot (~800K lines C++) | Craft (~50-80K lines Rust) |
|--------------------------|---------------------------|
| Deep OOP class hierarchy (1,811 GDCLASS registrations) | Single `Node` struct; types differ by component keys |
| ClassDB + Variant + MethodBind runtime reflection | Compile-time JSON Schema generation |
| GDScript (custom language, 30 files) | Lua 5.5 via mlua (standard, 30yr ecosystem) |
| RenderingServer (100+ methods, Vulkan/Metal/D3D12) | Render trait (4 methods, ANSI terminal) |
| Editor (333K lines, custom GUI toolkit) | egui desktop app (v2, ~15K lines) |
| Multi-platform windowing (93K lines) | Node.js process (NAPI host) |
| 69 third-party C/C++ libraries | ~6 Rust crate dependencies |
| No built-in replay | Deterministic recording + hash-verified replay |
| No AI agent interface | sync NAPI bridge + TypeScript SDK + schema introspection |

## Documentation

| Document | Purpose |
|----------|---------|
| `docs/ARCHITECTURE.md` | System overview, crate DAG, key patterns, Godot comparison |
| `docs/superpowers/specs/...-prd.md` | Product requirements, v1 goals, success criteria |
| `docs/adr/0001-0015.md` | Engine core: node model, behavior runtime, schema, replay, bridge, testing |
| `docs/adr/0016.md` | Lua 5.5 scripting: two-tier behavior model, mlua bindings, GDScript parity |
| `docs/adr/0017-0021.md` | Editor: egui architecture, panels, Copilot, Lua editor, UX specification |

## Status

**Pre-implementation — architecture design phase.**

- [x] Product requirements (PRD)
- [x] Architecture decision records (21 ADRs)
- [x] Overarching system design (ARCHITECTURE.md)
- [ ] Engine core implementation (v1)
- [ ] Reference game: tower defense
- [ ] Lua scripting
- [ ] Editor (v2)

## License

MIT
