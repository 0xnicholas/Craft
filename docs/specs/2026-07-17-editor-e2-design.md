# Editor E2 — Behavior + Lua Editors

**Date:** 2026-07-17
**Status:** Approved (pending user review)
**Author:** brainstorming session
**Scope:** v2 Editor, milestone E2 only

## Context

E1 (commit `3937cd0` through `f84df56`) shipped a working `craft-editor` shell
with 7 panels (4 functional, 3 stubs), persistence, and quality gates. The
two editor panels that remain stubs are the focus of E2:

- `BehaviorEditorPanel` — needs to host an inline JSON editor for the
  `Node.behaviors` array entries, with schema-aware auto-complete and inline
  validation.
- `LuaEditorPanel` — needs syntax highlighting, LuaLS-driven auto-complete,
  diagnostics, and hot reload on save.

E2 also adds a small dedicated surface for editing **standalone behavior files**
that are not yet embedded in any scene (used by authors writing new behaviors
to be wired into `Node.behaviors` later). Standalone behaviors live at
`<project>/behaviors/<name>.behavior.json` and are referenced from
`Node.behaviors[i].id` or inlined as objects — see "Standalone behavior files"
below.

E2 deliberately does **not** introduce undo/redo, multi-file tabs, or drag-drop.
Those land in E3/E4 per ROADMAP.

## Decisions (E2-specific)

These supplement (do not replace) ADRs 0017, 0018, 0019.

| Decision | Choice | Rationale |
|---|---|---|
| Behavior storage | Inline in `scene.json` `Node.behaviors[]` | Existing path; no new FS abstraction; one canonical source |
| Standalone behavior authoring | New `<project>/behaviors/<name>.behavior.json` files (offline authoring) | Lets authors draft reusable behaviors without a target scene |
| Behavior editor placement | Inline expandable per-row in `InspectorPanel` | Keeps authoring context next to the node using it |
| Standalone behavior editor | Dedicated `BehaviorEditorPanel` (currently a stub) loads `.behavior.json` files from File Browser | Separates "edit a node's behavior" from "edit a reusable behavior file" |
| Lua editor scope | Full LuaLS integration + graceful fallback | Best UX when LSP present; doesn't break when absent |
| Schema-LSP impl | Hand-rolled JSONPath + `craft-schema::get_full_schema()` lookup | Zero new deps, fits E1's library + binary target shape |
| Multi-file tabs | Single buffer per panel | Matches E1 panel simplicity; defer to E3+ |
| LuaLS discovery | Auto-spawn if found on `$PATH`; warn + local-only fallback | Honest about state; doesn't gate the panel |
| Syntax highlighting (JSON) | Regex-based (keys / strings / numbers / booleans / errors) | Sufficient for E2; tree-sitter deferred |
| Syntax highlighting (Lua) | Regex-based (keywords / strings / numbers / comments) | Sufficient for E2; tree-sitter deferred |
| Hot reload trigger | Ctrl+S in Lua editor; `LuaRuntime::reload_class(name, source)` | Already exposed in `craft-lua` (no new kernel API needed) |
| `engine_types.lua` regeneration | On schema version bump + on editor startup | Cheap; keeps LuaLS completions in sync |

## Out of scope (deferred)

- Undo/redo history (E4)
- Multi-file tabs / split editors (E3+)
- Drag-drop behavior blocks onto nodes (E4)
- Behavior library / snippet templates (E3+)
- Tree-sitter syntax highlighting (v3)
- LuaLS workspace symbols (call hierarchy, find references) — only basic
  completion + diagnostics + go-to-definition in E2
- Auto-format Lua source (`stylua` integration) — E3
- Debugger / breakpoints — E3+
- Auto-converting standalone behaviors into inline `Node.behaviors` entries — manual
  authoring workflow stays explicit in E2

## 1. Crate structure & dependencies

```
crates/craft-editor/src/
├── json_path.rs             # NEW — JSONPath parser + schema lookup engine
├── lua_lsp.rs               # NEW — LuaLS subprocess client (JSON-RPC framed stdio)
├── lua_stub_gen.rs          # NEW — writes craft_schema::lua_engine_stub() to .craft/engine_types.lua
├── lsp/
│   └── mod.rs               # NEW — generic LSP framing + types
├── panels/
│   ├── inspector.rs         # MOD — inline behavior JSON sub-editor
│   ├── lua_editor.rs        # MOD — replaces stub
│   └── behavior_editor.rs   # MOD — replaces stub (loads standalone .behavior.json)
└── state.rs                 # MOD — LuaEditorState + BehaviorEditState + inspector.behavior_edits

crates/craft-schema/src/
└── lua_engine_stub.lua      # NEW — checked-in LuaCATS-annotated stub asset; loaded via include_str!

crates/craft-editor/tests/
├── e2_json_path.rs          # NEW — unit tests for JsonPathLsp
├── e2_lua_stub_gen.rs       # NEW — unit tests for stub generation
├── e2_inspector_behavior.rs # NEW — integration: schema flags invalid behavior edits
├── e2_lua_editor.rs         # NEW — integration: LuaLS subprocess lifecycle + skip-if-missing
└── e2_panels_kittest.rs     # NEW — render smoke tests for behavior + lua panels (no PNG snapshots;
                              #      same approach as E1's e1_panels_kittest.rs)
```

### `Cargo.toml` dependencies

Add to `[dependencies]` in `crates/craft-editor/Cargo.toml`:

```toml
craft-lua = { path = "../craft-lua" }
which = "7"            # locate lua-language-server
```

`craft-lua` is required so `EditorEngine` can hold a `LuaRuntime` field
(§7.1). `which = "7"` is workspace-addable. No new direct dependency on
`uuid` — LSP request IDs use `std::sync::atomic::AtomicI64` for sequencing.

Add to `[dev-dependencies]`:

```toml
temp_env = "0.3"       # already present (E1)
```

### Workspace rule (no exceptions)

`craft-kernel`, `craft-lua`, and `craft-schema` MUST NOT depend on `craft-editor`.

## 2. JSON schema-LSP (JsonPathLsp)

### 2.1 Data model

```rust
// crates/craft-editor/src/json_path.rs

use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct JsonPathLsp {
    schema_root: Value,
    /// Path of the JSON file currently being edited. Used for `$ref` resolution
    /// against the on-disk craft-schema if `schema_root` is a wrapper.
    schema_root_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,   // e.g., type hint
    pub insert_text: String,      // may differ from label (with snippet body)
    pub insert_range: Range<usize>, // byte range in buffer to replace
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Property,
    Value,
    Snippet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchemaError {
    pub line: u32,    // 0-indexed
    pub col: u32,     // 0-indexed, byte offset within line
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity { Error, Warning }
```

### 2.2 Public API

```rust
impl JsonPathLsp {
    /// Build a new LSP. `schema_root` typically comes from
    /// `craft_schema::get_full_schema()`.
    pub fn new(schema_root: Value) -> Self;

    /// Parse the JSON buffer best-effort and return the path segments
    /// leading up to the cursor. Handles incomplete JSON (trailing commas,
    /// unclosed braces) by stopping at the last fully-formed token.
    pub fn path_at(&self, buffer: &str, cursor_byte: usize) -> Vec<PathSeg>;

    /// Return completion items for the given path + context (key vs value).
    pub fn complete(&self, path: &[PathSeg], ctx: CursorCtx) -> Vec<Completion>;

    /// Validate the buffer and return errors.
    pub fn validate(&self, buffer: &str) -> Vec<SchemaError>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathSeg { Key(String), Index(usize) }

#[derive(Debug, Clone, PartialEq)]
pub struct CursorCtx {
    pub in_object_key: bool,
    pub in_object_value: bool,
    pub partial_token: String,   // what's already typed at cursor
}
```

### 2.3 Implementation outline

`path_at` does a single forward pass with a tiny recursive-descent JSON tokenizer
(not a full parser). On unclosed braces/brackets it stops cleanly and returns
the path leading up to the incomplete region. The tokenizer recognizes:

- `{` and `}` — object boundaries
- `[` and `]` — array boundaries
- `"…"` strings (including `\"`, `\\`)
- Numbers, booleans, null
- `:` separating key/value
- `,` separating items

After each token, the path state is updated:

```
start      -> []
{          -> []
key"foo":  -> [Key("foo")]
:value     -> [..., Key("foo")]
,key"bar": -> [..., Key("bar")]
,[0]       -> [..., Index(0)]
```

`complete` walks the schema via the path. At object context, returns
`schema.properties.keys()`. At enum value, returns `schema.enum`. For typed
fields (`type: "string"` / `"integer"` / etc.) returns a single snippet with
a placeholder default (`""`, `0`, `false`, `{}`, `[]`).

`validate` runs a single `serde_json::from_str::<Value>` first (full parse).
On parse error, returns a single `SchemaError { line, col, "invalid JSON: …" }`.
On parse success, walks the parsed value against the schema and reports:

- Missing required properties
- Type mismatches
- Unknown properties (warning, not error)
- Enum violations

### 2.4 Cursor→line/col mapping

`SchemaError.line/col` is computed from the byte offset using a precomputed
`Vec<usize>` of line-start offsets built once per `validate` call.

## 3. Behavior JSON editor (Inspector inline + standalone panel)

### 3.1 Inspector inline sub-editor

`InspectorPanel::show` extends each behavior row with an expand affordance:

```
[behavior 0] { "kind": "set_state", "params": { … } }   ▶ edit JSON
```

When expanded, the editor occupies the next ~12 rows. State:

```rust
// crates/craft-editor/src/state.rs (additions)

pub struct BehaviorEditState {
    pub node_id: String,
    pub behavior_idx: usize,
    pub buffer: String,                  // raw JSON text
    pub parsed: Option<Behavior>,
    pub errors: Vec<SchemaError>,
    pub completion: Option<CompletionPopup>,
    pub dirty: bool,
}

// In PanelsState::inspector:
pub struct InspectorState {
    pub search_text: String,
    pub add_component_menu_open: bool,
    pub expanded_behaviors: HashSet<(String, usize)>,
    pub behavior_edits: HashMap<(String, usize), BehaviorEditState>,
}
```

**Per-keystroke behavior:**

1. Append character to `buffer`.
2. Mark `dirty = true`.
3. Schedule debounced validation (250 ms via `InspectorState.last_validation_ms: u64`
   which the panel checks against elapsed wall-clock time). On fire:
   - Call `json_path.validate(&buffer)` → update `errors`.
   - Try `serde_json::from_str::<Behavior>` → update `parsed`. On success the
     in-memory `SceneDef` is NOT mutated here (that's `Apply` only, see §3.2).

`InspectorState.last_validation_ms: u64` is a new field added in this
section (it lives on `InspectorState` because the debounce is per-panel,
not global). The keystroke handler does NOT emit `PanelAction::SetStatus` on
every character — that would spam the status bar. Status messages are
emitted only on Apply success / failure.

**Ctrl+Space** opens `CompletionPopup` at cursor. Arrow keys + Enter inserts.

**Syntax highlighting:** `TextEdit::layouter` closure feeds each line through
`highlight_json_line(text, errors) -> Vec<egui::text::LayoutJob>`. Errors
emerge as red underlines beneath the offending span.

**Acceptance at this level:** an invalid edit shows a red marker; a valid edit
shows `valid · {kind} = {name}` in the footer.

### 3.2 Apply model (separate from edit)

**Note on ADR divergence:** ADR 0018's Behavior Editor description shows only
`Ctrl+S → SaveFile` (one-step save). E2 introduces a two-step Edit → Apply
model so authors can stage partial edits without immediately mutating the
scene def. This is additive — ADR 0018's `Ctrl+S` behavior still works at the
file level once Apply has been performed.

Two distinct operations:

- **Edit (always on):** updates `buffer` + `errors`. Mutates NO scene state.
- **Apply (`Apply` button or Ctrl+Enter — note Ctrl+S is reserved for SaveScene
  to stay consistent with the E1 keyboard map; see "Ctrl+S semantics" below):**
  1. If `parsed.is_some()` AND `errors` is empty (or only warnings):
     - Mutate `scene.def.nodes[node_id].behaviors[behavior_idx]` in place.
     - `SceneState.is_dirty()` will now return `true` because the hash of the
       mutated `scene.def` no longer matches `last_saved_hash`. The hash is
       intentionally NOT updated here — `last_saved_hash` is updated only by
       `SaveScene` (`EditorState::save_dirty()` writes the file and refreshes
       the hash in one step, matching E1's existing semantics).
     - Emit `PanelAction::SetStatus("applied behavior {node_id}#{behavior_idx}")`.
  2. If `parsed.is_none()` or errors contains an Error:
     - Emit `PanelAction::SetStatus("cannot apply: {N} errors")`.

The `Apply` button only appears when `errors` are empty (disabled otherwise).
This keeps "type fast" safe — invalid intermediate states never reach the
scene def.

**Ctrl+S semantics in the behavior editor:** Ctrl+S does NOT apply the
inline behavior edit (Ctrl+S globally maps to `PanelAction::SaveScene`,
which writes the entire `scene.json` to disk). Two outcomes:

- If the inline editor is `dirty` AND valid AND not yet applied: `Ctrl+S`
  emits `PanelAction::SetStatus("behavior modified — press Apply first")`
  (does NOT save). This protects authors from losing un-applied edits to disk.
- If the inline editor is `dirty` AND `parsed` is set (already applied):
  `Ctrl+S` writes the scene to disk as normal, picking up the applied change.

The standalone `BehaviorEditorPanel` does NOT have this guard — there is no
separate Apply step for standalone files (§3.3), so Ctrl+S there is the
direct save.

### 3.3 Standalone behavior files (`BehaviorEditorPanel`)

A second route: authors may author reusable behaviors as standalone files.
File format:

```json
{
  "$schema": "https://craftengine.dev/schema/behavior.v1.json",
  "kind": "behavior",
  "id": "tower.target_priority",
  "description": "…",
  "behavior": { "kind": "set_state", "params": { "state": "firing" } }
}
```

Saved at `<project_root>/behaviors/<id>.behavior.json`. Authoring flow:

- File Browser double-click `.behavior.json` → `BehaviorEditorPanel` activates
  with that file as its single buffer.
- Same `JsonPathLsp`-driven editing + Apply → file write.
- "Apply" in this panel writes the file directly (no scene def to mutate).
- "Bind to node…" affordance is **not** in E2 (E3+).

`id` is metadata-only in E2: the file is a self-contained behavior object;
no `Node.behaviors[i]` entry references it by `id` yet. The mechanism for
referencing a standalone behavior from `Node.behaviors[i]` (e.g., a
`BehaviorRef { ref: "tower.target_priority" }` variant) is deferred to
E3+. E2 ships only the inline behavior editing path; the standalone path
is for offline authoring + future hookup.

`BehaviorEditorState`:

```rust
pub struct BehaviorEditorState {
    pub path: Option<PathBuf>,
    pub buffer: String,
    pub errors: Vec<SchemaError>,
    pub completion: Option<CompletionPopup>,
    pub dirty: bool,
}
```

### 3.4 Stale-load warning

If the file watched by `Watcher` changes externally while the editor is open,
the E1 flow prompts `[Reload] [Keep mine]`. The same flow applies to both
behavior editor surfaces.

## 4. Lua editor

### 4.1 LSP client architecture

```rust
// crates/craft-editor/src/lua_lsp.rs

use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::mpsc;
use std::io::{BufRead, Write, Read};

pub struct LspClient {
    child: Child,
    stdin: std::process::ChildStdin,
    /// Inbound messages: server-to-client responses and notifications. All
    /// responses include their `id`; notifications have `id = None`.
    stdout_rx: mpsc::Receiver<LspMessage>,
    /// Outbound completions: completion request id → consumer's oneshot
    /// sender. Stored in an `Arc<Mutex<…>>` so the reader thread can resolve
    /// responses to their requesters.
    pending: Arc<Mutex<HashMap<i64, mpsc::SyncSender<LspResponse>>>>,
    next_id: AtomicI64,
    workspace_root: PathBuf,
}

pub struct LuaEditorState {
    pub current_path: Option<PathBuf>,
    pub buffer: String,
    pub dirty: bool,
    pub lsp: Option<LspClient>,
    pub diagnostics: Vec<LspDiagnostic>,
    pub fallback_mode: bool,            // true if LuaLS not found
    pub completion: Option<CompletionPopup>,
    pub last_validation_ms: u64,        // debounce timer for diagnostics
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: u32,    // 0-indexed
    pub col: u32,     // 0-indexed (LSP uses UTF-16 code units; we convert)
    pub end_line: u32,
    pub end_col: u32,
    pub severity: Severity,
    pub message: String,
}
```

**Sync transport, no tokio dependency.** The LSP client uses
`std::sync::mpsc` exclusively:

- Inbound messages: reader thread posts every framed message to
  `stdout_rx`.
- Outbound request→response correlation: `pending` is wrapped in
  `Arc<Mutex<…>>`. When the reader thread sees a response (message with
  `id` matching a pending entry), it locks the map, removes the entry, and
  sends the `LspResponse` on the per-request `mpsc::SyncSender`. Callers
  hold the `mpsc::Receiver` and `recv_timeout` for the response.

This deliberately avoids `tokio::sync::oneshot`, `tokio::sync::Mutex`, and
async runtimes — keeping `craft-editor`'s dependency footprint minimal
and consistent with ADR 0015 (engine is single-threaded).

### 4.2 Subprocess lifecycle

`LspClient::spawn(workspace_root)`:

1. Use `which::which("lua-language-server")`. Fall back to checking
   `~/.local/bin/lua-language-server`, `/opt/homebrew/bin/lua-language-server`,
   `/usr/local/bin/lua-language-server`. If none found, return
   `Err(LspError::NotFound)` — caller sets `fallback_mode = true`.
2. `Command::new(path).arg("--stdio").stdin(Stdio::piped()).stdout(Stdio::piped())`.
3. Spawn reader thread that reads `Content-Length: N\r\n\r\n…` framed JSON-RPC
   messages and posts to `stdout_rx`.
4. Send `initialize` request with `rootUri = file://{workspace_root}`.
5. Send `initialized` notification.
6. Send `workspace/didChangeConfiguration` with LuaLS config
   (`runtime.path` default, `diagnostics.globals`, etc.).

When the `EditorState` is dropped, send `shutdown` + `exit` and reap.

### 4.3 Document sync

When a `.lua` file is opened:

```json
textDocument/didOpen { textDocument: { uri, languageId: "lua", version: 1, text } }
```

On every buffer change (debounced 250 ms):

```json
textDocument/didChange {
  textDocument: { uri, version: N },
  contentChanges: [{ text: <full buffer> }]
}
```

On save (Ctrl+S):

```json
textDocument/didSave { textDocument: { uri, text?: <full buffer> } }
```

### 4.4 Completion flow

User triggers `editor.completion` (in egui: Ctrl+Space OR automatic on `.`):

```rust
// Pseudocode in LuaEditorPanel::show
if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Space)) {
    let req_id = lsp.request(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri_for(path) },
            "position": { "line": line, "character": col_utf16 },
            "context": { "triggerKind": 1 }
        })
    );
    // store pending; when response arrives, populate completion popup
}
```

### 4.5 Diagnostics flow

`textDocument/publishDiagnostics` is server-initiated. Each message carries
`{ uri, diagnostics: [{ range, severity, message, code?, source? }] }`.
Convert LSP positions to (line, col) byte/UTF-8 by reading the current buffer
to map UTF-16 code units → byte offset. Cache by URI.

### 4.6 Engine type stubs

`lua_stub_gen.rs` writes `.craft/engine_types.lua` at the workspace root:

```lua
-- AUTO-GENERATED by craft-editor. Do not edit.
-- schema-version: <SCHEMA_VERSION>
--- @meta

--- @class Engine
--- @field get_node fun(id: string): Node | nil
--- @field emit fun(signal: string, args: table)
--- @field call_system fun(name: string, args: table): any
local Engine = {}

--- @class Node
--- @field id string
--- @field position Vec2
--- @field [string] any  -- components
local Node = {}

--- @class Vec2
--- @field [1] number
--- @field [2] number
local Vec2 = {}

--- @class SignalBus
--- @field emit fun(name: string, args: table)
local SignalBus = {}

return {}
```

Regeneration rules:

- If `.craft/engine_types.lua` does not exist → write it (first-run case).
- If it exists and the `-- schema-version:` line matches `craft_schema::SCHEMA_VERSION` → no-op.
- If it exists and the version differs → overwrite.

Manual `Regenerate stubs` button in the panel header for force-rerun
(skips the version check).

**Stub header substitution:** the stub is generated by
`format!("-- schema-version: {}\n-- AUTO-GENERATED by craft-editor. Do not edit.\n\n{}",
         craft_schema::SCHEMA_VERSION,
         craft_schema::lua_engine_stub())`. The first line is the canonical
version stamp read on subsequent regenerations; the second line is
human-readable.

The exact set of fields/methods exposed comes from a new function in
`craft-schema`:

```rust
// crates/craft-schema/src/lib.rs (additions)
pub fn lua_engine_stub() -> String {
    // hardcoded for E2; will move to schemars-driven generation in v3
    include_str!("lua_engine_stub.lua").to_string()
}
```

The asset file `crates/craft-schema/src/lua_engine_stub.lua` is checked in to
the `craft-schema` crate (not the editor). `craft-editor` calls
`craft_schema::lua_engine_stub()` to obtain the stub body and writes it to
`<workspace_root>/.craft/engine_types.lua`. This avoids any editor→schema
recursion and keeps the asset with the schema it describes.

**Note:** `craft_schema::SCHEMA_VERSION` already exists (currently `"1.0"`).
E2 reuses it as the only schema-version constant — no new constant is added.
The stub generator compares the constant against the header comment in the
existing on-disk stub to decide whether to regenerate.

### 4.7 Hot reload on save

`LuaEditorPanel::save_buffer()`:

```rust
pub fn save_buffer(&mut self, state: &mut EditorState) -> Result<(), LuaSaveError> {
    if let Some(path) = &self.current_path {
        std::fs::write(path, &self.buffer)?;
        let class_name = derive_class_name(path); // "towers.target_priority"
        if let Some(runtime) = state.engine.lua_runtime_mut() {
            runtime.reload_class(&class_name, &self.buffer)
                .map_err(LuaSaveError::Script)?;
        }
        self.dirty = false;
        state.ui.status_message = format!("hot-reloaded Lua class: {class_name}");
    }
    Ok(())
}
```

`derive_class_name(path)` is a deterministic function:
- Strip `.lua` extension
- Replace `/` with `.`
- Lower-case nothing (Lua is case-sensitive)

`state.engine.lua_runtime_mut()` is a new accessor on `EditorEngine` (see
§7.1) that exposes the underlying `craft_lua::LuaRuntime`. Every editor
session attempts to create one in `EditorEngine::new()`; the accessor's
`None` branch covers construction failure (e.g., mlua incompatibility on
the host).

### 4.7a `craft.toml` `[lua]` parsing — done in the editor, not the kernel

`craft_kernel::Project` (`crates/craft-kernel/src/project.rs:10`) has no
`lua` field, and we deliberately do NOT extend it: keeping Lua as a
craft-lua concern (not a kernel concern) preserves ADR 0007's sync-NAPI
boundary and ADR 0015's "kernel is single-threaded + GUI-free" property.

The editor parses the `[lua]` section of `craft.toml` locally. New module:

```rust
// crates/craft-editor/src/io/project.rs (additions)

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct CraftTomlLua {
    #[serde(default)]
    pub modules_dir: Option<PathBuf>,
}

pub fn read_lua_section(root: &Path) -> CraftTomlLua {
    let manifest = root.join("craft.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else {
        return CraftTomlLua::default();
    };
    let Ok(table) = text.parse::<toml::Table>() else {
        return CraftTomlLua::default();
    };
    let Some(lua_value) = table.get("lua").cloned() else {
        return CraftTomlLua::default();
    };
    toml::from_value::<CraftTomlLua>(lua_value).unwrap_or_default()
}
```

`toml = { workspace = true }` is already a dependency of `craft-editor`
(added in E1 Task 1.2 for `io/project.rs`).

### 4.7b `load_scene_file` flow (refined)

`EditorEngine::load_scene_file(scene_json_path: &Path)` keeps its current
single-argument signature (E1 tests in `e1_engine_lifecycle.rs:16`,
`e1_panels_kittest.rs:20` still compile unchanged). The added Lua setup
runs immediately after the scene load succeeds:

1. Read `scene_json_path`'s parent directory (call it `project_root`).
2. Call `crate::io::project::read_lua_section(project_root)`.
3. If `modules_dir` is `Some(rel)` AND the resolved path exists on disk:
   - Resolve `absolute_path = project_root.join(rel)` (relative paths in
     `craft.toml` are relative to the manifest, not to CWD).
   - Call `state.engine.lua_runtime_mut().map(|rt| rt.set_modules_dir(absolute_path))`.
4. If `modules_dir` is `None` or the resolved path doesn't exist, do nothing
   (Lua runtime stays inert; `reload_class` calls still succeed because
   `set_modules_dir` is only required for `require()` from in-game scripts,
   not for direct `reload_class` calls).

No new entry point, no new signature. Existing tests keep working.

Note: `LuaRuntime::set_modules_dir` returns `()` (does not return a Result).
A bad `modules_dir` path is therefore not detected at this point; it
surfaces later when game scripts attempt `require()` and fail. This is
acceptable for E2 because the editor's primary use of Lua is direct
`reload_class`, not game-script `require()`.

## 5. State persistence

E2 adds to `EditorState`:

```rust
// crates/craft-editor/src/state.rs (additions)

pub struct EditorState {
    // ... existing E1 fields
    pub lua_editor: LuaEditorState,        // NEW
    pub standalone_behavior: BehaviorEditorState,  // NEW (current file; section §3.3)
}

pub struct InspectorState {
    // ... existing E1 fields
    pub expanded_behaviors: HashSet<(String, usize)>,   // NEW (section §3.1)
    pub behavior_edits: HashMap<(String, usize), BehaviorEditState>,  // NEW (section §3.1)
    pub last_validation_ms: u64,           // NEW (section §3.1)
}
```

`LuaEditorState` is defined in §4.1 and `BehaviorEditorState` in §3.3.

**Default impl update:** `crates/craft-editor/src/state.rs` has a manual
`impl Default for EditorState` (E1) and `impl Default for InspectorState`
(implicit, via `#[derive(Default)]`). E2's new fields must be initialized
in the `EditorState::default()` body and added to `InspectorState`'s
`Default` derive. A planner must update both.

Neither new field is persisted to disk across editor sessions in E2 (open
Lua/behavior files re-open empty). Window/dock layout persistence (E1)
covers layout restoration; "remember open files" is deferred to E3.

### 5.1 Index-keyed behavior edits caveat

`behavior_edits` and `expanded_behaviors` are keyed by
`(node_id: String, behavior_idx: usize)`. Apply mutates
`scene.def.nodes[node_id].behaviors[behavior_idx]` in place but does NOT
reorder, so indices stay valid as long as no upstream Inspector action
inserts/removes/reorders behaviors during an open edit session. If a
future E2.x action does reorder, the planner must add an explicit refresh
step. The alternative (keying on a stable behavior id) is deferred to v3
when the standalone format's `id` field becomes universal.

## 6. Testing strategy

### 6.1 Unit tests (`#[cfg(test)] mod tests`)

`crates/craft-editor/src/json_path.rs::tests`:
- `path_at_cursor_returns_dotted_path_for_simple_object`
- `path_at_cursor_returns_array_index_for_array_element`
- `path_at_cursor_handles_incomplete_buffer` (trailing comma, unclosed brace)
- `complete_at_object_key_returns_schema_properties`
- `complete_at_enum_value_returns_enum_values`
- `complete_at_typed_value_returns_default_snippet`
- `validate_flags_missing_required_property`
- `validate_flags_type_mismatch`
- `validate_ignores_unknown_property_as_warning`
- `line_col_mapping_byte_offset_to_line_col`

`crates/craft-editor/src/lua_stub_gen.rs::tests`:
- `stub_includes_schema_version_header`
- `stub_contains_engine_class`
- `stub_contains_node_class`
- `regenerate_skipped_when_version_unchanged`
- `regenerate_runs_when_version_changed`

`crates/craft-editor/src/lsp/mod.rs::tests`:
- `parse_content_length_header`
- `frame_inbound_message`

### 6.2 Integration tests

`crates/craft-editor/tests/e2_inspector_behavior.rs`:
- Load `games/tower_defense/scene.json` into an `EditorState` fixture.
- "Open behavior 0" programmatically means: insert a `BehaviorEditState`
  directly into `state.panels.inspector.behavior_edits` for the first node's
  first behavior. The test does not simulate the row-click UI affordance
  (that's covered by `e2_panels_kittest` below). It does insert into
  `expanded_behaviors` so the inspector renders the editor pane.
- Mutate buffer to invalid JSON → assert `errors` contains a parse error.
- Mutate buffer to invalid `target.type` → assert schema flags it.
- Mutate buffer to valid behavior → assert `errors` is empty.
- Apply → assert `scene.def.nodes[..].behaviors[..]` reflects the edit AND
  `state.scene.as_ref().unwrap().is_dirty()` returns `true` (proves the
  hash mismatch propagated, and that `last_saved_hash` was NOT spuriously
  updated by Apply).

`crates/craft-editor/tests/e2_lua_editor.rs`:
- Helper `lua_ls_available() -> bool` via `which::which("lua-language-server")`.
  If false, the test body returns `Ok(())` with an `eprintln!("skip: LuaLS missing")`.
- Spawn `LspClient` against a tempdir, send `initialize` → expect
  `InitializeResult` with `capabilities` via `recv_timeout(2s)`.
- Send `textDocument/didOpen` for a sample Lua file with a syntax error → expect
  `textDocument/publishDiagnostics` notification within 5 seconds.
- Save + call `LuaRuntime::reload_class(name, source)` directly (bypassing the
  Lua editor panel) → assert it returns `Ok(())`. Then call
  `engine.tick()` (via `EditorEngine::step()`) and assert no panic on the
  unknown-class path.

`crates/craft-editor/tests/e2_panels_kittest.rs`:
- Render smoke tests for `BehaviorEditorPanel` and `LuaEditorPanel` (both in
  empty and loaded states). No PNG snapshots — same approach as E1's
  `e1_panels_kittest.rs`. `Harness::new_ui(...).run().fit_contents()` only.

### 6.3 Coverage gate

`scripts/coverage.sh` (already extended in E1) — no change needed; `craft-editor`
stays in the coverage list with the 65% threshold. E2's new modules are pure
logic + subprocess wrappers, all unit-testable.

### 6.4 Test time budget

E2 additions target ≤ +10s to the E1 suite:

- `e2_json_path` unit: 1s
- `e2_lua_stub_gen` unit: 0.5s
- `e2_inspector_behavior` integration: 3s
- `e2_lua_editor` integration: 4s (with skip-if-missing) / 0.5s (skip path)
- LSP framing unit: 0.5s
- `e2_panels_kittest` render-smoke tests: 1s (covers behavior + lua panels
  in empty + loaded states; same approach as `e1_panels_kittest.rs`)

### 6.5 What is NOT unit-tested

- Live LuaLS completion UI (covered by kittest render smoke tests)
- File-watcher conflict UX (covered by E1's analogous test)
- Real `.lua` roundtrip against `craft-lua` (covered in craft-lua's own tests)

## 7. API additions

E2 requires the following additive API changes. None modify existing public
API; all are net-new.

### 7.1 `craft-editor` additions (EditorEngine gains a Lua runtime)

`EditorEngine` (already defined in E1 at `crates/craft-editor/src/engine.rs:12`)
gains a new field, an init-error slot, and accessors:

```rust
// crates/craft-editor/src/engine.rs (additions to EditorEngine)
pub struct EditorEngine {
    // ... existing E1 fields ...
    pub lua_runtime: Option<craft_lua::LuaRuntime>,  // NEW: Some if init succeeded
    pub lua_init_error: Option<String>,              // NEW: message if init failed
}

impl EditorEngine {
    pub fn new() -> Self {
        // LuaRuntime::new is fallible (LuaResult<Self>); per AGENTS.md we
        // can't use expect()/unwrap() in production code. Try construction;
        // on failure, store the error message and leave lua_runtime = None.
        match craft_lua::LuaRuntime::new(0) {
            Ok(rt) => Self {
                lua_runtime: Some(rt),
                lua_init_error: None,
                // ... existing E1 fields ...
            },
            Err(e) => Self {
                lua_runtime: None,
                lua_init_error: Some(e.to_string()),
                // ... existing E1 fields ...
            },
        }
    }

    pub fn lua_runtime_mut(&mut self) -> Option<&mut craft_lua::LuaRuntime> {
        self.lua_runtime.as_mut()
    }

    pub fn lua_runtime(&self) -> Option<&craft_lua::LuaRuntime> {
        self.lua_runtime.as_ref()
    }

    pub fn lua_runtime_error(&self) -> Option<&str> {
        self.lua_init_error.as_deref()
    }
}
```

The seed `0` is fixed for editor sessions — editor is non-replay by
definition (ADR 0015). Lua runtime construction failure (e.g., mlua version
mismatch on the host) surfaces in two places:

1. The `LuaEditorPanel` header shows the error message and disables all Lua
   functionality (no completion, no diagnostics, no reload).
2. `state.ui.status_message` is set once at construction.

`lua_runtime_mut()` returns `None` on init failure; every caller
(`LuaEditorPanel::save_buffer`, `Watcher::Changed` handler, etc.) is
required to handle the `None` branch.

This is **not** a `craft-kernel` change: the editor wraps `craft_kernel::Engine`
and adds `craft_lua::LuaRuntime` as an orthogonal sibling. The kernel stays
Lua-free.

### 7.2 Schema version constant

No new constant. E2 reuses `craft_schema::SCHEMA_VERSION` (already exported
at `crates/craft-schema/src/lib.rs:22`, currently `"1.0"`) as the single
schema-version source of truth. The Lua stub generator stamps this into the
header comment of `.craft/engine_types.lua`; the standalone behavior file
writer may stamp it into the `$schema` field of newly authored files (best
effort — not enforced by validation in E2).

## 8. File watcher integration

E1's `Watcher` already fires on external file changes. E2 adds two handlers:

- On `WatcherEvent::Changed(path)` where `path` ends with `.behavior.json`:
  - If `standalone_behavior.path == Some(path)` AND `standalone_behavior.dirty`:
    → show E1's `[Reload] [Keep mine]` prompt (already implemented).
  - Else: auto-reload (no prompt; standalone behaviors are not "in flight"
    unless actively being edited).

- On `WatcherEvent::Changed(path)` where `path` ends with `.lua` AND that file
  is currently the active Lua editor buffer AND `dirty`:
  → same `[Reload] [Keep mine]` prompt.
  - On `[Reload]`: replace buffer with file content; resend
    `textDocument/didChange` to LuaLS so its diagnostics reflect the new
    version.
  - On `[Keep mine]`: no change.

This is a small `state.file_change_pending: Option<PathBuf>` extension
(currently only scene.json triggers it in E1). E2 adds a small enum:

```rust
pub enum FileChangeKind { SceneJson, Lua, Behavior }
pub struct FileChangePending {
    pub path: PathBuf,
    pub kind: FileChangeKind,
}
```

## 9. Acceptance criteria (E2 done means...)

### Behavior editor (Inspector inline)

- [ ] Click behavior row in Inspector → row expands with `▶ edit JSON`
- [ ] JSON editor accepts multi-line input; line numbers + syntax colors visible
- [ ] Typing invalid JSON → red marker at error position + status bar shows count
- [ ] Ctrl+Space at object key position → completion popup shows schema-allowed keys
- [ ] Ctrl+Space at value position → enum values or type default snippet
- [ ] Apply button appears only when errors is empty
- [ ] Apply (button or Ctrl+Enter) mutates `scene.def.nodes[node_id].behaviors[behavior_idx]`;
      `is_dirty()` returns `true` after Apply; `last_saved_hash` is NOT updated
- [ ] Ctrl+S in the editor with un-applied dirty buffer → status bar warns
      `behavior modified — press Apply first`; does NOT save
- [ ] Ctrl+S in the editor after Apply → writes scene.json to disk
      (`save_dirty()` updates `last_saved_hash`)

### Standalone behavior editor

- [ ] File Browser double-click `.behavior.json` → `BehaviorEditorPanel` opens with file content
- [ ] Same JSON editor + schema-LSP affordances as Inspector inline
- [ ] Apply writes the file directly (no separate Ctrl+S needed)
- [ ] Closing panel without save → buffer kept as draft (warns if dirty)

### Lua editor

- [ ] File Browser double-click `.lua` → `LuaEditorPanel` opens with file content
- [ ] Syntax highlighting (keywords / strings / numbers / comments)
- [ ] If LuaLS found on `$PATH`: spawned within 2 seconds of activation,
  `initialized` handshake completes, capabilities populated
- [ ] Typing `engine.` in editor → completion popup shows LuaLS-resolved symbols
  (`get_node`, `emit`, `call_system`, …)
- [ ] Typing a syntax error → red marker within 1 second (LSP diagnostic)
- [ ] Ctrl+S → file saved + `LuaRuntime::reload_class` invoked + status bar reflects
- [ ] If LuaLS NOT found: banner `LuaLS not found at <path>. Local syntax
  highlighting only.` + saving still works + reload still works
- [ ] External `.lua` file change while editor has buffer → `[Reload] [Keep mine]`
  prompt, behaving as in §8

### Quality gates

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `craft-editor` coverage ≥65%
- [ ] Total E2 test suite additions ≤ +10s vs E1
- [ ] No new `unwrap()` / `expect()` in production code
- [ ] No new `clippy::pedantic` warnings
- [ ] `craft-kernel` and `craft-lua` compile without egui
- [ ] E1 acceptance criteria still pass (no regressions)

### Documentation

- [ ] `crates/craft-editor/README.md` updated with editor panels section
- [ ] CHANGELOG entry for E2
- [ ] ROADMAP.md updated: E2 marked done

### Done checklist (AGENTS.md "Before Claiming Done")

- [ ] All acceptance criteria above pass
- [ ] `cargo test --workspace` green
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] No new `unwrap()` / `expect()` in production code
- [ ] No commented-out code
- [ ] No unrelated changes in diff
- [ ] Spec reviewed and committed before implementation began

## 10. References

- ADR 0017 — Editor Architecture
- ADR 0018 — Editor State Management & Panel Design (Behavior Editor section, Appendix A: Lua Script Editor)
- ROADMAP.md §v2: Editor (E1–E4)
- AGENTS.md §Rust conventions, §Architecture Constraints
- E1 spec `docs/superpowers/specs/2026-07-16-editor-e1-design.md` (for panel/dock conventions)
- LuaLS docs: <https://luals.github.io/>