# craft-editor

The v2 desktop editor for Craft.

## Quickstart

Run the editor from the workspace root:

```bash
cargo run -p craft-editor
```

To open a project directly:

```bash
cargo run -p craft-editor -- path/to/craft.toml
```

## Panels

- **Scene Tree**: Edit the scene hierarchy; right-click nodes for the context menu.
- **Inspector**: Edit the selected node's components and inline behavior JSON with schema-aware validation.
- **Files**: Browse project files. Double-click `.behavior.json` or `.lua` files to open in their respective editors.
- **Terminal Preview**: Run the embedded engine at 60 Hz with `F5`.
- **Behavior Editor**: Standalone `.behavior.json` editor with schema validation and auto-complete (Ctrl+Space).
- **Lua Editor**: Edit `.lua` files with syntax highlighting, hot-reload on save, and LuaLS integration (Ctrl+Space for completions, inline diagnostics).
- **Agent Copilot**: Stub; planned for E3.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save scene (warns if behavior edits are unapplied) |
| `F5` | Run scene |
| `F8` | Stop scene |
| `F10` | Step one tick |
| `Ctrl+Space` | Trigger auto-complete (JSON behaviors, Lua with LuaLS) |
| `Ctrl+1`–`Ctrl+4` | Focus panels 1–4 |

## Persistence

Dock layout and recent projects are stored in `~/.config/craft-editor/`, or the platform-equivalent configuration directory.
