# ADR 0015: Performance Budgets

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: PRD §10.0 — committed v1 performance targets

## Context

PRD §10.0 defines concrete performance targets. These are not aspirations — they drive data-structure and algorithm choices. A milestone is "done" only if it meets its perf criterion.

## Decision

**The following budgets are committed v1 targets. All data-structure and algorithm choices must be validated against them.**

### Budget Table

| Metric | Target | Data-Structure Impact |
|--------|--------|----------------------|
| Active nodes per scene | 1,000 | `SlotMap<NodeId, Node>` — O(1) by-ID lookup, O(n) walk |
| Signals fired per tick | 100 | `VecDeque<SignalEvent>` — O(1) push/pop, no reallocation |
| Actions executed per tick | 10,000 | `Vec<ActionCommand>` — amortized O(1) push, linear drain |
| Tick rate (default) | 60 Hz | ~16.67ms wall clock per tick |
| Tick wall-clock budget (tower_defense scene) | ≤ 8 ms | 50% headroom for agent overhead |
| Hot-reload latency (tower_defense scene) | ≤ 100 ms | Fast path: component-only diff; slow path: structural diff is still O(n) |
| Replay of 1,000 ticks | ≤ 5 s wall clock | 5ms/tick average. Replay must be faster than real-time (16.67ms/tick) |
| Cold start (binary to first tick) | ≤ 500 ms | Scene parse + validate + load. No GPU init. JSON-only. |
| `engine.getSchema()` latency | ≤ 50 ms | Cached after first call. Subsequent calls return `OnceLock` value. |
| Schema validation latency (1,000-node scene, full validation) | ≤ 200 ms | All validation errors collected before returning (ADR 0008). Budget applies to `engine.validateScene()` and implicit validation during `engine.start()`. Agent may call `lint()` + `dryRun()` in a tight loop; validation must not be the bottleneck. |
| `engine.lint()` latency (1,000-node scene) | ≤ 50 ms | Static analysis only — no tick execution. Must be fast enough for agent to call before every `dryRun`. |

### Design Implications

**1. `SlotMap` over `HashMap` for Node storage**:
- `SlotMap<NodeId, Node>` gives O(1) lookup, O(1) insertion, O(1) removal
- Generational indices prevent ABA on despawn + respawn
- Iteration order is insertion-order stable (not HashMap's random order) — essential for determinism
- Depth-first walk: follow `Node.children` Vec, not iterate the SlotMap

**2. `VecDeque` for signal queue**:
- Signals are FIFO (declaration order within tick, tick-sequential across ticks)
- `VecDeque` gives O(1) push_back + pop_front
- Pre-allocated to 256 capacity (signals/tick target is 100)

**3. Pre-sized `Vec` for command buffer**:
- 10,000 action commands/tick target
- Single `Vec::with_capacity(10000)` allocated once, cleared between ticks with `.clear()`
- No per-action allocation during tick
- Action evaluation is the hot path — avoid heap allocation at all costs

**4. `OnceLock` for schema cache**:
- Schema generation (walking all types, recursing through definitions) is expensive
- `OnceLock<serde_json::Value>` computes once, returns `&Value` thereafter
- Cold start includes schema generation → falls within the 500ms budget

**5. Hot-reload fast path**:
- Component-only diff (no structural changes): O(component_count) ≈ O(1,000) — well within 100ms
- Structural diff (node add/remove): O(node_count × component_count). For 1,000 nodes: <100ms
- No partial diff application. Diff is computed in one pass, applied in one pass.

**6. Replay speed**:
- Replay skips the render phase (no terminal output)
- Replay skips hot-reload polling
- 1,000 ticks × 5ms = 5s wall clock (vs 16.67s if running at 60Hz with rendering)
- Input replay feed is O(1) per tick (look up frame by tick index)

**7. Schema validation**:
- Bulk error collection (ADR 0008) means validation runs to completion even after finding errors — must complete within 200ms for 1,000 nodes
- Validation is CPU-bound (JSON parsing + jsonschema traversal). Use `jsonschema` crate with pre-compiled schema — compile once at engine init, reuse per-validation
- Lint is a subset of validation (no JSON parse, only AST traversal). 50ms budget is aggressive but achievable with cached lookups (pre-computed signal subscriber index, state machine reachability cache)

### Profiling and Enforcement

- `craft-kernel/benches/` contains micro-benchmarks using `criterion`
- CI fails on >10% regression on any benchmark
- Reference game integration test measures wall-clock time per 1,000 ticks
- Hot reload latency measured as wall-clock time from file write detection to engine resumption

### Rejected: Performance without budgets

Without explicit budgets, performance decays silently. Every PR that adds O(n²) where O(n) sufficed, or allocates in the hot path, accumulates into a slow engine. Budgets make performance a first-class acceptance criterion — same as correctness.

## Godot Mapping

Godot does not publish explicit performance budgets. Its targets are implicit (60fps on reference hardware, "runs well"). Craft's explicit budgets are a consequence of the AI-native thesis: the agent needs predictable performance to plan its authoring loop.
