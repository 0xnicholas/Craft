# ADR 0017: Editor Architecture — egui Desktop App with Embedded Engine

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: Godot editor (`editor/`, 646 files, embedded model). Craft editor is v2 sub-project.

## Context

The editor is a Rust desktop application for human authoring of Craft games. It must provide: scene tree editing, component inspector, behavior editing (JSON + Lua), terminal preview, file management, and agent Copilot integration. The editor runs the engine in-process for live preview with hot reload.

Godot's editor is compiled into the same binary as the engine — the editor IS a Godot game built with Godot's own GUI system. Craft follows this embedded model but uses egui instead of building a custom GUI toolkit.

## Decision

**`craft-editor` crate: egui + eframe desktop app that links `craft-kernel` directly. File-based editing with embedded engine preview.**

### Crate Structure

```
crates/craft-editor/
├── Cargo.toml
└── src/
    ├── main.rs              # Entry point: parse CLI args, launch eframe
    ├── app.rs               # eframe::App impl — main frame loop
    ├── state.rs             # EditorState — persistent across immediate-mode frames
    ├── panels/
    │   ├── mod.rs           # Panel trait + docking layout
    │   ├── scene_tree.rs    # Node hierarchy tree view
    │   ├── inspector.rs     # Component form editor
    │   ├── behavior_editor.rs # JSON+LSP behavior editor
    │   ├── lua_editor.rs    # Lua script editor (LSP + syntax highlight)
    │   ├── terminal_preview.rs # ANSI terminal emulator panel
    │   ├── file_browser.rs  # File system dock
    │   └── agent_panel.rs   # Copilot sidebar (see ADR 0019)
    ├── lsp/
    │   ├── mod.rs           # LSP client abstraction
    │   └── schema_lsp.rs    # JSON Schema → auto-complete provider
    └── engine.rs            # Embedded Engine handle + hot reload bridge
```

### Dependencies

```toml
[dependencies]
craft-kernel = { path = "../craft-kernel" }
craft-lua = { path = "../craft-lua" }
craft-schema = { path = "../craft-schema" }
egui = "0.31"
eframe = "0.31"
egui_dock = "0.14"          # docking/tab system
rfd = "0.15"                 # native file dialogs
notify = "7"                 # file watcher for hot reload
serde_json = "1"
```

### Engine Integration

The editor holds an `Engine` instance directly — no serialization, no WebSocket, no IPC:

```rust
// crates/craft-editor/src/engine.rs
use craft_kernel::Engine;

pub struct EditorEngine {
    engine: Engine,
    is_running: bool,
    tick_timer: Instant,
    scene_path: Option<PathBuf>,
}

impl EditorEngine {
    pub fn new() -> Self {
        let renderer = Box::new(EditorRenderer::new()); // ANSI → texture for egui
        Self { engine: Engine::new(renderer, rand::random()), is_running: false, ... }
    }

    pub fn run_scene(&mut self, path: &Path) -> EngineResult<()> {
        self.engine.start(&format!("res://{}", path.display()))?;
        self.is_running = true;
        Ok(())
    }

    pub fn tick_preview(&mut self) {
        if self.is_running {
            self.engine.tick();
        }
    }

    pub fn hot_reload(&mut self) -> EngineResult<SceneDiff> {
        self.engine.reload()
    }
}
```

### Panel Docking

`egui_dock` provides tab-based panel layout (like Godot's dock system):

```
┌──────────┬──────────────────────────┬───────────┐
│ Scene    │                          │ Inspector │
│ Tree     │    Terminal Preview      │           │
│          │    (ANSI render)         │           │
│          │                          │           │
├──────────┤                          ├───────────┤
│ File     │                          │ Agent     │
│ Browser  │                          │ Copilot   │
│          │                          │           │
└──────────┴──────────────────────────┴───────────┘
         ┌──────────────────────────┐
         │ Behavior Editor (JSON)   │
         │ or Lua Editor            │
         │ (central tab area)       │
         └──────────────────────────┘
```

Panels are user-rearrangeable, resizable, and tabbable — same as Godot's docking system.

### File-Based Editing Model

Like Godot, the editor works on files:

1. **File Browser** shows project tree (`games/tower_defense/`)
2. Double-click `scene.json` → opens in **Scene Tree** + **Inspector**
3. Double-click `enemy.lua` → opens in **Lua Editor**
4. Press **Run (F5)** → engine starts with current scene.json → **Terminal Preview** panel shows live output
5. Editing scene.json during preview → file watcher triggers hot reload

The editor never operates on the engine's live state directly (except through hot reload). All edits go to disk first, then engine picks them up.

### Rendering the Terminal Preview

The embedded engine's `Render` trait output (ANSI escape codes) is captured and rendered into an egui texture:

```rust
// crates/craft-editor/src/panels/terminal_preview.rs
struct TerminalPreview {
    grid: Vec<Cell>,              // char grid from parser
    texture: egui::TextureHandle, // GPU texture for rendering
}

impl TerminalPreview {
    fn update(&mut self, renderer: &EditorRenderer) {
        // 1. Parse ANSI output from renderer
        // 2. Build char grid with foreground/background colors
        // 3. Upload to GPU texture (or draw as egui text)
        self.grid = renderer.current_grid();
    }
}
```

## Rationale

1. **Embedded engine eliminates transport overhead**: No WebSocket, no serialization, no async. The editor calls `engine.tick()` as a function call. This is how Godot works and it's the right model.

2. **egui is the pragmatic Rust GUI choice**: Immediate mode matches the frame-by-frame nature of game editors. Every frame, the editor reads `EditorState`, draws UI, and captures user actions. State lives separately — egui just renders it.

3. **`egui_dock` gives Godot-style docking for free**: Tabbed, draggable, resizable panels without building a dock system from scratch.

4. **File-based editing preserves the agent path**: Agent writes JSON files. Editor opens the same files. Both paths converge on the same data format. No "editor state" that can drift from the agent's file-based state.

5. **Separate crate keeps engine and editor decoupled**: `craft-editor` depends on `craft-kernel` but not vice versa. The engine has no knowledge of the editor. This means CI can run `cargo test -p craft-kernel` without compiling egui.

## Godot Mapping

| Godot Editor | Craft Editor |
|-------------|-------------|
| `editor/editor_node.cpp` (main editor singleton) | `craft-editor/src/app.rs` (eframe::App impl) |
| `scene/gui/` (130 files, custom GUI toolkit) | egui + eframe (Rust ecosystem) |
| Docking system (tabs, splitters) | `egui_dock` crate |
| Scene tree dock | `panels/scene_tree.rs` |
| Inspector | `panels/inspector.rs` |
| Script editor (GDScript) | `panels/lua_editor.rs` (Lua) |
| 2D/3D viewport | `panels/terminal_preview.rs` (ANSI) |
| FileSystem dock | `panels/file_browser.rs` |
| Editor → engine: direct C++ call | Editor → engine: direct Rust fn call (same binary) |
| Hot reload: script only | Hot reload: scene.json + .lua + resources |
