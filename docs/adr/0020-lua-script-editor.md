# ADR 0020: Lua Script Editor

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: ADR 0016 (Lua scripting), ADR 0017 (editor architecture), ADR 0018 (panels).
This panel is the Craft equivalent of Godot's Script Editor, but for Lua 5.5.

## Context

The Lua Editor is where humans write game logic in Lua. It must provide: syntax highlighting, auto-complete (Lua stdlib + engine API), go-to-definition, error reporting with line-level diagnostics, and hot reload on save.

Godot's Script Editor provides these for GDScript via a built-in language server. Craft uses the Lua Language Server (LuaLS / sumneko-lua) via LSP protocol, plus engine-specific completion.

## Decision

**A Lua-aware text editor panel with LSP integration via Lua Language Server, plus engine API completion from schema.**

### LSP Architecture

```
┌──────────────────────────────────────┐
│ craft-editor                         │
│                                      │
│  ┌────────────────────────────┐     │
│  │ LuaEditorPanel             │     │
│  │  - egui TextEdit           │     │
│  │  - Syntax highlighting     │     │
│  │  - Auto-complete popup     │     │
│  │  - Diagnostics (squiggles) │     │
│  └──────────┬─────────────────┘     │
│             │                        │
│  ┌──────────▼─────────────────┐     │
│  │ LspManager                 │     │
│  │  - manages LSP subprocess  │     │
│  │  - stdio JSON-RPC          │     │
│  │  - merges LuaLS + engine   │     │
│  │    completions             │     │
│  └──────────┬─────────────────┘     │
│             │                        │
└─────────────┼────────────────────────┘
              │ stdio (JSON-RPC 2.0)
┌─────────────▼────────────────────────┐
│ lua-language-server (separate proc)  │
│  - Lua 5.5 syntax + stdlib          │
│  - LuaRocks package types            │
│  - Workspace diagnostics             │
└──────────────────────────────────────┘
```

The LSP manager spawns the Lua Language Server as a child process and communicates via stdin/stdout JSON-RPC. This is the same architecture VS Code uses.

### Engine API Completion

The Lua Language Server knows Lua stdlib but not Craft's engine API. The editor injects engine completions by:

1. Reading `craft-schema` output → engine API types
2. Generating a `.lua` type definition stub file from the schema
3. Feeding it to LuaLS as a workspace file

```lua
-- .craft/engine_types.lua (auto-generated, in LuaLS workspace)
--- @class Node
--- @field id string
--- @field type string
--- @field position Vec2
--- @field hp integer
--- @field speed number
local Node = {}

--- @class Engine
--- @field get_node fun(id: string): Node | nil
--- @field emit fun(signal: string, args: table)
--- @field spawn fun(type: string, parent: string | nil, components: table): string
--- @field call_system fun(name: string, args: table): any
--- @field rng fun(min: number, max: number): number
--- @field log fun(level: string, message: string, fields: table)
--- @field wait_ticks fun(n: integer)
--- @field start_coroutine fun(f: fun())
local Engine = {}

--- @class Vec2
--- @field x number
--- @field y number
local Vec2 = {}
```

This stub is regenerated every time `engine.getSchema()` changes. LuaLS picks it up and provides auto-complete for engine types.

### Panel State

```rust
// crates/craft-editor/src/panels/lua_editor.rs

pub struct LuaEditorState {
    pub open_files: HashMap<PathBuf, LuaEditorFile>,
    pub active_file: Option<PathBuf>,
    pub lsp: LuaLspHandle,
}

pub struct LuaEditorFile {
    pub content: String,
    pub path: PathBuf,
    pub diagnostics: Vec<LspDiagnostic>,    // from LuaLS
    pub is_dirty: bool,
    pub cursor_pos: TextCursor,
    pub completion_state: Option<CompletionState>,  // open auto-complete popup
}
```

### Syntax Highlighting

egui's `TextEdit` can take a custom `layouter` for coloring. The Lua editor provides:

```rust
// Lightweight regex-based syntax highlighting (no tree-sitter needed for v2)
fn highlight_lua(ui: &egui::Ui, text: &str, diagnostics: &[LspDiagnostic]) {
    // Keywords: function, end, if, then, else, for, while, return, local
    // → blue
    // Strings: "...", '...' → green
    // Comments: --... → gray
    // Numbers: 123, 1.5 → orange
    // Engine API: engine.*, node:* → purple
    // Functions: foo(...) → yellow
    // Error squiggles: from LSP diagnostics
}
```

### Auto-Complete

When the user types `.` after `node` or `engine`, the editor shows a completion popup:

```rust
fn show_completions(ui: &mut egui::Ui, file: &LuaEditorFile, state: &EditorState) {
    let word_at_cursor = file.word_at_cursor();
    let completions = state.lsp_clients.get_completions(&file.path, file.cursor_pos);

    // Merge LSP completions (Lua stdlib) with engine completions (from schema)
    let all = merge_completions(completions, engine_completions_for(word_at_cursor));

    egui::Area::new("lua_completion_popup")
        .fixed_pos(cursor_screen_pos)
        .show(ui.ctx(), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for c in &all {
                    if ui.selectable_label(false, &c.label).clicked() {
                        // Insert completion into text buffer
                    }
                }
            });
        });
}
```

### Inline Diagnostics

LSP diagnostics (from LuaLS) are rendered as red squiggly underlines:

```rust
fn render_diagnostics(text: &str, diagnostics: &[LspDiagnostic], painter: &egui::Painter) {
    for diag in diagnostics {
        // Map line/column to screen position
        let pos = line_col_to_pos(text, diag.range.start);
        // Draw red squiggly underline
        painter.line_segment([pos, pos + underline_width], (2.0, egui::Color32::RED));
    }
}
```

### Hot Reload

When the user saves a `.lua` file (Ctrl+S or auto-save):

1. If the engine is running → re-require the Lua module in the engine's Lua VM (ADR 0016)
2. Lua compile errors → shown as diagnostics in the editor (red squiggles), engine continues with old version
3. Lua runtime errors during next tick → shown in Error Panel, mapped to source file

## Rationale

1. **LuaLS is the standard Lua LSP**: Used by VS Code, Neovim, and every major Lua IDE. It handles syntax, stdlib, LuaRocks types, and workspace diagnostics. Craft just needs to spawn it as a subprocess and render its output in egui.

2. **Type stubs bridge the engine ↔ LSP gap**: The engine API is not part of Lua's stdlib. Generating a `.lua` type stub from `craft-schema` makes LuaLS understand `engine.get_node()`, `node.position`, etc. This is the same approach TypeScript uses for `.d.ts` files.

3. **Regex highlighting is sufficient for v2**: A full tree-sitter grammar is more accurate but adds complexity (compiling tree-sitter for Lua, managing grammars). Regex coloring covers 90% of visual needs.

4. **Hot reload on save matches Godot's workflow**: Godot re-parses GDScript on save. Craft re-requires the Lua module on save. Same mental model.

## Godot Mapping

| Godot Script Editor | Craft Lua Editor |
|--------------------|-----------------|
| GDScript syntax highlighting (built-in parser) | Lua 5.5 highlighting (regex + LSP semantic tokens) |
| Auto-complete from ClassDB | Auto-complete from LuaLS (stdlib) + engine stubs (schema) |
| Error highlighting (compile errors inline) | LSP diagnostics (red squiggles from LuaLS) |
| Ctrl+Click go-to-definition | LSP `textDocument/definition` |
| Script reload on save | Hot reload: re-require Lua module in VM |
| Signal connection dialog (visual link) | LSP auto-complete for `on_signal` function signatures |
| No external LSP (built-in) | Lua Language Server subprocess |
| No engine type stubs (ClassDB is runtime) | Auto-generated `.craft/engine_types.lua` from schema |
