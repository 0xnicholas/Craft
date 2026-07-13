# ADR 0007: Bridge Layer — Sync NAPI + JSON-RPC Adapter

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: Godot has no equivalent. GDScript runs inside the engine; the editor calls C++ directly. Craft needs a bridge because the agent runs in Node.js.

## Context

The agent (LLM) operates through a TypeScript SDK. The engine is a Rust library compiled as a native Node.js addon. The bridge must:
- Expose every `Engine` method to JavaScript/TypeScript
- Auto-generate TypeScript types from JSON Schema
- Run synchronously (no async reentrancy during tick)
- Serialize all data as JSON (schema-validated)
- Not introduce meaningful latency (no IPC, no network round-trips)

## Decision

**Sync NAPI bindings (via napi-rs) with a thin JSON-RPC dispatcher. All calls run in-process on the V8 main thread.**

```
Agent (TypeScript)
    │  import { Engine } from "craft-sdk";
    │  await engine.getSchema();
    ▼
SDK layer (TypeScript) — Promise.resolve wraps sync calls
    │
    ▼
NAPI bindings (Rust #[napi] fns) — sync, in-process, same thread
    │
    ▼
JSON-RPC dispatcher — deserialize JSON → call Engine method → serialize response
    │
    ▼
craft-kernel — Engine::start(), Engine::tick(), etc.
```

### Key Implementation

```rust
#[napi]
impl EngineHandle {
    #[napi(constructor)]
    pub fn new() -> Self;

    #[napi]
    pub fn start(&self, scene_path: String) -> napi::Result<String>;
    #[napi]
    pub fn tick(&self) -> napi::Result<String>;
    #[napi]
    pub fn get_schema(&self) -> napi::Result<String>;
    #[napi]
    pub fn get_component(&self, node_id: String, key: String) -> napi::Result<String>;
    #[napi]
    pub fn dry_run(&self, node_id: String, actions_json: String) -> napi::Result<String>;
    // ...
}
```

### TypeScript SDK

```typescript
export class Engine {
  private handle: EngineHandle;

  async start(scenePath: string): Promise<string> {
    return this.handle.start(scenePath);  // sync NAPI, Promise.resolve'd
  }
  async getSchema(): Promise<Schema> {
    return JSON.parse(this.handle.getSchema());
  }
  subscribe(signal: string, handler: (e: SignalEvent) => void): Unsubscribe;
  async dryRun(nodeId: NodeId, actions: Action[]): Promise<ComponentDiff>;
}
```

## Rationale

1. **Zero-latency engine calls**: Sync NAPI is a direct Rust function call — no IPC, no serialization overhead beyond JSON, no thread context switches. This preserves tick atomicity.

2. **No async reentrancy**: The engine is single-threaded (v1 invariant). Sync NAPI guarantees no concurrent engine calls. An agent cannot trigger `tick()` while `start()` is running.

3. **JSON as the wire format**: All Rust types implement `Serialize`/`Deserialize`. The JSON-RPC dispatcher is ~50 lines of code. No custom binary protocol needed.

4. **napi-rs is the standard**: Mature, well-maintained, handles NAPI versioning, cross-platform builds, and CI integration.

## Why Not JSON-RPC over IPC?

| Option | Latency | Complexity | When |
|--------|---------|------------|------|
| JSON-RPC over stdin/stdout | ~1-5ms/call | Child process, stdio protocol | v2 sidecar mode |
| JSON-RPC over WebSocket | ~0.5-2ms | Network stack, auth | Remote agent |
| **Sync NAPI (v1 choice)** | **~0μs (fn call)** | **Zero — direct Rust call** | Same-process agent |

v1 runs the agent and engine in the same Node.js process. IPC is unnecessary overhead.

## Godot Mapping

| Godot | Craft Bridge |
|-------|-------------|
| GDScript VM (embedded, ~30 files) | Node.js (host process, no VM to embed) |
| C# via Mono embedding | TypeScript via napi-rs |
| `core/extension/` C ABI + compatibility hash | NAPI (platform-standard, maintained by napi-rs) |
| ClassDB → runtime method lookup | Schema → compile-time TS type generation |
| Editor calls C++ directly | Agent calls TS SDK → NAPI → Rust |
| `Variant` serialization (proprietary) | JSON (schema-validated, standard) |

## Appendix: AI-Native Primitives (formerly ADR 0014)

The bridge exposes four agent-native primitives that make the engine "meaningfully more AI-native" (PRD §8.4):

| Primitive | Signature | Purpose |
|-----------|-----------|---------|
| `lint(scene)` | SceneDef → LintReport | Static analysis: signal wiring, state reachability, unused components, undefined refs. 6 rules. |
| `dryRun(nodeId, actions)` | NodeId × Action[] → ComponentDiff | Hypothetical execution in sandbox. Signals discarded, impure systems rejected. No side effects. |
| `explain(nodeId)` | NodeId → NodeExplanation | Structured node description optimized for LLM context windows (summary + components + state + children). |
| `diff(tickA, tickB)` | u32 × u32 → SnapshotDiff | Compare two recorded ticks. Component changes, node creations/destructions, signal emissions. |

**`dryRun` guardrails**: Uses a cloned read-only tree + write-tracked buffer. Signals emitted during dry run are discarded. Impure `call_system` is rejected. State hash unchanged after dry run — guaranteeing no side effects.

**`lint` is static**: Operates on parsed `SceneDef` before loading into `SceneTree`. Pure function, no tick loop needed.

All four primitives are schema-exposed via `engine.getSchema().primitives`, so the agent's TS types auto-generate and cannot drift.
