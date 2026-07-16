# Craft Editor — E1 Design

**Date**: 2026-07-16
**Status**: Approved (pending user review)
**Author**: brainstorming session
**Scope**: v2 Editor, milestone E1 only

## Context

Craft v0.1.0 ships the engine core (M1–M10) and Lua scripting (L1–L3). The next milestone is the v2 Editor. ADRs 0017–0019 already specify the high-level architecture (egui/eframe, embedded engine, 7 panels, file-first writes, Agent Copilot). This spec narrows the **first** editor milestone (E1) to a shippable, tested, CI-gated deliverable.

E1 delivers: editor shell + 7 panels (4 functional, 3 stubs) + persistence + tests + quality gates. Subsequent editor milestones (E2 behavior/Lua editing, E3 Agent Copilot, E4 UX polish) are separate specs.

## Out of scope

- Undo/redo history (E2)
- JSON behavior editor with schema auto-complete (E2)
- Lua editor with LuaLS / syntax highlighting (E2)
- Agent Copilot with LLM backend (E3)
- Drag-drop, visual state-machine editor, polished context menus (E4)
- Live engine values in Inspector — E1 keeps Inspector ↔ SceneDef only

## Decisions (E1-specific)

These supplement (do not replace) ADRs 0017–0019:

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Scope | E1 only, not E1–E4 | Smaller spec, faster ship, isolated risk |
| Panel scope | 4 functional + 3 stubs | Per ROADMAP E1 |
| Inspector data source | Always reads/writes SceneDef | Simpler; engine state via Terminal Preview |
| Test strategy | Unit + integration + egui_kittest snapshots | Covers logic + rendering path |
| Persistence | Dock layout + recent projects | Listed in ROADMAP, no undo |
| Platform | macOS primary; Linux/Windows supported via eframe features | Matches dev environment |
| Renderer | Custom `EditorRenderer` sibling of `craft-terminal::AnsiRenderer` | Avoids ANSI round-trip |

---

## 1. Crate structure & dependencies

```
crates/craft-editor/
├── Cargo.toml
├── src/
│   ├── main.rs                   # eframe entry; parse CLI args; build EditorApp
│   ├── app.rs                    # EditorApp: eframe::App impl — per-frame orchestration
│   ├── state.rs                  # EditorState (single source of truth across frames)
│   ├── engine.rs                 # EditorEngine — wraps craft_kernel::Engine
│   ├── io/
│   │   ├── mod.rs                # craft.toml / scene.json / recent projects IO
│   │   ├── scene_def.rs          # load/save SceneDef
│   │   ├── project.rs            # parse craft.toml, resolve resource paths
│   │   └── recent.rs             # ~/.config/craft-editor/recent.json (persistent)
│   ├── watcher.rs                # notify-based file watcher with debounce + dedupe
│   ├── panels/
│   │   ├── mod.rs                # Panel trait + PanelAction enum
│   │   ├── scene_tree.rs         # Functional (E1)
│   │   ├── inspector.rs          # Functional (E1)
│   │   ├── file_browser.rs       # Functional (E1)
│   │   ├── terminal_preview.rs   # Functional (E1)
│   │   ├── behavior_editor.rs    # Stub (E2)
│   │   ├── lua_editor.rs         # Stub (E2)
│   │   └── agent_panel.rs        # Stub (E3)
│   ├── theme.rs                  # Dark theme + monospace font setup
│   └── persist.rs                # Dock layout save/load
└── tests/
    ├── e1_state_integration.rs   # EditorState transitions, scene IO
    ├── e1_io_integration.rs      # Load/save round-trip
    ├── e1_watcher_integration.rs # notify debounce + reload semantics
    ├── e1_engine_lifecycle.rs    # load scene, run, stop, assert grid
    ├── e1_persistence_integration.rs # dock + recent round-trip
    └── e1_panels_kittest.rs      # egui_kittest snapshot per panel
```

### Cargo.toml

```toml
[package]
name = "craft-editor"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
craft-kernel = { path = "../craft-kernel" }
craft-schema = { path = "../craft-schema" }
egui = "0.31"
eframe = { version = "0.31", default-features = false, features = ["default_fonts", "glow", "wayland", "x11"] }
egui_dock = "0.14"
rfd = "0.15"
notify = "7"
serde = { workspace = true }
serde_json = { workspace = true }
directories = "5"
tracing = "0.1"
thiserror = { workspace = true }

[dev-dependencies]
egui_kittest = "0.31"
tempfile = "3"
```

Workspace `Cargo.toml` gains `crates/craft-editor` under `members`.

### Workspace rule (no exceptions)

`craft-kernel` and `craft-lua` MUST NOT depend on `craft-editor`. Editor is a leaf consumer; engine stays GUI-free for headless CI.

### Platform

- Primary: macOS (dev environment)
- Supported via `eframe` features: Linux (Wayland + X11), Windows
- CI smoke: headless test suite runs on Linux only; manual macOS verification for visual panels

---

## 2. EditorState

### Top-level struct

```rust
pub struct EditorState {
    pub project: Option<ProjectState>,
    pub scene: Option<SceneState>,
    pub engine: EditorEngine,
    pub panels: PanelsState,
    pub dock: DockState<EditorTab>,
    pub ui: UiState,
    pub lsp: LspManager,                    // empty in E1 (populated in E2)
    pub pending_actions: Vec<PanelAction>,
    pub errors: Vec<EditorError>,
}

pub struct ProjectState {
    pub root: PathBuf,
    pub craft_toml: CraftToml,
}

pub struct SceneState {
    pub path: PathBuf,
    pub def: SceneDef,
    pub file_watcher_epoch: u64,
    pub last_saved_hash: u64,
    pub is_dirty: bool,
}

pub struct UiState {
    pub status_message: String,
    pub file_change_pending: Option<PathBuf>,
    pub quit_requested: bool,
}
```

### Panel sub-states

```rust
pub struct PanelsState {
    pub scene_tree: SceneTreeState,
    pub inspector: InspectorState,
    pub file_browser: FileBrowserState,
    pub terminal_preview: TerminalPreviewState,
    pub behavior_editor: BehaviorEditorStub,
    pub lua_editor: LuaEditorStub,
    pub agent_panel: AgentPanelStub,
}

pub struct SceneTreeState {
    pub selected_node: Option<NodeId>,
    pub expanded_nodes: HashSet<NodeId>,
    pub filter_text: String,
}

pub struct InspectorState {
    pub search_text: String,
    pub add_component_menu_open: bool,
}

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub filter: String,
}

pub struct TerminalPreviewState {
    pub is_running: bool,
    pub last_tick: u64,
    pub grid: TerminalGrid,
}
```

### Dirty tracking

`SceneState.is_dirty = hash(def) != last_saved_hash`. Inspector edits mark dirty. `Ctrl+S` writes file + reloads hash + resets `is_dirty`.

### File change handling

When the notify watcher detects an external change to `scene.json`:
1. Update `file_watcher_epoch` and store path in `ui.file_change_pending`
2. Status bar shows "scene.json changed externally — [Reload] [Keep mine]"
3. User chooses → re-parse or discard in-memory edits

### Recent projects

Owned outside `EditorState` by `io/recent.rs`. Saved on project close; loaded at startup.

### Error collection

`EditorState.errors: Vec<EditorError>` aggregates structured errors from panels, engine, and IO. Errors shown in a collapsible bottom drawer.

---

## 3. Panel trait & 7 panels

### Panel trait

```rust
pub trait Panel {
    fn id(&self) -> &'static str;
    fn title(&self, state: &EditorState) -> String;
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction>;
}

pub enum PanelAction {
    OpenScene(PathBuf),
    SaveScene,
    RunScene,
    StopScene,
    StepTick,
    ReloadScene,
    SwitchPanel(EditorTabKind),
    SetStatus(String),
    ReportError(EditorError),
    RequestQuit,
}
```

### Functional panels (E1)

**Scene Tree** — renders `state.scene.def.root` recursively. Click selects, right-click opens context menu (Add Child, Delete, Duplicate, Rename), drag reorders/reparents, filter box at top, type indicator chip per node.

**Inspector** — selected node's components as form fields. DragValue for int/float, TextEdit for string, Checkbox for bool, custom widgets for Vec2/Color/Enum/NodeRef. `+ Add Component` opens menu from `craft-schema`. Transient components show `(transient)` chip. Edits mark dirty.

**File Browser** — lazy directory tree of `state.project.root`. Filter box. Double-click `.json` → open as scene (if `kind: "scene"`) or behavior. Double-click `.lua` → open Lua editor stub. Right-click → New File/Folder/Delete via `rfd::FileDialog`.

**Terminal Preview** — Run/Pause/Stop/Reload/Step toolbar. Engine output captured via `EditorRenderer` (Section 4) and rendered as egui `colored_label` grid. Ticks at 60Hz when running, frozen last frame when stopped.

### Stubs (E1)

Behavior Editor / Lua Editor / Agent Copilot each render centered placeholder text:

```rust
"Behavior Editor — coming in E2"
"Lua Editor — coming in E2"
"Agent Copilot — coming in E3"
```

Stubs exist as panels so:
- The dock layout includes all 7 tabs from day one (no layout shift when E2/E3 land)
- E1 acceptance can assert tabs are present and labeled

### Toolbar & menu

Top-level menu bar (above dock): File / Edit / Scene / View / Help.

### Keyboard shortcuts (E1)

| Key | Action |
|-----|--------|
| Ctrl+S | Save scene |
| Ctrl+O | Open project |
| Ctrl+W | Close current tab |
| F5 | Run |
| F8 | Stop |
| F10 | Step one tick |
| Ctrl+1 | Focus Scene Tree |
| Ctrl+2 | Focus Inspector |
| Ctrl+3 | Focus Terminal Preview |
| Ctrl+4 | Focus File Browser |

Undo/redo shortcuts deferred to E2.

---

## 4. EditorEngine & hot reload integration

### EditorEngine wrapper

```rust
pub struct EditorEngine {
    engine: craft_kernel::Engine,
    renderer: EditorRenderer,
    scene_path: Option<PathBuf>,
    is_running: bool,
    is_paused: bool,
    tick_timer: Instant,
    tick_rate_hz: u32,                              // default 60
}

impl EditorEngine {
    pub fn new() -> Self;
    pub fn load_scene(&mut self, path: &Path) -> EngineResult<()>;
    pub fn run(&mut self) -> EngineResult<()>;
    pub fn stop(&mut self);
    pub fn pause(&mut self);
    pub fn resume(&mut self);
    pub fn step(&mut self) -> EngineResult<()>;
    pub fn reload(&mut self) -> EngineResult<SceneDiff>;
    pub fn tick_if_due(&mut self) -> bool;
    pub fn grid(&self) -> &TerminalGrid;
}
```

### EditorRenderer (implements `craft_kernel::Render`)

```rust
pub struct EditorRenderer {
    grid: TerminalGrid,
    cols: u16,
    rows: u16,
}

pub struct Cell {
    pub ch: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

pub struct TerminalGrid {
    pub cells: Vec<Cell>,
    pub cols: u16,
    pub rows: u16,
}

impl craft_kernel::Render for EditorRenderer {
    fn begin_frame(&mut self, cols: u16, rows: u16) { ... }
    fn put_cell(&mut self, col: u16, row: u16, ch: char, fg: [u8; 3], bg: [u8; 3]) { ... }
    fn end_frame(&mut self) { ... }
    fn present(&mut self) { }                        // no-op
}
```

Mirrors `craft-terminal` ANSI-emit logic but writes to in-memory grid. Avoids ANSI round-trip.

**Code-sharing**: copy emit logic into `craft-editor` for E1 (<100 lines). De-duplication refactor (extract `craft_terminal_core`) deferred.

### Tick loop

`EditorApp::update()` calls `engine.tick_if_due()` each frame:
- If `is_running && !is_paused`, and ≥ `1_000_000 / tick_rate_hz` µs have passed, call `engine.tick()`
- After tick, request `ctx.request_repaint()`
- Step (F10) calls tick once regardless of timer

### Hot reload flow

```
Inspector edit → SceneDef mutated → is_dirty=true → Ctrl+S → write file
                                                              │
External edit → notify watcher fires → status bar prompt      │
                                            │                 │
              user clicks [Reload] ──────────┴──► re-parse file into SceneDef
                                                                │
                                                                ▼
                                                        engine.reload() → SceneDiff
                                                                │
                                                                ▼
                                                        status bar shows summary
```

`engine.reload()` is `craft-kernel`'s existing entry point (ADR 0009). Result treated as black box.

### File watcher

```rust
pub struct Watcher {
    _inner: notify::RecommendedWatcher,
    receiver: crossbeam_channel::Receiver<WatcherEvent>,
    debounce: Duration,                              // 100ms
}

pub enum WatcherEvent {
    Changed(PathBuf),
    Removed(PathBuf),
}
```

Recursive watch on `state.project.root`. 100ms debounce. Ignores writes performed by editor itself.

### Scene loading

`EditorEngine::load_scene(path)`:
1. Read file content
2. Parse via `craft-kernel::Scene::from_json`
3. Call `engine.start(path)`
4. Set `scene_path`, `is_running = true`

---

## 5. Persistence

| Data | Where | When written | When read |
|------|-------|--------------|-----------|
| Dock layout | `egui_dock::DockState::save` → `~/.config/craft-editor/dock.bin` | On window close | On editor startup |
| Recent projects | `~/.config/craft-editor/recent.json` | On project close | On editor startup |
| Window size/pos | eframe native | eframe | eframe |

### Recent projects schema

```json
{
  "version": 1,
  "entries": [
    { "root": "/Users/me/projects/tower_defense", "last_opened": "2026-07-16T10:00:00Z" }
  ]
}
```

Max 10 entries, sorted by `last_opened` desc, deduplicated by `root`.

### What's NOT persisted (E1)

- Open scene path (re-opens empty)
- SceneDef in-memory state
- Open tabs (dock state restores tab *positions* but not which tabs are open — for E1 the dock starts with the 4 functional panels in default positions)
- Undo history (E2)

### Path resolution

`directories::ProjectDirs::from("ai", "craft", "editor")`:
- macOS: `~/Library/Application Support/ai.craft.editor/`
- Linux: `~/.config/craft-editor/`
- Windows: `%APPDATA%\craft\editor\`

### Failure mode

Persistence failures are non-fatal:
- Dock layout read failure → default layout
- Recent projects read failure → empty list
- Write failure → status bar warning, editor continues

---

## 6. Testing strategy

### Three layers (per ADR 0010 + egui-specific additions)

**Unit tests** (`#[cfg(test)] mod tests`) — pure functions in `state.rs`, `io/*.rs`, `watcher.rs`, `engine.rs`.

**Integration tests** (`tests/e1_*.rs`):
- `e1_state_integration.rs` — EditorState transitions: open project → load scene → edit via Inspector → save → reload
- `e1_io_integration.rs` — Load tower_defense scene.json, mutate SceneDef, save, reload, assert content matches
- `e1_watcher_integration.rs` — Spawn tempdir, write file, wait for watcher event with timeout, assert debounce coalesces multiple writes
- `e1_persistence_integration.rs` — Write dock.bin + recent.json, simulate restart by reloading both, assert state restored
- `e1_engine_lifecycle.rs` — Load tower_defense scene, run 10 ticks, stop, assert grid has non-empty cells

**egui_kittest snapshots** (`e1_panels_kittest.rs`):

```rust
#[test]
fn scene_tree_empty_state() {
    let mut harness = Harness::new_ui(|ui| {
        let mut state = EditorState::empty();
        SceneTreePanel.show(ui, &mut state)
    });
    harness.run();
    harness.fit_contents();
    harness.snapshot("scene_tree_empty");
}
```

Snapshots stored under `tests/snapshots/`. CI verifies by default; developer-overwrite mode explicit.

### Coverage gate

`scripts/coverage.sh` adds `craft-editor` to the coverage run. Target: ≥70% line coverage on production code.

### Headless test environment

`egui_kittest` runs without a display server. Works on Linux CI and macOS dev machines without `Xvfb`. Tests use kittest's headless harness — no actual window opens in tests.

### Test time budget

E1 test suite target: <30s total:
- Unit: 2s
- IO integration: 3s
- Watcher: 4s
- Engine lifecycle: 5s
- egui_kittest snapshots: 15s
- Persistence: 1s

### What is NOT unit-tested

- Pixel-perfect rendering (snapshots catch UI regressions)
- Mouse drag-drop (deferred to E4)
- GPU texture upload (E1 uses egui labels, not textures)

---

## 7. Acceptance criteria (E1 done means...)

### Functional

- [ ] `cargo run -p craft-editor -- path/to/craft.toml` opens the project
- [ ] `cargo run -p craft-editor` opens recent-projects dialog if no arg
- [ ] Scene Tree displays `games/tower_defense/scene.json` with all nodes visible (≥10 nodes)
- [ ] Click node → Inspector populates with that node's components
- [ ] Edit component value → file marked dirty
- [ ] Ctrl+S writes dirty file and clears `is_dirty`
- [ ] File Browser shows project tree under `craft.toml`'s directory
- [ ] Double-click `.lua` → Lua Editor stub opens
- [ ] Double-click `scene.json` → scene loads
- [ ] Terminal Preview Run (F5) starts engine → grid renders ANSI → ticks at 60Hz
- [ ] F8 stops engine → grid freezes
- [ ] F10 advances one tick when stopped
- [ ] External `scene.json` edit → status bar shows prompt with [Reload] [Keep mine]
- [ ] Click [Reload] → SceneDef replaced with file content, dirty edits discarded

### Layout / dock

- [ ] All 7 panel types present in dock (4 functional + 3 stubs)
- [ ] User can drag tabs to rearrange dock
- [ ] Close and reopen editor → dock layout preserved
- [ ] Recent projects shows last 5 projects, sorted by recency

### Stubs

- [ ] Behavior Editor renders "Behavior Editor — coming in E2"
- [ ] Lua Editor renders "Lua Editor — coming in E2"
- [ ] Agent Copilot renders "Agent Copilot — coming in E3"

### Engine integration

- [ ] Editor loads `games/tower_defense/scene.json` via `craft-kernel::Scene::from_json`
- [ ] `engine.tick()` runs; tower_defense renders the same first frame as standalone `craft-terminal`
- [ ] `engine.reload()` after edit returns `SceneDiff`; status bar shows summary

### Quality gates

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] `craft-editor` coverage ≥70%
- [ ] Total E1 test suite <30s
- [ ] No `unwrap()` / `expect()` in production code
- [ ] No new `clippy::pedantic` warnings
- [ ] `craft-kernel` compiles without egui

### Documentation

- [ ] `crates/craft-editor/README.md` with quickstart
- [ ] CHANGELOG entry for E1
- [ ] ROADMAP.md updated: E1 marked done

### Done checklist (AGENTS.md "Before Claiming Done")

- [ ] All acceptance criteria above pass
- [ ] `cargo test --workspace` green
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] No new `unwrap()` / `expect()` in production code
- [ ] No commented-out code
- [ ] No unrelated changes in diff
- [ ] Spec reviewed and committed before implementation began

---

## References

- ADR 0017 — Editor Architecture
- ADR 0018 — Editor State Management & Panel Design
- ADR 0019 — Agent Copilot Panel
- ROADMAP.md §v2: Editor (E1–E4)
- AGENTS.md §Rust conventions, §Architecture Constraints