# ADR 0020: GPU Renderer Runs as a Subprocess

**Date**: 2026-08-06
**Status**: Accepted (retroactive — decision shipped in v0.2.0)

## Context

The editor's "run game" (F5) launches the GPU renderer (`craft-gpu`, wgpu + winit) as a **separate process**: it writes the current scene to a temp JSON file and spawns `craft-gpu <scene.json> --asset-root <asset_root>`; F8 kills the child. The game window is therefore a full, independent engine instance — not a viewport embedded in the egui process.

## Decision

**The GPU game window is a subprocess, and the handoff between editor and window is file-based (a temp scene JSON), not shared memory.**

The choice was driven by the same constraint that shaped ADR 0015 (single-threaded engine, 8ms tick budget): the editor process is already busy with egui's event loop on the main thread, and wgpu/winit insists on owning its own event loop. Running both in one process means two event loops fighting for one thread — a real problem on macOS. A separate process also isolates renderer crashes from the editor, and keeps GPU code out of the editor's tick discipline entirely.

## Considered Options

- **In-process embedding** (wgpu surface inside egui): rejected — event-loop contention on the main thread (macOS), renderer crash takes the editor down, GPU code leaks into the editor's single-threaded loop.
- **Shared-memory bridge** (editor pushes live scene state to the window): rejected for v0.2.0 — the window is a replayable unit (same scene, seed, determinism rules as any engine run); a live bridge would blur which engine owns the truth.

## Consequences

- The window's engine and the editor's engine are two instances of the same scene and can diverge — acceptable: the window is a preview run, the editor remains the source of truth.
- The window reflects the scene **at launch time**, not live edits; hot reload applies to the editor's embedded engine, not the preview window.
- Future live-preview work should add a scene-file watcher inside `craft-gpu`, not an IPC channel.
