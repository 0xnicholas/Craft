/// Independent toggle switches that compose into a determinism mode.
/// Each switch enforces a specific kind of determinism when on:
/// - **rng**: replace `math.random` with `engine.rng()` (deterministic,
///   seed-driven).
/// - **float**: lock floating-point math to deterministic results
///   (reject NaN/inf, enforce a stable rounding mode).
/// - **order**: force iteration over map/array containers into a stable
///   order (typically insertion order or sorted by key).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DeterminismSwitches {
    pub rng: bool,
    pub float: bool,
    pub order: bool,
}

/// Named determinism modes (ADR 0016 §"Determinism Mode"). Each maps to
/// a fixed switch combination; users may also compose switches directly
/// via `DeterminismSwitches`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeterminismMode {
    /// Full Lua power: direct mutation, `math.random`, coroutines,
    /// `require` any package. The default for authoring/debugging.
    Development,
    /// Locks RNG (math.random -> engine.rng) and disables os.clock so
    /// recording is reproducible.
    Recording,
    /// Locks RNG + Float + Order. Lua output is observed by the
    /// recorder but does not write back to the scene — replay ignores
    /// Lua input and replays the recorded actions.
    Replay,
}

impl DeterminismMode {
    pub fn switches(self) -> DeterminismSwitches {
        match self {
            DeterminismMode::Development => DeterminismSwitches::default(),
            DeterminismMode::Recording => DeterminismSwitches {
                rng: true,
                ..DeterminismSwitches::default()
            },
            DeterminismMode::Replay => DeterminismSwitches {
                rng: true,
                float: true,
                order: true,
            },
        }
    }
}

/// Collected engine API calls emitted by Lua scripts during a Recording
/// session. The recorder stores this with the tick and uses it for
/// replay verification (Lua output must match the recorded sequence).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordedCall {
    pub api: &'static str,
    pub args_summary: String,
}

#[derive(Debug, Clone, Default)]
pub struct RecordingLog {
    pub calls: Vec<RecordedCall>,
    pub dropped_writes: u64,
}

impl RecordingLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_call(&mut self, api: &'static str, args_summary: String) {
        self.calls.push(RecordedCall { api, args_summary });
    }

    pub fn record_dropped_write(&mut self) {
        self.dropped_writes = self.dropped_writes.wrapping_add(1);
    }

    pub fn clear(&mut self) {
        self.calls.clear();
        self.dropped_writes = 0;
    }
}

/// Lightweight state shared across the Lua closures installed by the
/// determinism layer. Holder of the active switches plus the recording
/// log that the runtime writes into.
#[derive(Default)]
pub(crate) struct DeterminismState {
    pub switches: DeterminismSwitches,
    pub mode: Option<DeterminismMode>,
    pub recording: RecordingLog,
}
