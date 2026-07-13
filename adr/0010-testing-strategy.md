# ADR 0010: Testing Strategy — Four-Layer Test Pyramid

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: Godot uses doctest with test files in `tests/`. Craft extends this with replay regression and agent benchmarks.

## Context

PRD requirements:
- Test coverage on engine core ≥80%
- AI agent completes ≥3/4 benchmark tasks reproducibly
- Replay regression: hash-identical state across runs
- All `cargo test` green

Testing a game engine presents unique challenges: determinism must be verified, the behavior runtime must be tested in isolation, and the engine's fitness for AI agents must be measured quantitatively.

## Decision

**Four-layer test pyramid:**

```
┌─────────────────────────────────────────┐
│ Layer 4: Reference game integration      │  ~10 tests
│ (full tower defense at 1000+ ticks)      │
├─────────────────────────────────────────┤
│ Layer 3: Agent benchmark harness         │  4 benchmark tasks
│ (LLM-driven task completion)             │
├─────────────────────────────────────────┤
│ Layer 2: Replay regression tests         │  ~20 tests
│ (record + replay + hash compare)         │
├─────────────────────────────────────────┤
│ Layer 1: Unit tests                      │  ~200+ tests
│ (Rust #[test] per module)                │
└─────────────────────────────────────────┘
```

### Layer 1: Unit Tests

Standard `#[cfg(test)] mod tests` in every Rust module. Covers:
- Scene graph operations (spawn, despawn, depth-first walk order)
- Component read/write with type validation
- Signal subscription, emission, and next-tick delivery
- Behavior evaluation: each action verb in isolation
- State machine transitions with various conditions
- Transient component lifecycle (counter decrement, reset, .done signal)
- Hot reload diff computation
- Resource registry operations
- RNG determinism (same seed → same sequence)

```rust
#[test]
fn spawn_despawn_preserves_deterministic_order() {
    let mut tree = SceneTree::new();
    let a = tree.spawn(node_def("A", None)).unwrap();
    let b = tree.spawn(node_def("B", Some(a))).unwrap();
    let c = tree.spawn(node_def("C", None)).unwrap();
    let order: Vec<_> = tree.walk_depth_first().collect();
    assert_eq!(order, vec![a, b, c]);
}
```

### Layer 2: Replay Regression Tests

End-to-end determinism verification. Covers:
- Same scene + same seed + same input → byte-identical state at every tick
- Hot reload at tick N doesn't affect ticks 0..N-1
- Resource snapshot independence (replay uses recorded version, not current)
- Cross-platform determinism (f64 only, no f32, ordered collections)

```rust
#[test]
fn tower_defense_1000_ticks_replay_hash_match() {
    let recording = record_run(scene, 42, 1000);
    let replay = replay_run(&recording);
    for (tick, (expected, actual)) in replay.hashes.iter().enumerate() {
        assert_eq!(expected, actual, "hash mismatch at tick {}", tick);
    }
}
```

### Layer 3: Agent Benchmark Harness

Measures the engine's fitness for AI agents. Each benchmark is:
- A natural language task description
- An initial scene (JSON)
- A goal check function (Rust predicate)
- Max agent actions and time limit

```rust
pub struct AgentBenchmark {
    pub name: String,
    pub task: String,                          // e.g. "Spawn a wave of 5 enemies"
    pub initial_scene: serde_json::Value,
    pub goal_check: fn(&SceneTree) -> bool,
    pub max_actions: u32,
    pub time_limit_seconds: u32,
}

const BENCHMARKS: &[AgentBenchmark] = &[
    AGENT_TASK_SPAWN_WAVE,       // Agent spawns 5 enemies with correct positions
    AGENT_TASK_BALANCE_HP,       // Agent adjusts enemy HP to balance difficulty
    AGENT_TASK_REPAIR_BROKEN_SM, // Agent fixes a broken state machine transition
    AGENT_TASK_BEAT_LEVEL,       // Agent configures towers to beat the level
];
```

Reproducibility: same LLM model + same prompt + same seed → byte-identical pass/fail.

### Layer 4: Reference Game Integration Tests

Full tower defense tests:
```rust
#[test]
fn full_game_1000_ticks_no_crash() {
    let engine = Engine::new(Box::new(NullRenderer), 42);
    engine.start("res://games/tower_defense/scene.json").unwrap();
    for _ in 0..1000 { engine.tick(); }
}

#[test]
fn win_condition_achievable() {
    // Verifies the game is actually winnable with the right seed
}

#[test]
fn replay_regression_tower_defense() {
    // Same as Layer 2, but for the reference game specifically
}
```

## Test Infrastructure

### NullRenderer

```rust
struct NullRenderer;
impl Render for NullRenderer {
    fn render(&mut self, _: &[ComponentView], _: u64) {}
    fn viewport(&self) -> Viewport { Viewport { width: 0, height: 0 } }
    fn resize(&mut self, _: Viewport) {}
    fn shutdown(&mut self) {}
}
```

Allows running engine ticks without a terminal. Used by Layer 2-4 tests.

### Deterministic Test Harness

```rust
pub fn record_run(scene: &str, seed: u64, ticks: u32) -> Recording;
pub fn replay_run(recording: &Recording) -> ReplayResult;
pub fn run_with_hot_reload(
    scene: &str, modified: &str, reload_at_tick: u32, seed: u64, total_ticks: u32
) -> Recording;
```

## CI Requirements

- `cargo test` — all Layer 1 + 2 + 4 tests
- `cargo test --bench` — Layer 3 (agent benchmarks, may be on-demand due to LLM cost)
- `cargo clippy -- -D warnings`
- `cargo fmt --check`
- Coverage: `cargo tarpaulin` with ≥80% threshold

## Godot Mapping

| Godot | Craft |
|-------|-------|
| doctest (embedded in `tests/`) | `cargo test` (standard Rust test runner) |
| Manual GDScript test scripts | Replay regression tests (automated hash comparison) |
| No determinism guarantee | `RngState` + per-tick state hash verification |
| No agent benchmarks | 4 structured LLM benchmark tasks |
| Headless build for CI | `NullRenderer` trait object (zero-overhead, no special build) |
| No coverage target | ≥80% (PRD requirement) |
