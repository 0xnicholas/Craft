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
    /// Starts recording. Returns `EngineError::Validation` if the scene contains
    /// lua_class nodes without determinism locks enabled (see ADR 0003 §"Lua Determinism Lock").
    /// This is a hard error — silent non-deterministic recording is worse than no recording.
    pub fn start(scene: &Scene, seed: u64, resources: &ResourceRegistry) -> EngineResult<Self>;
    pub fn record_tick(&mut self, tick: u32, input: &InputFrame, report: &TickReport);
    pub fn finish(self) -> Recording;
}

/// Recorder::start() calls this validation before recording begins.
/// Scans the scene tree for any node with a lua_class that has not called
/// engine.set_determinism(true) (i.e., the three locks from ADR 0003 are not all ON).
/// Returns structured validation errors listing affected nodes and which lock is missing.
fn validate_lua_determinism(tree: &SceneTree) -> EngineResult<()>;
```

The validation lives inside `Recorder::start()` and executes **regardless of entry point**:

| Caller | Path | Lock behavior |
|--------|------|---------------|
| `engine.start_recording()` (bridge/agent SDK) | Bridge → `Engine::start_recording()` → `Recorder::start()` | Agent-facing API. The bridge automatically enables all three determinism locks on `lua_class` nodes before calling `Recorder::start()`, so the agent doesn't need to manually call `engine.set_determinism(true)`. |
| `Recorder::start()` (direct, test harness) | `record_run(scene, seed, ticks)` → `Recorder::start()` | Test/benchmark API. The test harness must manually enable locks or the validation will reject the recording. This is defense-in-depth: even if the test author forgets, the validation catches it before a non-deterministic recording is created. |
| `Recorder::start()` (editor, ADR 0017) | Editor "Record" button → `Recorder::start()` | Same as test harness — editor-side code must enable locks. |

The validation is a **single gate** in `Recorder::start()`, not duplicated across callers. Any path that reaches `Recorder::start()` hits the same check. This means a recording can never be created with unlocked Lua determinism, regardless of whether it was triggered by an agent, a test, or the editor.

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

### Recording Pre-Flight: Lua Determinism Validation

ADR 0016 defines Lua's default development mode as non-deterministic (`math.random` usable, no lock). Before recording, `Recorder::start()` performs a mandatory pre-flight check:

1. Scan the scene tree for every node with `lua_class` set
2. Verify `engine.determinism_locks()` returns all three switches ON (ADR 0003)
3. If any node has Lua scripts running without determinism locks → `Recorder::start()` returns `EngineError::Validation` with structured errors listing:
   - Which node IDs have unlocked Lua scripts
   - Which locks are missing (RNG, Float, Order)
   - Suggestion: "Call engine.set_determinism(true) on all lua_class nodes before recording"

This is a **hard error**, not a warning. Silent non-deterministic recording produces replays that fail hash verification — creating false-positive "determinism bugs" that waste debugging time. The recorder refuses to create a recording it knows will be unreplayable.

For scenes without any `lua_class` nodes (pure JSON behaviors), this check passes trivially — JSON behaviors are always deterministic.
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

## Replay × Hot Reload — Cross-System Semantics

Hot reload (ADR 0009) and deterministic replay are orthogonal systems that intersect when a recording spans a hot reload boundary.

**v1 decision: Replay does NOT cross hot reload boundaries.** If a recording captured ticks 0-1000 and a hot reload occurred at tick 500, replay is only valid for tick ranges that share the same scene snapshot:

| Tick range | Scene version | Replayable? |
|------------|--------------|-------------|
| 0-499 | scene_v1.json (pre-reload) | Yes — replay from tick 0 with scene_v1 snapshot |
| 500-1000 | scene_v2.json (post-reload) | Yes — replay from tick 500 with scene_v2 snapshot + accumulated state |
| 0-1000 | Two different scenes | **No** — cross-patch replay is a v1 non-goal |

**Rationale**: Cross-patch replay requires the recording to embed the full "patch manifest" (what changed, at which tick, the old and new scene files, the old and new Lua scripts). The replay engine would need to stop at tick 500, apply the patch, verify the state hash, then continue. This is feasible but complex — deferred to v2. ADR 0009 is updated to note this constraint.

**v2 goal (explicitly deferred)**: Recordings embed `PatchEntry { tick, diff, old_scene_hash, new_scene_hash }`. Replay engine pauses at patch ticks, applies the diff, verifies the hash match, then continues.

## Agent Use Cases

1. **Counterfactual reasoning**: "Change the `enemy_spawner` state machine transition from `every_60_ticks` to `every_30_ticks`, replay the same recording, compare the state diffs at tick 500."

2. **Bug reproduction**: "The game crashed at tick 847. Play back the recording, step to tick 846, inspect the full scene state."

3. **Regression testing**: "Every PR must pass `cargo test` including replay regression tests that verify hash equality for the reference game at 1000 ticks."
