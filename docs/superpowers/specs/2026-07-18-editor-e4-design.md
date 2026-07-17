# Editor E4 Design: UX Polish

> v0.4.0 | 2026-07-18 | ADR 0018 Appx B

E4 是编辑器 v2 的最后一个里程碑——从功能完整到体验打磨。不做新功能，专注交互质量和视觉一致性。

参考 ADR: 0018 (Appx B: Keyboard Shortcuts, Drag & Drop, Context Menus, Undo/Redo), 0017 (编辑器架构).

## 0. 分支依赖

E4 基于 `feat/editor-e3` 的完整 E3 实现，而非主分支。**E4 分支需 rebase 到 `feat/editor-e3` 或等待 E3 合并到 main 后再开始。**

若需立即并行开发，E4 仅修改与 E3 无交集的文件（undo.rs, theme.rs, app.rs 快捷键部分, scene_tree.rs, inspector.rs, file_browser.rs），避免触碰 agent_panel.rs。AgentPanel 的 undo 包裹点推迟到 E3 合并后。

---

## 1. Undo/Redo — 命令模式

### 1.1 设计

采用 Godot 同款命令模式：每次编辑操作注册正向+逆向闭包，推入环形栈。全量快照和 SceneDiff 方案均不采用。

```
新文件: crates/craft-editor/src/undo.rs
```

```rust
pub struct UndoRedo {
    actions: Vec<Action>,
    current: i32,
    max_steps: usize,
}

struct Action {
    name: String,
    do_ops: Vec<Box<dyn FnOnce(&mut EditorState)>>,
    undo_ops: Vec<Box<dyn FnOnce(&mut EditorState)>>,
}

impl UndoRedo {
    pub fn new(max_steps: usize) -> Self;
    pub fn begin_action(&mut self, name: &str);
    pub fn add_do<F: FnOnce(&mut EditorState) + 'static>(&mut self, f: F);
    pub fn add_undo<F: FnOnce(&mut EditorState) + 'static>(&mut self, f: F);
    pub fn commit_action(&mut self);
    pub fn undo(&mut self, state: &mut EditorState) -> bool;
    pub fn redo(&mut self, state: &mut EditorState) -> bool;
    pub fn has_undo(&self) -> bool;
    pub fn has_redo(&self) -> bool;
    pub fn clear(&mut self);
}
```

### 1.2 操作包裹点

每个编辑点用 `begin_action` / `commit_action` 包裹：

| 操作 | 位置 | do | undo |
|------|------|-----|------|
| 改 component 值 | Inspector panel | set_component_value(new) | set_component_value(old) |
| 添加子节点 | SceneTree (Ctrl+Shift+A) | scene.add_node(node) | scene.remove_node(id) |
| 删除节点 | SceneTree context menu | scene.remove_node(id) | scene.add_node(克隆) |
| 复制节点 | SceneTree context menu | scene.add_node(克隆) | scene.remove_node(clone_id) |
| 改 behavior JSON | BehaviorEditor / Inspector inline | 写新 JSON | 写旧 JSON |
| Agent accept diff | AgentPanel | 同 accept 流程 | 反向 SceneDiff |
| 文件浏览器重命名/删除 | FileBrowser | 实际 IO | 逆向 IO |

### 1.3 合并策略

800ms 内同名操作（如连续修改同一个 component 值）合并为一个 action。`text_edit` 场景下每个 keypress 不产生独立历史条目。

### 1.4 键盘绑定

- Ctrl+Z → `undo(&mut state)`
- Ctrl+Shift+Z → `redo(&mut state)`

---

## 2. 键盘快捷键

### 2.1 完整映射

| Scope | Key | Action | 已有 |
|-------|-----|--------|------|
| File | Ctrl+S | Save + trigger hot reload | ✅ |
| Scene | F5 | Run | ✅ |
| Scene | F8 | Stop | ✅ |
| Scene | F10 | Step one tick | ✅ |
| Scene | Ctrl+Shift+A | Add child node to selected | ❌ |
| View | Ctrl+1 | Focus Scene Tree panel | ❌ |
| View | Ctrl+2 | Focus Inspector panel | ❌ |
| View | Ctrl+3 | Focus Terminal Preview panel | ❌ |
| View | Ctrl+4 | Focus File Browser panel | ❌ |
| View | Ctrl+5 | Focus Agent Copilot panel | ❌ |
| Edit | Ctrl+Z | Undo | ❌ |
| Edit | Ctrl+Shift+Z | Redo | ❌ |

### 2.2 实现

快捷键处理集中在 `app.rs` 的 `update()` 中，与已有 F5/F8/F10/Ctrl+S 同位置。Ctrl+1-5 通过 `egui_dock` 的 `DockState::set_focused_tab()` API 实现。若 API 不可用，回退到通过 `PanelAction::FocusPanel(DockKind)` 实现。

---

## 3. 右键菜单

### 3.1 场景树节点右键

```
┌─────────────────────┐
│ Add Child Node   ▶  │ → 列出已注册 node type
│ Duplicate           │
│ Rename...           │
│ Attach Lua      ▶  │ → 列出 .lua 文件
│ ─────────────────  │
│ Delete              │
└─────────────────────┘
```

实现：在 SceneTree panel `show()` 中检测 `response.secondary_clicked()`，弹出 `egui::Area` 或 `egui::Window`（不可移动，跟随鼠标位置）。

### 3.2 文件浏览器

**文件右键：**
```
┌─────────────────┐
│ Open             │
│ ─────────────── │
│ Delete           │
└─────────────────┘
```

**文件夹右键：**
```
┌─────────────────┐
│ New File     ▶  │ → Scene / Lua / Behavior / Resource
│ New Folder       │
│ ─────────────── │
│ Delete           │
└─────────────────┘
```

### 3.3 行为编辑器

不需要右键菜单——Inspector 已有内联编辑，BehaviorEditor 是独立文件编辑器。

---

## 4. 拖拽

### 4.1 场景树：重新父级/排序

- 拖起节点 → 悬停目标节点 → 松手 = 改变 `node.parent`
- 拖起节点 → 悬停同级节点之间 → 松手 = 改变 `nodes` 数组顺序
- 无效拖拽（拖到自己子节点上）显示禁止光标

实现：egui 0.31 无原生 tree drag-drop。自建于 `response.drag_started()` / `drag_released()` + `ctx.input().pointer.hover_pos()`。拖拽时绘制半透明预览。

### 4.2 文件浏览器 → 节点

- 拖 `.lua` 文件到场景树节点 → 设置 `node.lua_class` 为该文件名
- 无效文件类型显示禁止光标

### 4.3 文件浏览器 → 编辑器

- 拖 `.json` 文件到编辑器区域 → 打开文件
- 拖 `.behavior.json` 到编辑器 → 在 BehaviorEditor 中打开
- 拖 `.lua` 到编辑器 → 在 LuaEditor 中打开

---

## 5. 视觉设计语言

### 5.1 色彩体系

```
背景:     #1A1A2E  (深蓝黑)
面板:     #16213E  (中蓝黑)
选中:     #0F3460  (亮蓝)
主色调:   #6C9FFF  (天蓝，按钮/链接)
强调色:   #E94560  (红，删除/错误)
成功色:   #0F9B58  (绿，成功状态)
文字:     #E0E0E0  (浅灰)
次要文字: #A0A0A0  (中灰)
```

### 5.2 节点类型 → 颜色 + emoji

| 类型 | 颜色 | Emoji | 用途 |
|------|------|-------|------|
| Enemy | #E94560 | 🔴 | 场景树、Inspector header |
| Tower | #6C9FFF | 🔵 | |
| Player | #0F9B58 | 🟢 | |
| Projectile | #FFD700 | 🟡 | |
| Resource | #A0A0A0 | ⚪ | |
| Default | #A0A0A0 | ⬜ | 未匹配类型 |

节点类型通过字符串前缀匹配（大小写不敏感）：`enemy_*` → Enemy, `tower_*` → Tower, `player_*` → Player, 含 `projectile` → Projectile, `resource_*` → Resource。

### 5.3 egui Visuals 覆写

在 `craft-editor/src/theme.rs` 中扩展：

```rust
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    // 覆写关键属性
    visuals.panel_fill = Color32::from_rgb(22, 33, 62);
    visuals.window_fill = Color32::from_rgb(26, 26, 46);
    visuals.override_text_color = Some(Color32::from_rgb(224, 224, 224));
    visuals.selection.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 33, 62);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(108, 159, 255);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(15, 52, 96);
    visuals.widgets.active.bg_fill = Color32::from_rgb(108, 159, 255);
    visuals.window_rounding = egui::Rounding::same(6.0);
    visuals.window_shadow = egui::epaint::Shadow::small_light();
    visuals.menu_rounding = egui::Rounding::same(4.0);
    visuals.button_frame = true;
    visuals.indent_has_left_vline = false;
    visuals.striped = false;
    visuals.slider_trailing_fill = true;
    visuals.collapsing_header_frame = true;
    ctx.set_visuals(visuals);

    // 字体
    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.1;
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.indent = 16.0;
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width_inner: 4.0,
        bar_width_outer: 4.0,
        ..Default::default()
    };
    ctx.set_style(style);
}
```

### 5.4 节点类型图标

在 SceneTree 和 Inspector header 中，节点名前面显示对应 emoji。使用已有 `type_name` 字段做前缀匹配。

---

## 6. 文件结构

```
crates/craft-editor/src/
├── undo.rs                     # NEW — UndoRedo
├── theme.rs                    # MOD — 完整 egui Visuals 覆写
├── app.rs                      # MOD — 键盘快捷键 + undo/redo 绑定
├── panels/
│   ├── scene_tree.rs           # MOD — 右键菜单 + 拖拽 + Ctrl+Shift+A + 图标
│   ├── inspector.rs            # MOD — 图标 + undo 包裹
│   ├── file_browser.rs         # MOD — 右键菜单 + 拖拽源
│   ├── behavior_editor.rs      # MOD — undo 包裹
│   └── agent_panel.rs          # MOD — undo 包裹 (accept diff)
└── state.rs                    # MOD — 添加 `undo_redo: UndoRedo` 字段
```

---

## 7. 测试策略

| 层级 | 测试 | 文件 |
|------|------|------|
| 单元 | UndoRedo 栈操作 | `tests/e4_undo.rs` |
| 单元 | 节点类型颜色匹配 | `tests/e4_theme.rs` |
| 集成 | 快捷键完整映射 | `tests/e4_keybindings.rs` |
| 集成 | 右键菜单弹出 | `tests/e4_context_menu.rs` |
| 集成 | 拖拽状态机 | `tests/e4_dragdrop.rs` |
| 集成 | 面板渲染烟雾测试 | `tests/e4_panels_kittest.rs` |

---

## 8. 接受标准

1. Ctrl+Z 撤销最近操作，状态恢复正确（Inspector/SceneTree/Behavior 同步）
2. Ctrl+Shift+Z 恢复已撤销操作
3. F5/F8/F10/Ctrl+S 快捷键保持原有行为不变
4. Ctrl+1-5 切换到对应面板
5. Ctrl+Shift+A 在选中节点下添加子节点（弹出类型选择）
6. 场景树节点右键显示菜单：Add Child / Duplicate / Rename / Attach Lua / Delete
7. 文件浏览器文件右键：Open / Delete；文件夹右键：New File / New Folder / Delete
8. 场景树节点可拖拽重新父级/排序
9. `.lua` 文件可拖拽到场景树节点设置 lua_class
10. 暗色主题覆盖全局 egui 视觉样式，颜色体系统一
11. 节点类型在 SceneTree 中显示对应颜色 emoji 图标
12. `cargo clippy --workspace -- -D warnings` clean
13. `cargo fmt --check` clean
14. `cargo test -p craft-editor` 全部通过
