# AGENTS.md

Behavioral guidelines and project context for AI agents working on Craft.

## Project Context

**Craft** is an AI-native game engine in Rust. It uses Godot's architecture (scene tree, signals, resources) as a design reference but reimagines every subsystem through Rust idioms.

- **Docs**: `docs/ARCHITECTURE.md` (system design), `docs/superpowers/specs/` (PRD), `docs/adr/` (15 architecture decision records)
- **Reference**: Godot 4.x source in `_references/godot-master/` (read-only, do not modify)
- **Godot root at:** `_references/godot-master/`

## Code Conventions

### Rust
- **Edition 2024**. Use workspace `Cargo.toml` at repo root. Crates under `crates/`.
- **No comments unless asked.** Code should be self-documenting through clear naming. Exceptions: `// SAFETY:` for unsafe blocks, doc comments on public API types and functions.
- **No unused code.** If your changes make a function/import/variable unused, remove it. Do not remove pre-existing dead code unless asked.
- **Match existing style.** Follow surrounding code patterns. Do not reformat adjacent code.
- **No `unwrap()` or `expect()` in production code.** Use `EngineResult<T>` with structured errors (ADR 0008). Every error must carry `file`, `json_path`, and `suggestion`.
- **`cargo clippy -- -D warnings` must pass.** No warnings allowed in CI.
- **`cargo fmt` must pass.**

### TypeScript (SDK)
- Types are **auto-generated** from `craft-schema`. Never hand-write types that duplicate schema definitions.
- SDK wraps sync NAPI calls in `Promise.resolve()`. No real async — engine is single-threaded (ADR 0007).

### Lua (game scripts)
- Lua 5.5. Scripts live in `games/<name>/scripts/`.
- Engine API uses field syntax (`node.position`, not `node:get("position")`). See ADR 0016.
- **No `math.random()`** — use `engine.rng()` for deterministic behavior.
- **No `io.*`, `os.*`** — Lua VM is sandboxed.

## Architecture Constraints (Non-Negotiable)

These reflect decisions in `docs/adr/`. Do not reverse them without writing a new ADR.

| Constraint | ADR |
|------------|-----|
| No class inheritance — single `Node` struct with property-bag components | 0002 |
| Two-tier behavior: Lua (direct mutation) + JSON (command buffer → apply) | 0003, 0016 |
| 9 closed-set action verbs — agents cannot invent new verbs | 0003 |
| Signals delivered next-tick, not same-tick | 0003 |
| Render trait is 4 methods — no GPU abstraction in v1 | 0004 |
| Schema is compile-time JSON Schema generation, not runtime reflection | 0005 |
| Replay is deterministic — per-tick state hash must match | 0006 |
| Bridge is sync NAPI — no IPC, no WebSocket in v1 (transport trait exists for v2) | 0007 |
| Errors are structured JSON — never panic, never print to stderr | 0008 |
| Hot reload is the default authoring loop — not an afterthought | 0009 |
| Engine is single-threaded for v1 — no async, no concurrency | 0015 |
| Tick budget ≤8ms for tower_defense scene | 0015 |
| Editor is v2 — do not implement editor features in v1 engine crates | 0017-0019 |

## File Layout Rules

- `crates/craft-kernel/src/` — engine core modules (`scene/`, `signal/`, `behavior/`, `resource/`, `system/`, `hot_reload/`, `lint/`)
- `crates/craft-lua/src/` — Lua VM + engine bindings. Depends on kernel. Kernel does NOT depend on lua.
- `crates/craft-schema/src/` — schemars + Craft attribute macros. No kernel dependency.
- `crates/craft-replay/src/` — recording codec + replay runner. Depends on kernel.
- `crates/craft-bridge/src/` — NAPI bindings + JSON-RPC dispatcher. Depends on kernel + schema.
- `crates/craft-terminal/src/` — ANSI renderer. Implements `Render` trait from kernel.
- `crates/craft-editor/src/` — egui desktop app. Depends on kernel + lua + schema. v2 only.
- `games/` — reference games. JSON scenes, Lua scripts, behavior files.
- `_references/` — Godot source (do not modify).

## Testing

Four-layer test pyramid (ADR 0010):

1. **Unit tests** — `#[cfg(test)] mod tests` in every .rs file. `cargo test`
2. **Replay regression** — record + replay + hash compare. Fields in `crates/craft-replay/tests/`
3. **Agent benchmarks** — 4 LLM-driven tasks. `benchmarks/` directory. Reproducible pass/fail.
4. **Reference game integration** — tower defense at 1000+ ticks. `games/tower_defense/tests/`

Use `NullRenderer` (no-op `Render` impl) for headless tests.

## Before Claiming "Done"

- `cargo test` passes all four layers
- `cargo clippy -- -D warnings` clean
- `cargo fmt --check` clean
- Replay regression: hash matches at every tick
- No new `unwrap()` or `expect()` in production code
- No commented-out code (delete it)
- No unrelated changes in diff
