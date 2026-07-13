# ADR 0021: Editor UX Specification — Visual Design, Interactions, and Workflow

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: ADR 0017 (editor architecture), ADR 0018 (panels), ADR 0019 (Copilot), ADR 0020 (Lua editor). This ADR covers the visual design language, interaction patterns, and end-to-end authoring workflow.

## 1. Visual Design Language

### Color Scheme

Dark theme only for v2 (no light mode — all game editors default to dark for reduced eye strain during long sessions). Palette inspired by Godot 4's editor but adapted for egui's rendering:

```
Background hierarchy:
  #1e1e1e   Editor background (darkest)
  #252526   Panel background
  #2d2d30   Tab bar, toolbar
  #333337   Input fields, tree rows
  #3e3e42   Hovered row, selected item
  #505050   Borders, separators
  #007acc   Selection highlight (blue accent)

Text:
  #cccccc   Primary text
  #969696   Secondary text (labels, descriptions)
  #6e6e6e   Disabled text
  #ffffff   Headings, active tab

Status:
  #4ec9b0   Success / info
  #cca700   Warning
  #f44747   Error
  #569cd6   Keyword / link

Node type colors (in scene tree):
  #4ec9b0   Control nodes (teal)
  #569cd6   Node2D types (blue)
  #dcdcaa   Node3D types (yellow)
  #c586c0   Resource types (purple)
```

### Syntax Highlighting Colors (JSON + Lua editors)

```
JSON:
  #dcdcaa   Keys ("position")
  #ce9178   Strings ("player")
  #b5cea8   Numbers (100, 2.0)
  #569cd6   Booleans (true, false)
  #808080   Brackets, colons, commas
  #f44747   Validation errors (red squiggle)

Lua:
  #569cd6   Keywords (function, end, if, then, local)
  #ce9178   Strings ("scripts/classes/enemy")
  #b5cea8   Numbers
  #4ec9b0   Engine API (engine.*, node:*, require)
  #c586c0   Function names
  #dcdcaa   Method calls
  #6a9955   Comments (-- ...)
  #f44747   LSP diagnostics (red squiggle)
```

### Typography

| Context | Font | Size | Notes |
|---------|------|------|-------|
| Panel titles, headings | System sans-serif | 14px bold | egui default |
| Labels, buttons, UI text | System sans-serif | 13px | egui default |
| Code (JSON, Lua) | JetBrains Mono | 13px | Monospace, ligatures enabled |
| Terminal preview | JetBrains Mono | 13px | Same as code for alignment |
| Status bar | System sans-serif | 11px | Smaller, secondary color |

egui uses the system's default font. JetBrains Mono is loaded as a custom font for code editors and terminal.

### Panel Styling

| Element | Style |
|---------|-------|
| Panel background | `#252526` fill, 1px `#505050` border |
| Active tab | `#333337` fill, `#007acc` top-border accent (2px) |
| Inactive tab | `#2d2d30` fill, no accent |
| Tab hover | `#3e3e42` fill |
| Toolbar | `#2d2d30` fill, 28px height |
| Tree row | 22px height, alternating `#252526` / `#2a2a2d` (subtle zebra) |
| Tree row selected | `#094771` fill (dark blue, not bright) |
| Tree row hover | `#2a2d2e` fill |
| Expand/collapse arrow | ▸ collapsed, ▾ expanded, 12px |
| Input field | `#333337` fill, 1px `#505050` border, 22px height |
| Button | `#3e3e42` fill, 2px border-radius, hover=`#505050` |
| Scrollbar | 8px wide, `#424242` thumb, `#2d2d30` track |
| Splitter handle | 3px wide, `#505050`, hover=`#007acc` |

### Icon System — Node Type Icons

In the scene tree, each node type has a colored icon (12x12 px, generated programmatically, not from image files):

```
Icon shape → Node category:
  ■  Square       → Node (base type)
  ▶  Triangle     → 2D nodes (Node2D, Sprite2D)
  ●  Circle       → 3D nodes (Node3D, MeshInstance3D)
  ◆  Diamond      → Control/GUI nodes
  ⬡  Hexagon      → Resources
  ⚡ Lightning     → Transient components (badge on component row)
  ⚙  Gear         → Systems (in system browser)
  📡 Antenna      → Signals (in signal connection view)
```

Colors follow the node type color scheme above. This is the same system Godot uses — icon shape = category, color = type.

### Godot Visual Comparison

| Element | Godot 4 | Craft |
|---------|---------|-------|
| Color scheme | Dark blue-gray (#202531 bg) | Dark neutral gray (#1e1e1e bg), closer to VS Code |
| Node icons | SVG icons, detailed | Geometric shapes + color (simpler, generated) |
| Inspector | Property groups with colored headers | Component sections with transient badges |
| Scene tree | Icon + name, alternating rows | Same pattern, with type color dots |
| Script editor | GDScript syntax, built-in | Lua + JSON, LSP-powered coloring |
| Docking | Built-in dock system (undock, stack, float) | egui_dock (tab-based, rearrangeable) |
| Viewport | Full 2D/3D render with gizmos | Terminal ANSI grid (character-level precision) |

---

## 2. Interaction Patterns

### Keyboard Shortcuts

Aligned with Godot shortcuts where applicable. Non-Godot shortcuts added for Craft-specific features.

| Shortcut | Action | Godot Equivalent |
|----------|--------|-----------------|
| **File** | | |
| Ctrl+N | New scene | Godot: new scene dialog |
| Ctrl+O | Open project | Same |
| Ctrl+S | Save current file | Same |
| Ctrl+Shift+S | Save all | Same |
| **Edit** | | |
| Ctrl+Z | Undo | Same |
| Ctrl+Shift+Z | Redo | Same |
| Ctrl+X/C/V | Cut/Copy/Paste | Same |
| Ctrl+D | Duplicate selected node | Same |
| Delete | Delete selected node | Same |
| Ctrl+A | Select all | Same |
| Ctrl+F | Find in current file | Same |
| Ctrl+Shift+F | Find in project | Same |
| **Scene** | | |
| Ctrl+A (scene tree focused) | Add child node | Godot: Ctrl+A |
| F2 | Rename selected node | Same |
| Ctrl+Up/Down | Move node up/down in sibling order | (Godot: drag only) |
| **View** | | |
| F5 | Run scene | Godot: F5 (play current scene) |
| F6 | Run project (main scene) | Godot: F6 |
| F8 | Stop | Godot: F8 |
| F10 | Step one tick | Craft-only (replay step) |
| Ctrl+` | Toggle Agent Copilot | Craft-only |
| Ctrl+Shift+T | Toggle Terminal Preview | Craft-only |
| **Editor** | | |
| F11 | Toggle fullscreen | Same |
| Ctrl+Tab | Switch open tab | Same |
| Ctrl+W | Close current tab | Same |
| Ctrl+1-5 | Switch to panel (1=Scene Tree, 2=Inspector, 3=Terminal, 4=File Browser, 5=Agent) | (Godot: workspace switcher) |
| Ctrl+Shift+L | Toggle Lua editor / Behavior editor | Craft-only |

### Drag and Drop

| Operation | Behavior |
|-----------|----------|
| **Scene tree — reparent node** | Drag node onto another node → becomes child. Visual: insertion line indicator. |
| **Scene tree — reorder sibling** | Drag node between siblings → changes declaration order. Visual: horizontal insertion line. |
| **File browser → open** | Drag .json/.lua file onto editor area → opens in appropriate panel |
| **File browser → scene tree** | Drag script file onto node → sets `lua_class` on that node |
| **File browser → behavior editor** | Drag behavior JSON file → inserts `$ref` or copy content |
| **Component → inspector** | Drag component from one node to another → copy component value |
| **Invalid drop** | Cursor shows forbidden icon (🚫 circle-slash) |

### Context Menus (Right-Click)

**Scene tree — right-click on node**:
```
┌─────────────────────┐
│ Add Child Node    ▶ │ → submenu: type list
│ Duplicate           │
│ Rename           F2 │
│ ─────────────       │
│ Cut              ^X │
│ Copy             ^C │
│ Paste            ^V │
│ ─────────────       │
│ Attach Lua Script ▶ │ → file picker
│ Detach Lua Script   │
│ ─────────────       │
│ Delete           Del│
└─────────────────────┘
```

**Scene tree — right-click on empty space**:
```
┌─────────────────────┐
│ Add Root Node    ▶  │ → submenu: type list
│ Paste            ^V │
└─────────────────────┘
```

**File browser — right-click on file**:
```
┌─────────────────────┐
│ Open                │
│ Open With...     ▶  │ → Lua Editor / Behavior Editor / Text
│ ─────────────       │
│ Rename              │
│ Delete              │
│ ─────────────       │
│ Copy Path           │
└─────────────────────┘
```

**File browser — right-click on folder**:
```
┌─────────────────────┐
│ New File...      ▶  │ → submenu: Scene, Lua Script, Behavior, Resource
│ New Folder          │
│ ─────────────       │
│ Delete Folder       │
└─────────────────────┘
```

### Selection & Multi-Selection

| Action | Behavior |
|--------|----------|
| Single click | Select node in scene tree, populate inspector |
| Ctrl+Click | Toggle multi-select (for batch delete/copy) |
| Shift+Click | Range select (from last selected to clicked) |
| Click in empty space | Deselect all |
| Double-click node | Expand/collapse in tree |
| Double-click file | Open in editor |

### Undo/Redo

The editor maintains an undo stack for scene edits:

```rust
pub struct UndoEntry {
    pub description: String,              // "Add node 'enemy_3'"
    pub forward: SceneMutation,           // how to redo
    pub reverse: SceneMutation,           // how to undo
    pub affected_nodes: Vec<NodeId>,      // for selection restore
}

pub enum SceneMutation {
    AddNode { node_def: NodeDef, parent: NodeId, index: usize },
    RemoveNode { node_def: NodeDef, parent: NodeId, index: usize },
    SetComponent { node: NodeId, key: String, old: ComponentValue, new: ComponentValue },
    SetLuaClass { node: NodeId, old: Option<String>, new: Option<String> },
    ReorderChildren { parent: NodeId, old_order: Vec<NodeId>, new_order: Vec<NodeId> },
    // ... composite mutations for structural changes
}
```

Undo stack depth: 100 entries (same as Godot). Stack is per-scene (cleared when switching scenes).

### Status Bar

Bottom bar showing context-dependent information:

```
▌ scene.json │ 12 nodes, 3 scripts │ tick: 420 │ 60 FPS │ errors: 0 │ saved ▐
```

---

## 3. End-to-End Authoring Workflow

### Workflow 1: New Project → First Scene → Run Preview

```
1. Launch Craft Editor
   ┌──────────────────────────────┐
   │         Craft Editor         │
   │                              │
   │  [Open Project]  [New Proj.] │
   │                              │
   │  Recent:                     │
   │  ~/games/tower_defense       │
   └──────────────────────────────┘

2. Click "New Project" → native folder picker (rfd)
   → creates: craft.toml + games/<name>/ directory

3. File Browser shows project tree:
   games/my_game/
   ├── (empty)

4. Right-click games/my_game/ → New File → Scene
   → creates scene.json with template:
   {
     "kind": "scene",
     "id": "my_game_main",
     "root": {
       "id": "root",
       "type": "Node",
       "parent": null,
       "children": [],
       "components": {},
       "behaviors": []
     }
   }

5. Double-click scene.json → opens in editor:
   - Scene Tree: shows "root (Node)"
   - Inspector: shows root's empty components
   - Behavior Editor: empty tab (no behaviors yet)

6. Right-click "root" → Add Child Node → type "Player"
   → Inspector shows: position, hp, speed (from Player type schema)

7. Edit component values in Inspector:
   - set position to [5, 5]
   - set hp to 100

8. Right-click "root" → Add Child Node → type "Enemy"
   → Inspector shows Enemy components

9. Right-click "enemy_1" → Attach Lua Script → pick scripts/classes/enemy.lua
   → Lua Editor opens, shows on_tick() template

10. Press F5 → Terminal Preview panel shows the game running
    → Enemy at [10, 20], Player at [5, 5]
    → Enemy moves, signals fire, game logic runs

11. Edit enemy.lua: change speed from 2.0 to 5.0
    → Ctrl+S → hot reload
    → Enemy immediately moves faster in Terminal Preview
```

### Workflow 2: Agent-Assisted Scene Creation

```
1. Human creates scene with root node

2. Opens Agent Copilot (Ctrl+`)
   Types: "Add 5 enemies in a row along the top of the grid,
          each with random hp between 30-70"

3. Agent receives context:
   - Active file: scene.json
   - Selected node: root
   - Engine schema: all node types, actions, systems

4. Agent responds with structured diff:
   - Add 5 Enemy nodes with positions [0,0]..[20,0]
   - Each with random hp using engine.rng()
   - Each with lua_class pointing to scripts/classes/enemy.lua

5. Human clicks [Preview Diff] → side-by-side diff viewer
   - Left: current scene (just root)
   - Right: proposed scene (root + 5 enemies)

6. Human clicks [Modify] on enemy_3:
   → opens inspector on enemy_3
   → manually changes hp from 42 to 80

7. Human clicks [Accept All]
   → Scene Tree updates: shows 5 new enemies
   → SceneDef modified, file marked dirty

8. Human presses Ctrl+S to save, F5 to test
```

### Workflow 3: Debugging with Replay

```
1. Game is running, something looks wrong at tick 200

2. Press F8 to stop

3. Open recording from recent run:
   Engine → Load Recording → pick recording_2026-07-13.craftrec

4. Replay scrubber appears:
   ┌──────────────────────────────────────────┐
   │  Replay                                 │
   │  ├────●─────────────┤  tick 200 / 500    │
   │  ◀◀  ◀  ▶   ▶▶  [Step]  [Play]         │
   │                                          │
   │  State at tick 200:                      │
   │  enemy_3: hp=0 (died unexpectedly)       │
   │  enemy_3: signal "damage_taken" @ t=198  │
   │  Player: attack_power=100 (too high)     │
   └──────────────────────────────────────────┘

5. Scrub to tick 198 → see enemy_3 received damage=100
   → Open Player inspector → attack_power=100 (should be 10)

6. Press F2 on Player node, change attack_power to 10
   → Save → Replay from tick 0
   → enemy_3 now survives past tick 200
```

### Workflow 4: Hot Reload Iteration

```
1. Game is running in Terminal Preview (F5)

2. Scene Tree is live-updating:
   - Running nodes shown in green (active in engine)
   - Component values show live state (not just file content)
   - Transient components show remaining counter

3. Inspector shows live component values:
   Enemy_3:
     hp: 45/100 (live, changes each tick)
     flash_tint: "red" (transient, 3 ticks remaining)
     position: [15, 8]

4. File-based edit during run:
   - Add a new Tower node in scene.json via Behavior Editor
   - Ctrl+S → file watcher detects change
   - Engine hot-reloads: computes diff, spawns Tower node
   - Tower appears in Terminal Preview at next tick
   - Agent Copilot notification: "Hot reload applied: 1 node added"

5. Lua hot reload:
   - Edit enemy.lua: add "if hp < 20 then speed = speed * 2 end"
   - Ctrl+S → engine re-requires enemy.lua
   - All Enemy instances immediately use updated script
   - No restart, no state loss
```

### Workflow 5: Agent Debugging Loop

```
1. Game crashes at tick 847
   → Error Panel shows:
     RuntimeError { tick: 847, node: "enemy_7",
       message: "Tried to read 'target.position' but target node was destroyed at tick 845",
       suggestion: "Add nil check: if target then ... end" }

2. Human clicks [Ask Agent to Fix] in error panel

3. Agent context includes:
   - The error (full structured Error object)
   - enemy.lua (the script that crashed)
   - The scene state at tick 845 and 847 (from recording)

4. Agent proposes fix:
   - Add guard: `if target and target.position then ... end`
   - Shows as inline diff in Lua Editor

5. Human reviews [Accept]
   → Script updated, Ctrl+S → hot reload
   → Replay from tick 840 → error no longer occurs
```

---

## 4. Godot UX Comparison

### What Godot Does Well — We Copy

| Godot UX Pattern | Craft Implementation |
|-----------------|---------------------|
| **Scene dock + Inspector split**: click node → see properties | Same — Scene Tree panel + Inspector panel, linked selection |
| **FileSystem dock**: drag-and-drop onto scene | Same — File Browser panel with drag to Scene Tree |
| **F5 to run, F8 to stop**: muscle-memory shortcuts | Identical keybindings |
| **Script editor attached to node**: click node → see its script | Lua class binding: click node → Lua Editor opens its class file |
| **Output panel at bottom**: errors appear inline | Error Panel at bottom of editor (same position) |
| **Undo/redo per-scene**: granular edits | Same — per-SceneDef undo stack, 100 levels |
| **Dark theme default**: reduces eye strain | Same — darker neutral palette |
| **Node type icons**: visual category indicators | Same — geometric shape + color |

### What Godot Does Poorly — We Improve

| Godot Pain Point | Craft Improvement |
|-----------------|-------------------|
| **No AI assistance**: all edits manual | Agent Copilot: suggest nodes, behaviors, Lua code, debug |
| **No dry-run**: must actually run to test behavior | `dryRun` in editor: select a node, compose actions, see predicted diff — no run needed |
| **No replay/step debugging**: must use print() | Replay scrubber: pause, step, inspect any tick. Timeline scrubber with state diff. |
| **GDScript only**: learning curve for non-Python devs | Lua 5.5: universal, smaller, 30 years of ecosystem (LuaRocks). Plus JSON for agents. |
| **Script reload loses state**: variable reset on reload | Hot reload preserves component state. Lua re-require keeps `self` table alive. |
| **No schema introspection**: must read docs to know what methods exist | `engine.getSchema()` powers LSP auto-complete for both JSON and Lua. Always up-to-date. |
| **No structured errors**: parse stderr for line numbers | Structured EngineError JSON with json_path + suggestion. Agent and human both parse it. |
| **No recording/replay**: can't rewind to debug | Craft-native: record → replay → scrub → diff → fix → replay again. Counterfactual reasoning built in. |
