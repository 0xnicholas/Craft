# ADR 0006: Replay System — Deterministic Recording & Hash-Verified Replay

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: Godot has no built-in replay system. This is a Craft-native capability.

## Context

PRD success criterion: "Reference game replays to hash-equal state." The replay system must record all inputs, capture per-tick state hashes, and verify byte-identical reproduction. Agents use replay for counterfactual reasoning: "What would have happened if state machine X had a different transition?" This requires the engine to be fully deterministic.

## Decision

**`craft-replay` is a separate crate that depends on `craft-kernel`. Recording and replay operate on a defined interface (`Replayable` trait) without modifying the kernel's tick loop.**

### Recording Format

```rust
pub struct Recording {
    pub meta: RecordingMeta,
    pub frames: Vec<Frame>,
}

pub struct RecordingMeta {
    pub scene_snapshot: serde_json::Value,        // frozen copy at record start
    pub resource_snapshots: HashMap<String, Value>, // path → resource content
    pub seed: u64,
    pub tick_rate: u32,
    pub total_ticks: u32,
}

pub struct Frame {
    pub tick: u32,
    pub input: InputFrame,
    pub state_hash: u64,                           // hash after applying all mutations
    pub signals_emitted: Vec<SignalRecord>,
}
```

### Recorder

```rust
pub struct Recorder {
    recording: Recording,
    is_recording: bool,
}

impl Recorder {
    pub fn start(scene: &Scene, seed: u64, resources: &ResourceRegistry) -> Self;
    pub fn record_tick(&mut self, tick: u32, input: &InputFrame, report: &TickReport);
    pub fn finish(self) -> Recording;
}
```

### Replay Runner

```rust
pub struct ReplayRunner {
    engine: Engine,               // independent Engine instance
    recording: Recording,
    current_tick: u32,
}

impl ReplayRunner {
    pub fn new(recording: Recording) -> Self;
    pub fn tick(&mut self, mode: ReplayMode) -> ReplayEvent;
    pub fn snapshot_at(&self, tick: u32) -> Option<&Frame>;
    pub fn diff(&self, tick_a: u32, tick_b: u32) -> StateDiff;
}
```

### Core Invariant

```
Given: scene_snapshot + seed + input_log + resource_snapshots
Replay yields: byte-identical state at every tick
Verified by: per-tick state_hash comparison
```

Any hash mismatch is a **determinism bug** — tracked and fixed before v1 ships.

## Rationale

1. **Separate crate for storage concerns**: The recording codec has its own serialization format, file I/O, and versioning. It should not live inside the kernel.

2. **`Replayable` trait defines the contract**: Kernel exposes `fn seed()`, `fn input_at_tick()`, `fn scene_snapshot()`. Replay crate consumes this interface. No coupling in the reverse direction.

3. **Resource snapshots embedded in recordings**: A replay must be independent of subsequent `registerResource()` calls. Embedding resource content at record time makes replays self-contained.

4. **Hash is computed per-tick, not just end-to-end**: This enables pinpointing exactly which tick diverged, making determinism bugs debuggable.

## Determinism Requirements

| Component | Determinism Mechanism |
|-----------|----------------------|
| RNG | `RngState` with seeded `StdRng` (ChaCha). All engine randomness goes through it. |
| Tick ordering | Declaration order in scene JSON (stable across reloads). |
| Float operations | v1 uses `f64` consistently. No `f32`/`f64` mixed arithmetic. |
| Signal delivery | Next-tick queue (ADR 0003). No same-tick cascades. |
| Resource resolution | `ResourceRef` stores `snapshot_version`. Replay uses recorded version, not current. |
| Tick rate | Recording stores `tick_rate`. Replay runs at recorded rate regardless of `setTickRate()`. |

## Agent Use Cases

1. **Counterfactual reasoning**: "Change the `enemy_spawner` state machine transition from `every_60_ticks` to `every_30_ticks`, replay the same recording, compare the state diffs at tick 500."

2. **Bug reproduction**: "The game crashed at tick 847. Play back the recording, step to tick 846, inspect the full scene state."

3. **Regression testing**: "Every PR must pass `cargo test` including replay regression tests that verify hash equality for the reference game at 1000 ticks."
