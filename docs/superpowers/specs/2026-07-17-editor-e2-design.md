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
├── lua_stub_gen.rs          # NEW — craft-schema → LuaCATS-annotated Lua stub
├── lsp/
│   └── mod.rs               # NEW — generic LSP framing + types
├── panels/
│   ├── inspector.rs         # MOD — inline behavior JSON sub-editor
│   ├── lua_editor.rs        # MOD — replaces stub
│   └── behavior_editor.rs   # MOD — replaces stub (loads standalone .behavior.json)
└── state.rs                 # MOD — LuaEditorState + BehaviorEditState + inspector.behavior_edits

crates/craft-editor/tests/
├── e2_json_path.rs          # NEW — unit tests for JsonPathLsp
├── e2_lua_stub_gen.rs       # NEW — unit tests for stub generation
├── e2_inspector_behavior.rs # NEW — integration: schema flags invalid behavior edits
└── e2_lua_editor.rs         # NEW — integration: LuaLS subprocess lifecycle + skip-if-missing
```

### `Cargo.toml` dependencies

Add to `[dependencies]` in `crates/craft-editor/Cargo.toml`:

```toml
which = "7"            # locate lua-language-server
uuid = { workspace = true, features = ["v4"] }  # LSP request IDs
```

`which = "7"` is workspace-addable. `uuid` already in workspace at the right version.

Add to `[dev-dependencies]`:

```toml
assert_cmd = "2"       # integration: run lua-language-server if available
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
2. Mark `dirty = true`. Emit `PanelAction::SetStatus("behavior modified")`.
3. Schedule debounced validation (250 ms via `EditorState.last_validation_ms`
   + `InspectorPanel` checking elapsed). On fire:
   - Call `json_path.validate(&buffer)` → update `errors`.
   - Try `serde_json::from_str::<Behavior>` → update `parsed`. On success the
     in-memory `SceneDef` is NOT mutated here (that's `Apply` only, see §3.3).

**Ctrl+Space** opens `CompletionPopup` at cursor. Arrow keys + Enter inserts.

**Syntax highlighting:** `TextEdit::layouter` closure feeds each line through
`highlight_json_line(text, errors) -> Vec<egui::text::LayoutJob>`. Errors
emerge as red underlines beneath the offending span.

**Acceptance at this level:** an invalid edit shows a red marker; a valid edit
shows `valid · {kind} = {name}` in the footer.

### 3.2 Apply model (separate from edit)

Two distinct operations:

- **Edit (always on):** updates `buffer` + `errors`. Mutates NO scene state.
- **Apply (Ctrl+Enter or `Apply` button):**
  1. If `parsed.is_some()` AND `errors` is empty (or only warnings):
     - Mutate `scene.def.nodes[node_id].behaviors[behavior_idx]` in place.
     - Update `SceneState.last_saved_hash` (forces dirty flag flip).
  2. If `parsed.is_none()` or errors contains an Error:
     - Emit `PanelAction::SetStatus("cannot apply: {N} errors")`.

The `Apply` button only appears when `errors` are empty (disabled otherwise).
This keeps "type fast" safe — invalid intermediate states never reach the
scene def.

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
    stdout_rx: mpsc::Receiver<LspMessage>,
    pending: HashMap<i64, oneshot::Sender<LspResponse>>,
    next_id: i64,
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

Regenerated only when the embedded `craft_schema::SCHEMA_VERSION` differs from
the comment header in the existing file. Manual `Regenerate stubs` button in
the panel header for force-rerun.

The exact set of fields/methods exposed comes from a new function in
`craft-schema`:

```rust
// crates/craft-schema/src/lib.rs (additions)
pub fn lua_engine_stub() -> String {
    // hardcoded for E2; will move to schemars-driven generation in v3
    include_str!("lua_engine_stub.lua").to_string()
}

pub const SCHEMA_VERSION: &str = "0.2.0";
```

The body of `lua_engine_stub.lua` is checked in (not auto-generated in E2 to
avoid recursion: craft-schema would need to depend on schemars-driven code
generation that crosses crates, which we defer).

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

`state.engine.lua_runtime_mut()` is a new accessor on `EditorEngine` that
exposes the underlying `craft_lua::LuaRuntime` if the kernel has been wired
with one. **This requires `craft-kernel` to grow a `lua_runtime: Option<LuaRuntime>`
field** — see §7.

## 5. State persistence

E2 adds:

```rust
// crates/craft-editor/src/state.rs

pub struct EditorState {
    // ... existing E1 fields
    pub lua_editor: LuaEditorState,        // NEW
    pub standalone_behavior: BehaviorEditorState,  // NEW (current file)
}
```

Neither is persisted to disk across editor sessions in E2 (open Lua/behavior
files re-open empty). Window/dock layout persistence (E1) covers layout
restoration; "remember open files" is deferred to E3.

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
- Load `games/tower_defense/scene.json` into an `EditorState` fixture
- Open behavior 0 in the inline editor
- Mutate buffer to invalid JSON → assert `errors` contains a parse error
- Mutate buffer to invalid `target.type` → assert schema flags it
- Mutate buffer to valid behavior → assert `errors` is empty
- Apply → assert `scene.def.nodes[..].behaviors[..]` reflects the edit

`crates/craft-editor/tests/e2_lua_editor.rs`:
- Helper `lua_ls_available() -> bool` via `which::which("lua-language-server")`.
  If false, the test body returns `Ok(())` with an `eprintln!("skip: LuaLS missing")`.
- Spawn `LspClient` against a tempdir, send `initialize` → expect `InitializeResult` with `capabilities`.
- Send `textDocument/didOpen` for a sample Lua file with a syntax error → expect
  `textDocument/publishDiagnostics` notification within 5 seconds.
- Save + call `reload_class` → assert subsequent `tick` does not crash on the
  class (covered indirectly by `editor.engine.engine.tick()` not erroring).

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

### 6.5 What is NOT unit-tested

- Live LuaLS completion UI (covered by kittest render smoke tests)
- File-watcher conflict UX (covered by E1's analogous test)
- Real `.lua` roundtrip against `craft-lua` (covered in craft-lua's own tests)

## 7. `craft-kernel` API additions

E2 requires the following additions to `craft-kernel`. None change existing
public API; all are additive.

### 7.1 `EditorEngine::lua_runtime_mut()`

Already added via `EditorEngine` wrapper in E1; needs accessor:

```rust
// crates/craft-editor/src/engine.rs (additions to EditorEngine)
impl EditorEngine {
    pub fn lua_runtime_mut(&mut self) -> Option<&mut craft_lua::LuaRuntime> {
        // requires EditorEngine to hold an `Option<craft_lua::LuaRuntime>`
    }
}
```

To support this, `EditorEngine` needs a new field `lua_runtime: Option<craft_lua::LuaRuntime>`.
Construction in `EditorEngine::new()` initializes it to `None`. The
`EditorEngine::load_scene_file()` path **does not** automatically initialize
the Lua runtime — wiring Lua into a project happens explicitly via a new
panel action `PanelAction::SetLuaRuntime` (set by File Browser → "Use Lua"
on a `craft.toml` whose `[lua] modules_dir = "scripts"` exists). This keeps
the kernel/engine agnostic of Lua until the project opts in.

### 7.2 `craft-kernel::Scene::schema_version() -> &'static str`

Returns a string constant the editor can stamp into the
`$schema` field of newly authored behavior files. Implementation:

```rust
pub const SCENE_SCHEMA_VERSION: &str = "0.2.0";

pub fn schema_version(&self) -> &'static str { SCENE_SCHEMA_VERSION }
```

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
- [ ] Apply mutates `scene.def.nodes[node_id].behaviors[behavior_idx]`; mark `dirty`
- [ ] Ctrl+S in the editor → Apply + save to disk + status bar reflects

### Standalone behavior editor

- [ ] File Browser double-click `.behavior.json` → `BehaviorEditorPanel` opens with file content
- [ ] Same JSON editor + schema-LSP affordances as Inspector inline
- [ ] Apply writes the file directly
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