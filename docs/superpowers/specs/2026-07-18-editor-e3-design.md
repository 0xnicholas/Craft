# Editor E3 Design: Agent Copilot

> v0.3.0 | 2026-07-18 | ADR 0019

Agent Copilot 是编辑器内的 AI 协作面板。Agent 能看到人类所见（场景、选中节点、最近变更），提出结构化的场景修改建议（SceneDiff），人类通过 accept/reject 审查。

参考 ADR: 0019 (面板设计), 0017 (编辑器架构), 0014 (AI 原生原语), 0007 (同步桥接), 0015 (单线程 + 后台 reader)

---

## 1. 架构概览

三层设计：

```
┌──────────────────────────────────────────────┐
│ AgentPanel (UI)                              │
│ 聊天消息列表 · 流式文本 · Diff 预览弹窗        │
│ Accept/Reject 按钮 · 上下文栏                 │
├──────────────────────────────────────────────┤
│ AgentBackend (数据)                          │
│ AgentClient · ContextBuilder · ToolRegistry   │
│ reqwest SSE 流式 · mpsc channel 回 UI 线程    │
├──────────────────────────────────────────────┤
│ Engine Primitives (内核)                     │
│ SceneDiff · lint · dry_run · explain(新)     │
└──────────────────────────────────────────────┘
```

**文件结构**：

```
crates/craft-editor/src/
├── agent/
│   ├── mod.rs              # 模块入口，re-export
│   ├── client.rs           # AgentClient: reqwest HTTP + SSE 解析
│   ├── context.rs          # ContextBuilder: EditorState → AgentContext
│   ├── tools.rs            # ToolRegistry + 本地执行
│   └── types.rs            # AgentStreamEvent, ChatMessage, ToolDef, AgentError
├── panels/
│   └── agent_panel.rs      # 替换占位符 — 聊天 UI + diff 审查
├── state.rs                # AgentPanelState, AgentSuggestion, SuggestionStatus
└── app.rs                  # 流式 drain + PanelAction 分发
```

**关键约束**：
- 引擎单线程 — LLM HTTP 调用在后台 `std::thread` 中执行
- 编辑器通过 `mpsc::Receiver<AgentStreamEvent>` 每帧 drain（与 LuaEditorPanel 的 LSP 完成轮询模式一致）
- API key 通过 `CRAFT_LLM_API_KEY` 环境变量或 `craft.toml` `[agent]` 段配置
- 新增依赖: `reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }` 加入 `crates/craft-editor/Cargo.toml`

**Accept 写入路径** (ADR 0017 + 0019 兼容)：
- **预览运行中** (F5): accept 写入文件 → `engine.apply_hot_reload()` 热加载 diff。显示加载中提示。
- **预览停止 / 编辑中**: accept 写入 `SceneState.def` 内存 → 标记 dirty。无实时引擎状态需要同步。

---

## 2. AgentClient

### 2.1 配置

优先级: 环境变量 > `craft.toml` `[agent]` > 硬编码默认值

```toml
# craft.toml (新增段)
[agent]
provider = "openai"               # openai | anthropic | custom
api_base = "https://api.openai.com/v1"
model = "gpt-4o"
api_key_env = "CRAFT_LLM_API_KEY"
```

`AgentConfig` 定义在 `crates/craft-editor/src/agent/config.rs`，通过 `AgentConfig::load(root: &Path)` 从 `craft.toml` 的 `[agent]` 段解析，缺失字段回退到环境变量和默认值。与 `read_lua_section` 的解析模式一致。

### 2.2 API

```rust
pub struct AgentClient {
    inner: Arc<AgentClientInner>,
}

struct AgentClientInner {
    http: reqwest::blocking::Client,
    config: AgentConfig,       // 已解析的 key、base URL、model
    request_in_flight: AtomicBool,
}

impl AgentClient {
    pub fn new(config: AgentConfig) -> Self;
}
```

`AgentConfig` 存储已解析的值（从 `api_key_env` 读出的实际 key 字符串），而非环境变量名。

### 2.3 流式协议

```rust
pub enum AgentStreamEvent {
    /// 来自 LLM 的逐 token 文本增量
    Token(String),
    /// LLM 完成了完整响应
    Done {
        full_text: String,
        tool_calls: Vec<ToolCall>,
    },
    /// 流错误
    Error(String),
}
```

### 2.4 线程模型

```
UI 帧循环                           后台 std::thread
    │                                    │
    ├── client.chat(messages, tools) ──→ │
    │                                    ├── reqwest POST (stream: false)
    │                                    │   等待完整响应
    │  ←── rx.try_recv() ───────────────│     Done { full_text, tool_calls }
    │                                    │     关闭 channel，线程退出
    │                                    │
    │ (回到 UI 线程)                      │
    │ 若 tool_calls 非空:                │
    │   本地执行 tool (lint, dry_run...)  │
    │   追加 assistant + tool 消息        │
    │   client.chat(...) ───────────────→│ (新线程)
    │   ...重复直到无 tool call...        │
    │                                    │
    │   client.chat(messages, tools) ──→ │
    │                                    ├── reqwest POST (stream: true)
    │                                    │   最终响应，流式返回文本
    │  ←── Token(delta) ─────────────────│
    │  ←── Token(delta) ─────────────────│
    │  ←── Done { full_text } ───────────│
    │                                    │
```

关键规则：
- Tool 检测阶段: `stream: false`，等完整响应后在 UI 线程执行 tool
- 最终文本阶段: `stream: true`，逐 token 推送
- `reqwest::blocking::Client` 是 `Send`，可以安全地传入 `Arc` 给线程
- `AgentClient` 内部用 `Arc<AgentClientInner>` 持有 http client 和 config
- 同一时间最多一个活跃请求（`Arc<AtomicBool>` 保护）

---

## 3. ContextBuilder

每次 agent 请求前，ContextBuilder 从 `EditorState` 构建 `AgentContext`，格式化为 system message。

```rust
pub struct AgentContext {
    pub active_file: Option<PathBuf>,
    pub scene_name: Option<String>,
    pub node_count: usize,
    pub selected_node: Option<NodeSummary>,
    pub visible_components: Vec<String>,
    pub recent_changes: Vec<ChangeRecord>,
    pub engine_schema: Value,
}

pub struct NodeSummary {
    pub id: String,
    pub type_name: String,
    pub component_keys: Vec<String>,
    pub component_types: Vec<(String, String)>,  // key → type name (Int, Float, Vec2, ...)
    pub behavior_count: usize,
    pub lua_class: Option<String>,
}

pub struct ChangeRecord {
    pub timestamp: Instant,
    pub description: String,
}
```

### 上下文注入格式

ContextBuilder 生成一个 system message（追加到现有 system prompt 之后）：

```
[EDITOR CONTEXT]
Active file: scene.json (tower_defense, 14 nodes)
Selected node: tower_1 (Tower)
  Components: cooldown, range, damage
  Behaviors: 1 (on_tick)
  Lua class: towers.target_priority
Recent changes:
  - modified tower_1.cooldown (10 → 5)
  - added enemy_spawner node
Engine schema version: 1.0
```

不发送完整 `Scene` JSON — 太大且 token 浪费。

---

## 4. ToolRegistry + 本地执行

### 4.1 工具定义

| 工具名称 | 描述 | 参数 | 由谁实现 |
|----------|------|------|-----------|
| `lint` | 场景静态分析 | `{}` | `craft_kernel::lint()` (已有) |
| `dry_run` | 模拟动作的无副作用预览 | `{ node_id, actions }`，其中 `actions` 通过 `serde_json::from_value` 反序列化为 `Vec<Action>` | `craft_kernel::evaluate_dry_run(scene, registry, node_id, &actions)` (已有) |
| `explain` | 节点结构化 JSON | `{ node_id }` | **新增** `craft_kernel::explain_node()` |
| `read_scene` | 当前场景摘要 | `{}` | 返回 `SceneInfo` JSON |
| `read_node` | 节点的完整 JSON | `{ node_id }` | 返回节点 `serde_json` |
| `propose_diff` | 建议修改场景 | `{ description, diff }` | 内联于响应，不走 tool 回环 |

**关键设计**：`propose_diff` 不实现为 tool call。Agent 在最终响应中直接返回 `SceneDiff` JSON。理由：
- 避免额外往返（tool call → 执行 → 结果返回 → LLM 再次生成）
- `SceneDiff` JSON 可能很大 — 作为 tool 结果回传给 LLM 浪费 token
- 编辑器从响应体中解析 `diffs` 数组直接展示

### 4.2 Tool 调用流程（两阶段）

**阶段 1: Tool 检测（非流式）**

```
Agent 发送请求 (system prompt + tools + context + user message)
  stream: false
     │
     ▼
LLM 完整响应:
  有 tool_calls → 在 UI 线程本地执行 → 追加 assistant + tool 消息
     │
     ▼
  重复阶段 1 (最多 3 轮 tool call)
  无 tool_calls → 进入阶段 2
```

**阶段 2: 流式文本生成**

```
Agent 发送同一对话 (含 tool 结果)
  stream: true
     │
     ▼
LLM 流式响应:
  Token(delta) × N → Done { full_text }
     │
     ▼
编辑器解析 full_text 中的 { reply, diffs }
```

tool 执行在 UI 线程进行，因为引擎类型（Engine, Scene, NodeRegistry）不是 Send/Sync。每轮 tool 检测后，AgentPanel 的 `show()` 方法 drain 响应、执行 tool、然后发起下一轮请求（如需要）。

若第 3 轮 tool call 后仍返回 tool_calls，编辑器追加 system message "Maximum tool rounds reached. Provide your final answer." 并进入阶段 2。

**tool 结果格式**: 工具返回的 JSON 被格式化为字符串放入 tool result 消息中。
- `lint` → `"3 warnings: unreachable state 'idle' in node 'tower'..."`
- `dry_run` → `"component changes: tower.cooldown Updated(10→5)..."`
- `explain_node` → `{"id":"tower_1","type":"Tower",...}` (紧凑 JSON)

### 4.3 `explain_node` Engine 方法 (新增)

在 `craft-kernel` 中添加：

```rust
/// 返回节点的小型结构化 JSON：id, type, lua_class, components (key → type + value),
/// behavior count, children count。用于 LLM 工具调用，token 占用小。
pub fn explain_node(node: &Node, registry: &NodeRegistry) -> serde_json::Value;
```

输出示例：
```json
{"id":"tower_1","type":"Tower","lua_class":"towers.target_priority","components":{"cooldown":{"type":"int","value":5},"range":{"type":"float","value":10.0}},"behaviors":1,"children":0}
```

---

## 5. Diff 审查流程

### 5.1 响应解析

编辑器从 LLM 响应中解析结构化 JSON。期望格式：

```json
{
  "reply": "自然语言回复文本...",
  "diffs": [
    {
      "description": "将 tower_1 的冷却时间从 10 改为 5",
      "diff": { "component_changes": { "tower_1": { "cooldown": { ... } } } }
    }
  ]
}
```

如果解析失败: 显示原始文本，不显示 diff UI。

### 5.2 AgentSuggestion 状态机

```
Pending ──→ Accepted  (diff 已应用)
  │
  ├───────→ Rejected  (已丢弃)
  │
  └───────→ Failed { reason }  (apply_scene_diff 失败，可重试)
```

### 5.3 Accept

**预览运行中**:
1. 先尝试 `engine.apply_hot_reload(&diff_scene)` 验证 diff 合法
2. 成功后将 diff 场景序列化为 JSON 写入磁盘
3. 失败时清理临时场景，状态变为 `Failed`，不写入文件
4. 状态消息: "Hot-reloaded: tower_1 cooldown 10→5"

**预览停止 / 编辑中**:
1. `apply_scene_diff(&mut scene.def, &engine.node_registry(), &suggestion.diff)` (已有)
2. `scene_state.last_saved_hash` 保持不变 — 标记 dirty
3. 状态消息: "Applied: tower_1 cooldown 10→5"

两种情况都将建议状态更新为 `Accepted`。

### 5.4 Reject

1. 建议状态变为 `Rejected`
2. 保留在聊天记录中（灰显），供历史查阅

---

## 6. UI 设计

### 6.1 AgentPanel 布局

```
┌──────────────────────────────────┐
│ Agent Copilot                    │
├──────────────────────────────────┤ ← 上下文栏 (CollapsingHeader)
│ 👁 scene.json · tower_1 (Tower) │   可折叠，默认打开
├──────────────────────────────────┤
│          (聊天区域)              │ ← ScrollArea::vertical()
│ ┌──────────────────────────────┐ │   每条消息是一个气泡
│ │ 🧑 User                      │ │
│ │ 添加 3 个 Enemy              │ │
│ └──────────────────────────────┘ │
│ ┌──────────────────────────────┐ │
│ │ 🤖 Agent                     │ │
│ │ 已生成 3 个 Enemy 节点...    │ │
│ │                               │ │
│ │ [Preview Diff] [Accept] [✕] │ │
│ └──────────────────────────────┘ │
├──────────────────────────────────┤
│ [输入消息...]              [→]   │
└──────────────────────────────────┘
```

### 6.2 Diff 预览弹窗

点击 `Preview Diff` 打开 `egui::Window` 模态窗口：

```
┌────────── Diff Preview ──────────┐
│ Description                      │
│ "将 cooldown 改为 5"             │
├──────────────────────────────────┤
│ Current          │ Proposed      │
├──────────────────┼───────────────┤
│ (JSON 并排)      │ (JSON 并排)   │
├──────────────────┴───────────────┤
│           [Accept] [Close]       │
└──────────────────────────────────┘
```

并排渲染：左侧 TextEdit (只读) 显示当前状态，右侧 (只读) 显示建议状态。
- 新增节点: 左侧显示 "— new node —"
- 删除节点: 左侧显示当前内容，右侧显示 "— removed —"

### 6.3 流式渲染

- 后台线程通过 channel 发送 `Token(text)`
- 每帧 `show()` drain receiver: 追加 token 到 `streaming_text`
- 渲染为斜体灰色文本（表示"还在生成中"）
- 收到 `Done` 后: 将 `streaming_text` 转为最终消息，解析 diffs

### 6.4 键盘快捷键

| 快捷键 | 操作 |
|--------|------|
| `Ctrl+Enter` | 发送消息 |
| `Ctrl+Shift+A` | 聚焦 Agent Copilot 面板 |

---

## 7. 状态 + PanelAction

### 7.1 AgentPanelState

替换 `state.rs` 中的 `AgentPanelStub`。

```rust
pub struct AgentPanelState {
    pub messages: Vec<AgentMessage>,
    pub input: String,
    pub is_streaming: bool,
    pub streaming_text: String,
}

pub enum AgentMessage {
    User { text: String },
    Agent { text: String, suggestions: Vec<AgentSuggestion> },
    System { text: String },
}

pub struct AgentSuggestion {
    pub id: String,                // 递增计数器: "suggestion-1", "suggestion-2"
    pub description: String,
    pub diff: craft_kernel::SceneDiff,
    pub status: SuggestionStatus,
}

pub enum SuggestionStatus { Pending, Accepted, Rejected, Failed { reason: String } }
```

### 7.2 PanelAction 扩展

不直接在 `PanelAction` 中携带 `SceneDiff`（避免污染轻量级消息枚举）。Agent 操作在 `AgentPanel` 内部通过本地回调处理：

```rust
// agent_panel.rs show() 中，不返回 PanelAction
// 直接操作 state.scene 和 state.panels.agent_panel
```

Accept: `show()` 内调用 `apply_scene_diff` + 标记 dirty + 更新 suggestion 状态。
Reject: `show()` 内更新 suggestion 状态。

无需新增 `PanelAction` 变体。

---

## 8. 错误处理

| 场景 | 处理 |
|------|------|
| API key 缺失 | 状态消息: "Set CRAFT_LLM_API_KEY env var or [agent] api_key in craft.toml" |
| HTTP 错误 (4xx/5xx) | 聊天中显示 System 消息: "API error: 429 rate limited" |
| 流解析错误 | System 消息: "Failed to parse agent response"，显示原始文本 |
| LLM 返回无效 JSON diffs | 仅显示 reply 文本（不含 diff 按钮）|
| `apply_scene_diff` 失败 | System 消息: "Failed to apply diff: ..."，diff 状态变为 `Failed { reason }` |
| Tool 执行 panic | 捕获为 System 消息，tool call 循环继续 |

不 crash。不 panic。所有错误通过 `EditorError` 通道报告。

---

## 9. 测试策略

| 层级 | 测试 | 文件 |
|------|------|------|
| 单元 | AgentClient 流解析 (mock reqwest) | `tests/e3_client.rs` |
| 单元 | ContextBuilder 输出格式 | `tests/e3_context.rs` |
| 单元 | ToolRegistry 本地执行 | `tests/e3_tools.rs` |
| 集成 | AgentPanel 渲染烟雾测试 (egui_kittest) | `tests/e3_panels_kittest.rs` |
| 集成 | 端到端: 模拟 LLM → diff → accept 流程 | `tests/e3_agent_flow.rs` |

mock LLM: 在测试中使用简单的固定响应，不进行真实 HTTP 调用。

---

## 11. 实现注意事项

### ComponentChange 命名冲突
`craft_kernel::hot_reload::ComponentChange` (用于 `SceneDiff`) 和 `craft_kernel::evaluator::ComponentChange` (由 `evaluate_dry_run` 返回) 是同名的不同枚举。两者都 derive `Serialize/Deserialize` 但变体字段不同。ToolRegistry 实现时必须注意导入路径区分。

---

## 10. 接受标准

1. `AgentPanel` 打开时显示上下文栏（活动文件 + 选中节点）
2. 用户输入文本并按发送 → 后台线程调用 LLM API
3. 流式响应逐 token 出现在聊天中
4. 如果 LLM 返回 `diffs`，每条建议显示 `[Preview Diff] [Accept] [Reject]`
5. 点击 Accept → diff 应用到 SceneDef，场景标记为 dirty
6. 点击 Reject → 建议灰显
7. lint/dry_run/explain 工具在请求时由 LLM 调用，本地执行，结果返回给 LLM
8. 如果 `CRAFT_LLM_API_KEY` 未设置，显示有用的错误
9. `cargo clippy -- -D warnings` 清洁
10. `cargo test -p craft-editor` 全部通过
11. `apply_scene_diff` 拒绝无效 diff（节点未找到等）并回报错误
