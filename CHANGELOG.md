# Changelog

Per-release notes for Craft. Versions follow the spirit of [Semantic Versioning](https://semver.org/).

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
