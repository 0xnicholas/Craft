# Craft v0.3.0: 2D Game Surface + Agent-Built Platformer — Design

**Date**: 2026-08-06
**Status**: Draft (proposed for v0.3.0)
**Decided by**: grilling session 2026-08-06 (all seven decisions confirmed)

## Summary

v0.3.0 completes the 2D game-making surface — tilemaps, sprite animation, HUD text, audio control — and proves the engine's core thesis by having an LLM agent build the reference game: a 2D platformer. The agent's build is formalized as a new full-game benchmark task: reproducible in CI via the stub backend, verified once against a live LLM run.

## Motivation

v0.2.0 shipped the runtime backends (GPU sprites, physics, particles, audio) but left the authoring surface thin: no tilemap, no sprite animation, no mouse/gamepad, no text/UI — a platformer or Zelda-like cannot be built. Meanwhile the engine's thesis — "agents author games as first-class operations" — has only been validated on micro-tasks (single behaviors). v0.3.0 closes both: the missing 2D primitives, and the first agent-built complete game.

## Decisions

1. **Reference game: 2D platformer** (Mario-style). The one form that forces all four missing primitives (tilemap levels, sprite animation, input, HUD text) and has a clear headless acceptance: the agent's build must reach the goal. Art footprint is minimal: one tileset + a character sheet + a few enemies.

2. **Asset pipeline: JSON pixel maps + a deterministic asset generator.** Sprites are authored as JSON pixel maps (rows of color strings); a generator bakes them into committed PNGs (and simple WAV tones for sfx). The agent only writes data — the harness needs no new tools for binaries; `craft-gpu`'s texture path is untouched; humans can later replace any PNG without breaking the data flow.

3. **Surface scope — IN**: tilemap (grid + tileset atlas); sprite animation (frame list, fps, loop); HUD bitmap text (5×7 digits + minimal charset); audio polish (volume + loop on signal-triggered playback); input injection in the eval harness (the engine's `InputState` already exists and is headless-driveable).

4. **Surface scope — DEFER**: save/load, gamepad/mouse, rich text/dialogue, tilemap editor tooling.

5. **Agent build form: 5th benchmark task.** `craft-eval` gains a full-game task type: the agent emits scene.json + behaviors + Lua + pixel maps; the pipeline runs generate-assets → headless run with scripted input → assert win condition. The stub backend's reference solution **is** the shipped game, so CI is always green; one live run is recorded per release.

6. **Live benchmark policy.** Backend: DeepSeek (already wired, has precedent). Results recorded in `benchmarks/RESULTS.md` (model, prompt hash, seed, date, per-task verdict). API key stays in `.env`, never committed. CI gate = stub backend only. Reproducibility standard: M10 — same (model, prompt, seed) → same pass/fail across two runs.

7. **Opening work order.** ① Physics-in-loop replay test — hard prerequisite: `craft-replay` and the 1000-tick replay test currently run **without** `PhysicsSystem`; determinism with physics is unverified and every platformer/benchmark acceptance depends on it. ② Revise ADR 0004 (trait has 5 methods, not 4). ③ Write ADR 0020 (GPU subprocess boundary). ④ Flip v3 GPU spec to Accepted. *(Items ②–④ already landed 2026-08-06.)*

## Acceptance

- [ ] Physics-in-loop replay test: byte-equal state hash across reruns with `PhysicsSystem` registered
- [ ] Platformer benchmark task (`05_platformer`): stub backend passes in CI; win condition asserted in a headless scripted-input run
- [ ] One live DeepSeek run of all 5 tasks recorded in `benchmarks/RESULTS.md`
- [ ] Platformer ships as a playable game (`games/platformer/`) with generated PNG/WAV assets
- [ ] `cargo test` / `cargo clippy -D warnings` / `cargo fmt --check` all clean

## Risks / Open Items

- **Tile collision** (decide at planning): extend `craft-physics` with tilemap collision vs. model solid tiles as static nodes through the existing AABB path (~600 tiles within the 1000-node budget).
- **HUD font**: bitmap-font generation quality; digits + minimal charset is the bar, not typesetting.
- **Audio**: volume/loop parameter shape on signal-triggered playback; music loop timing under the 8ms tick budget (ADR 0015).

## Related

- ADR 0006 (determinism), ADR 0015 (tick budget), ADR 0020 (GPU subprocess), `CONTEXT.md` (pixel map, tilemap, benchmark task), `docs/CHANGELOG.md`
