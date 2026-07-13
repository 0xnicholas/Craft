# ADR 0018: Editor State Management & Panel Design

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: ADR 0017 — Editor architecture. This ADR covers the internal state model and panel specifics.

## Context

egui is immediate-mode: every frame, the UI is rebuilt from scratch by calling widget functions. There is no persistent widget tree, no virtual DOM, no retained component state. This means the editor must manage **all** persistent state in a separate struct (`EditorState`) and pass it to egui each frame.

Godot's editor uses a retained GUI (the engine's own Control nodes hold their own state). Craft's egui path requires explicit state management.

## Decision

**Central `EditorState` struct with per-panel state modules. Every panel reads from and writes to `EditorState` (or its sub-state) each frame.**

### State Architecture

```rust
// crates/craft-editor/src/state.rs

pub struct EditorState {
    // Project
    pub project_path: Option<PathBuf>,
    pub scene_path: Option<PathBuf>,
    pub scene_def: Option<SceneDef>,       // parsed scene.json

    // Engine (embedded)
    pub engine: EditorEngine,

    // Panel states
    pub scene_tree: SceneTreeState,
    pub inspector: InspectorState,
    pub behavior_editor: BehaviorEditorState,
    pub lua_editor: LuaEditorState,
    pub file_browser: FileBrowserState,
    pub agent_panel: AgentPanelState,

    // UI
    pub dock_state: DockState<EditorTab>,
    pub status_message: String,
    pub error_panel: Vec<EditorError>,

    // LSP
    pub lsp_clients: LspManager,
}

pub enum EditorTab {
    SceneTree,
    Inspector,
    BehaviorEditor { path: PathBuf },
    LuaEditor { path: PathBuf },
    TerminalPreview,
    FileBrowser,
    AgentPanel,
}
```

### Panel Trait

All panels implement a common trait for consistent lifecycle:

```rust
// crates/craft-editor/src/panels/mod.rs

pub trait Panel {
    /// Unique identifier for docking persistence
    fn id(&self) -> &'static str;

    /// Display title in the tab bar
    fn title(&self, state: &EditorState) -> String;

    /// Draw the panel contents. Returns list of actions to apply after drawing.
    fn show(&mut self, ctx: &egui::Context, state: &mut EditorState) -> Vec<PanelAction>;
}

pub enum PanelAction {
    OpenFile(PathBuf),
    SaveFile(PathBuf),
    RunScene(PathBuf),
    StopScene,
    ReloadScene,
    SetStatus(String),
    ReportError(EditorError),
}
```

### Panel: Scene Tree

Displays the node hierarchy as an expandable tree. Allows:
- Click to select → populates Inspector
- Right-click → context menu (Add Child Node, Delete, Duplicate)
- Drag to reorder/reparent
- Inline type icons (Enemy = red, Tower = blue, etc.)

```rust
// crates/craft-editor/src/panels/scene_tree.rs

pub struct SceneTreeState {
    pub selected_node: Option<NodeId>,
    pub expanded_nodes: HashSet<NodeId>,
    pub filter_text: String,           // type-to-filter
    pub context_menu: Option<(NodeId, egui::Pos2)>,
}

impl Panel for SceneTreePanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let scene = &state.scene_def;

        // Filter box
        ui.text_edit_singleline(&mut self.filter_text);

        // Recursive tree
        if let Some(root) = &scene.root {
            self.show_node(ui, root, state);
        }

        // Context menu on right-click
        if let Some((node_id, pos)) = self.context_menu.take() {
            egui::Area::new("node_context_menu")
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    if ui.button("Add Child Node").clicked() { /* ... */ }
                    if ui.button("Delete Node").clicked() { /* ... */ }
                    if ui.button("Duplicate").clicked() { /* ... */ }
                });
        }
    }
}
```

### Panel: Inspector

Shows the selected node's components as editable forms:

```rust
// crates/craft-editor/src/panels/inspector.rs

pub struct InspectorState {
    pub editing_key: Option<String>,   // which component is being edited inline
    pub search_text: String,
}

impl Panel for InspectorPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let node_id = state.scene_tree.selected_node?;
        let node = state.scene_def.get_node(node_id)?;

        // Node header
        ui.heading(format!("{} ({})", node.type_name, node.id));
        ui.label(format!("lua_class: {}", node.lua_class.as_deref().unwrap_or("none")));

        ui.separator();

        // Components as form fields — one row per component
        for (key, comp) in &node.components {
            ui.horizontal(|ui| {
                ui.label(key);
                match &comp.value {
                    ComponentValue::Int(v) => {
                        let mut val = *v;
                        ui.add(egui::DragValue::new(&mut val));
                    }
                    ComponentValue::Float(v) => {
                        let mut val = *v;
                        ui.add(egui::DragValue::new(&mut val).speed(0.1));
                    }
                    ComponentValue::Vec2(v) => {
                        let mut x = v[0]; let mut y = v[1];
                        ui.add(egui::DragValue::new(&mut x));
                        ui.add(egui::DragValue::new(&mut y));
                    }
                    ComponentValue::String(v) => {
                        let mut val = v.clone();
                        ui.text_edit_singleline(&mut val);
                    }
                    ComponentValue::Bool(v) => {
                        let mut val = *v;
                        ui.checkbox(&mut val, "");
                    }
                    _ => { ui.label(format!("{:?}", comp.value)); }
                }

                // Transient badge
                if comp.kind == ComponentKind::Transient { /* ... */ }
            });
        }

        // "Add Component" button
        if ui.button("+ Add Component").clicked() { /* ... */ }
    }
}
```

### Panel: Behavior Editor (JSON + LSP)

A JSON text editor with schema-powered auto-complete and inline validation. This is the primary behavior authoring surface for human users who prefer not to write raw JSON.

```rust
// crates/craft-editor/src/panels/behavior_editor.rs

pub struct BehaviorEditorState {
    pub open_files: HashMap<PathBuf, BehaviorEditorFile>,
    pub active_file: Option<PathBuf>,
}

pub struct BehaviorEditorFile {
    pub content: String,                  // raw JSON text
    pub parsed: Option<BehaviorDef>,      // parsed into struct (for validation)
    pub errors: Vec<SchemaError>,         // inline validation errors
    pub cursor_pos: TextCursor,
    pub is_dirty: bool,
}

impl Panel for BehaviorEditorPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        let file = self.active_file_mut()?;

        // JSON text editor with:
        // 1. Line numbers
        // 2. Syntax highlighting (key/string/number/boolean coloring via regex)
        // 3. Schema-driven auto-complete (Ctrl+Space):
        //    - At object key position → show allowed keys from schema
        //    - At value position → show allowed types/enums
        // 4. Red squiggly underlines for validation errors
        // 5. Status bar: "valid JSON" / "3 errors"

        let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
            // Custom layouter for JSON syntax highlighting
            highlight_json(ui, string, &file.errors)
        };

        ui.add(
            egui::TextEdit::multiline(&mut file.content)
                .code_editor()           // monospace font
                .desired_rows(30)
                .layouter(&mut layouter)
        );

        // Auto-complete popup (positioned at cursor)
        if ui.input(|i| i.key_pressed(egui::Key::Space) && i.modifiers.ctrl) {
            show_autocomplete_popup(ui, file, state);
        }

        // Ctrl+S → save
        if ui.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            return vec![PanelAction::SaveFile(file.path.clone())];
        }
    }
}
```

**JSON Schema → Auto-Complete**:

```rust
// crates/craft-editor/src/lsp/schema_lsp.rs

pub struct SchemaLsp {
    schema: serde_json::Value,  // from craft-schema
}

impl SchemaLsp {
    /// Given cursor position in JSON, return completion suggestions
    pub fn complete(&self, json: &str, cursor: usize) -> Vec<Completion> {
        // 1. Parse JSON path at cursor (e.g., "$.behaviors[0].states.idle.on")
        let path = json_path_at_cursor(json, cursor);

        // 2. Look up schema at that path
        let schema_at_path = self.resolve_schema(&path);

        // 3. Return valid keys/values
        match schema_at_path {
            SchemaNode::Object { properties, .. } => {
                properties.keys().map(|k| Completion { label: k.clone(), kind: "property" }).collect()
            }
            SchemaNode::Enum { values } => {
                values.iter().map(|v| Completion { label: v.clone(), kind: "value" }).collect()
            }
            _ => vec![]
        }
    }
}
```

### Panel: Terminal Preview

Renders the embedded engine's ANSI output as a character grid in egui:

```rust
// crates/craft-editor/src/panels/terminal_preview.rs

pub struct TerminalPreviewState {
    pub grid: Vec<Vec<Cell>>,        // rows × cols
    pub viewport: (usize, usize),    // cols, rows
    pub is_running: bool,
}

struct Cell {
    pub ch: char,
    pub fg: egui::Color32,
    pub bg: egui::Color32,
}

impl Panel for TerminalPreviewPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        // Render the character grid as egui labels with colored backgrounds
        egui::Grid::new("terminal_grid")
            .spacing(egui::vec2(0.0, 0.0))
            .show(ui, |ui| {
                for row in &self.grid {
                    for cell in row {
                        ui.colored_label(cell.bg, cell.ch.to_string());
                    }
                    ui.end_row();
                }
            });

        // Toolbar: Run (F5), Stop, Step (F10), Pause
        ui.horizontal(|ui| {
            if ui.button("▶ Run").clicked() {
                return vec![PanelAction::RunScene(state.scene_path.clone().unwrap())];
            }
            if ui.button("⏸ Pause").clicked() { /* engine.pause() */ }
            if ui.button("↻ Reload").clicked() { /* trigger hot reload */ }
        });
    }
}
```

### Panel: File Browser

```rust
// crates/craft-editor/src/panels/file_browser.rs

pub struct FileBrowserState {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub filter: String,
}

struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileKind,  // Scene, Lua, Resource, Behavior, Directory
}

impl Panel for FileBrowserPanel {
    fn show(&mut self, ui: &mut egui::Ui, state: &mut EditorState) -> Vec<PanelAction> {
        // Double-click on .json → open scene or behavior
        // Double-click on .lua → open Lua editor
        // Right-click → new file, new folder, delete
    }
}
```

## Rationale

1. **Central state, per-frame drawing**: egui's immediate mode means state must live outside the UI. `EditorState` is the single source of truth — panels read from it, user actions write to it. This is cleaner than retained-mode frameworks where state is scattered across widget instances.

2. **JSON editor with schema auto-complete is the pragmatic behavior editing path**: Building visual behavior tools (state machine graph, action builder) is a v3 item. Schema-powered JSON editing gives 80% of the value at 10% of the cost. The same schema that validates agent output powers human auto-complete.

3. **Terminal preview as character grid**: No need for a GPU-accelerated 2D renderer in v2. The engine renders ANSI, the editor parses ANSI → character grid → egui colored labels. Simple, fast, debuggable.

## Godot Mapping

| Godot Editor Panel | Craft Editor Panel |
|--------------------|--------------------|
| Scene dock (tree of Nodes) | Scene Tree panel (recursive tree view) |
| Inspector (editable properties) | Inspector panel (component forms) |
| Script editor (GDScript code) | Lua Editor panel (ADR 0020) |
| No behavior editor (logic is in GDScript) | Behavior Editor panel (JSON + LSP for agent-authored rules) |
| 2D/3D viewport | Terminal Preview panel (ANSI character grid) |
| FileSystem dock | File Browser panel |
| Output / Debugger | Error Panel (structured errors in a table) |
| No agent integration | Agent Copilot panel (ADR 0019) |
