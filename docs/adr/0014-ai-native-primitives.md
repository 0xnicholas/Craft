# ADR 0014: AI-Native Primitives

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: PRD §8.4 — `lint`, `dryRun`, `explain`, `diff`

## Context

PRD §8.4 defines four methods that make the engine "meaningfully more agent-native." These are not convenience helpers — they are first-class architectural features with implications for the behavior runtime, schema generation, and the bridge layer. Each requires specific engine-side support beyond simple method dispatch.

## Decision

**Four AI-native primitives implemented as first-class `Engine` methods, each with dedicated schema exposure and architectural guardrails.**

### 1. `engine.lint(scene)` — Static Scene Analysis

Runs before load. Detects issues that schema validation misses (schema checks structure; lint checks semantics).

```rust
// craft-kernel/src/lint/mod.rs

pub struct LintReport {
    pub warnings: Vec<LintWarning>,
    pub stats: LintStats,
}

pub struct LintWarning {
    pub json_path: String,
    pub rule: String,
    pub message: String,
    pub severity: LintSeverity,        // "warning" by default
}

pub enum LintSeverity { Warning, Error }

pub struct LintStats {
    pub node_count: u32,
    pub signal_count: u32,
    pub behavior_count: u32,
    pub component_count: u32,
}
```

**Lint rules (v1, 6 rules per PRD §8.4)**:

| Rule | Check |
|------|-------|
| `signal-no-subscribers` | Any `emit` action references a signal with no subscribers |
| `unreachable-state` | State machine has states not reachable from `initial` |
| `unused-component` | A node declares a component never read by any behavior |
| `undefined-node-ref` | An action `target` references a node ID not in the scene |
| `missing-system` | `call_system` references a system name not in registry |
| `broken-resource-path` | A `res://` path in a `ResourceRef` doesn't resolve |

Lint is cheap (static analysis, no tick execution) and high agent value (catches the most common agent errors before runtime).

### 2. `engine.dryRun(nodeId, actions)` — Hypothetical Execution

Execute a proposed action list against the current state and return the resulting component diffs, **without mutating** the scene tree.

```rust
impl Engine {
    pub fn dry_run(&self, node_id: NodeId, actions: &[Action]) -> ComponentDiff {
        // 1. Clone the command buffer
        // 2. Evaluate actions in a sandbox (read-only tree access)
        // 3. Compute what WOULD change
        // 4. Discard the sandbox — no mutations to the real tree
        // 5. Return the diff
    }
}

pub struct ComponentDiff {
    pub node_id: NodeId,
    pub changes: HashMap<String, ComponentChange>,
}

pub struct ComponentChange {
    pub before: ComponentValue,
    pub after: ComponentValue,
}
```

**Guardrails**:

| Rule | Behavior |
|------|----------|
| Signals emitted during dry run | Discarded — not queued, not delivered |
| `call_system` with impure system | Rejected with `dryRun:system_must_be_pure` error |
| `spawn` / `destroy` | Tracked in diff but not executed |
| State hash after dry run | Unchanged — guarantees no side effects |

### 3. `engine.explain(nodeId)` — Structured Node Description

Return a `NodeExplanation` — a structured object designed for LLM context windows.

```rust
pub struct NodeExplanation {
    /// One-line summary, e.g., "Player at (10, 10), moving right, score 50, state: playing, 3 children"
    pub summary: String,

    /// All components with current values and types
    pub components: HashMap<String, ComponentExplained>,

    /// Current state machine state, if any
    pub active_state: Option<String>,

    /// Signal names this node subscribes to
    pub signal_subscriptions: Vec<String>,

    /// Child node IDs (for tree navigation)
    pub children: Vec<String>,
}

pub struct ComponentExplained {
    pub value: ComponentValue,
    pub kind: String,           // "regular" | "transient"
    pub default: ComponentValue,
    pub lifetime_remaining: Option<u32>,
}
```

The `summary` string is auto-generated from a template: `"{type_name} at {position}, {velocity}, {state}, {count} children"`. This format is optimized for LLM context windows — compact, informative, no extraneous tokens.

The schema is exposed via `engine.getNodeExplanationSchema()` so the agent's TS types stay in sync.

### 4. `engine.diff(tickA, tickB)` — State Difference

Return a `SnapshotDiff` between two ticks for agent debugging.

```rust
pub struct SnapshotDiff {
    pub tick_a: u32,
    pub tick_b: u32,
    pub component_changes: Vec<ComponentChangeEntry>,
    pub node_creations: Vec<String>,
    pub node_destructions: Vec<String>,
    pub signal_emissions: Vec<SignalRecord>,
}

pub struct ComponentChangeEntry {
    pub node_id: String,
    pub key: String,
    pub before: ComponentValue,
    pub after: ComponentValue,
}
```

The engine stores per-tick snapshots during recording. `diff()` computes the delta between two snapshots in the recording buffer. If either tick is not recorded, returns an error.

The schema is exposed via `engine.getSnapshotDiffSchema()`.

### Schema Exposure

All four primitives are exposed through `engine.getSchema()`:

```json
{
  "primitives": {
    "lint": { "input": "Scene", "output": "LintReport" },
    "dryRun": { "input": ["NodeId", "Action[]"], "output": "ComponentDiff" },
    "explain": { "input": "NodeId", "output": "NodeExplanation" },
    "diff": { "input": ["u32", "u32"], "output": "SnapshotDiff" }
  }
}
```

## Rationale

1. **`dryRun` needs sandboxed evaluation**: Cannot reuse the standard `evaluate → buffer → apply` pipeline because the apply phase must not reach the real tree. A cloned buffer with a `DryRunSceneTree` (read-only + write-tracked) provides this isolation.

2. **`lint` is static analysis, not runtime**: It operates on the parsed `SceneDef` before loading into `SceneTree`. This means it doesn't need a tick loop — it's a pure function `SceneDef → LintReport`.

3. **`explain` composes multiple queries into one call**: Without it, the agent would need `getNode(id)` + `getComponent(id, k1)` + `getComponent(id, k2)` + ... — N+1 round trips for each node. `explain` returns everything in one call.

4. **`diff` requires recording**: Per-tick snapshots are stored during recording (ADR 0006). `diff()` reads from this buffer, not from the live scene tree. This keeps `diff` fast (O(changed components)) and replay-independent.

## Godot Mapping

| Godot | Craft |
|-------|-------|
| No lint system | `engine.lint()` — 6 static analysis rules |
| No dry run (must actually run the scene) | `engine.dryRun()` — sandboxed evaluation, no side effects |
| `print_tree()` (string output) | `engine.explain()` — structured JSON, LLM-optimized format |
| No tick diff | `engine.diff()` — per-tick snapshot comparison |
| Manual debug inspection | Schema-exposed primitives — agent discovers them via `getSchema()` |
