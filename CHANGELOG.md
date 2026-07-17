# Changelog

Per-release notes for Craft. Versions follow the spirit of [Semantic Versioning](https://semver.org/).

## [v0.4.0] — 2026-07-18

Editor E4 polishes the editor experience with undo/redo, shortcuts, context menus, drag-drop, and a visual overhaul.

### Added

- **Undo/Redo**: command-pattern module (100 steps) wired into Inspector, SceneTree, BehaviorEditor, and AgentPanel. Ctrl+Z / Ctrl+Shift+Z.
- **Keyboard shortcuts**: Ctrl+1-5 (panel focus), Ctrl+Shift+A (add child node), Ctrl+Z/Ctrl+Shift+Z (undo/redo).
- **Context menus**: scene tree node (Add Child, Duplicate, Delete), file browser (Open, Delete, New File/Folder).
- **Drag-drop**: scene tree reparent/reorder, file browser .lua → node lua_class attachment.
- **Visual design language**: egui Visuals override with Craft dark palette (#1A1A2E background, #6C9FFF accent, 4px scrollbars, 0.1s animations). Node type emoji + color icons (🔴 Enemy, 🔵 Tower, 🟢 Player).

## [v0.3.0] — 2026-07-18

Editor E3 adds the Agent Copilot — an AI-powered chat panel for scene authoring.

### Added

- **Agent panel**: chat UI with context bar, streaming text, message bubbles, diff preview modal.
- **AgentClient**: reqwest-based HTTP client with SSE streaming, non-streaming tool call detection, and AtomicBool concurrency guard.
- **ToolRegistry**: 6 LLM tools (lint, dry_run, explain, read_scene, read_node, propose_diff) executed locally in the UI thread.
- **ContextBuilder**: EditorState → AgentContext injection into system prompts.
- **explain_node**: new `craft-kernel` primitive returning compact structured JSON for LLM consumption.
- **Thread model**: blocking reqwest calls on background `std::thread`, mpsc channel drain per-frame in UI thread. Max 3-round tool call loop.

## [v0.2.1] — 2026-07-17

Editor E2 adds behavior and Lua editing to `craft-editor`.

### Added

- **Behavior editor**: inline JSON sub-editor in Inspector (`▶ edit JSON`) with schema-aware auto-complete (Ctrl+Space) and inline validation errors. Standalone `.behavior.json` editor in `BehaviorEditorPanel`.
- **Lua editor**: replaces stub with full editor. Spawns `lua-language-server` as a subprocess when available; falls back to local editing otherwise. Auto-complete, inline diagnostics, and hot-reload on save via `LuaRuntime::reload_class`.
- **Engine integration**: `EditorEngine` gains a `LuaRuntime` field. `load_scene_file` reads `[lua] modules_dir` from `craft.toml`.
- **JsonPathLsp**: schema-driven JSON tokenizer, validator, and completion engine.
- **LSP framing + LspClient**: sync mpsc transport; JSON-RPC over framed stdio.
- **Watcher**: tracks `.lua` and `.behavior.json` external changes (in addition to scene.json from E1).
- **lua_stub_gen**: writes `.craft/engine_types.lua` (LuaCATS-annotated) for LuaLS engine API support.

## [v0.2.0] — 2026-07-16

Editor E1 establishes the first desktop editor milestone for Craft.

### Added

- `craft-editor` desktop application scaffold built with egui and eframe.
- `EditorEngine` lifecycle for loading, running, stopping, stepping, and reloading scenes.
- Scene Tree, Inspector, Files, and Terminal Preview panels, with stubs for the E2 and E3 editors.
- Persistence for dock layout and recent projects.
