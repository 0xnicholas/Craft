# ADR 0019: Agent Copilot Panel

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: ADR 0017, ADR 0018. The Copilot panel is the human-agent collaboration surface within the editor.

## Context

The project's core thesis is "AI-native by construction." The editor must make agent collaboration feel like Copilot in an IDE — the agent has access to the full engine API, sees what the human is looking at, and proposes changes that the human can accept, modify, or reject.

This is not a separate tool. It's a panel inside the editor, sharing the same scene data, file system, and engine instance.

## Decision

**A sidebar panel where the agent observes editor context, proposes changes as structured diffs, and the human reviews with accept/reject/modify controls.**

### Agent Panel UI

```
┌─────────────────────────────────┐
│ Agent Copilot                   │
├─────────────────────────────────┤
│ Context:                        │
│  Viewing: scene.json            │
│  Selected: node=player          │
│  Last action: 2s ago            │
├─────────────────────────────────┤
│ ┌─────────────────────────────┐ │
│ │ Agent: I see you have a     │ │
│ │ Tower node with no behavior.│ │
│ │ I can add a shoot-on-tick.  │ │
│ │                             │ │
│ │ [Preview Diff] [Apply] [✕] │ │
│ └─────────────────────────────┘ │
│ ┌─────────────────────────────┐ │
│ │ You: Add 3 enemies along    │ │
│ │ the top path.               │ │
│ └─────────────────────────────┘ │
│ ┌─────────────────────────────┐ │
│ │ Agent: Generated 3 Enemy    │ │
│ │ nodes at positions [0,0],   │ │
│ │ [5,0], [10,0]. I also added│ │
│ │ an on_tick rule to move     │ │
│ │ them right.                 │ │
│ │                             │ │
│ │ [Preview Diff] [Apply] [✕] │ │
│ └─────────────────────────────┘ │
├─────────────────────────────────┤
│ [Type a message...]        [→]  │
└─────────────────────────────────┘
```

### State

```rust
// crates/craft-editor/src/panels/agent_panel.rs

pub struct AgentPanelState {
    pub messages: Vec<AgentMessage>,
    pub input: String,
    pub pending_suggestions: Vec<AgentSuggestion>,
    pub is_processing: bool,
}

pub enum AgentMessage {
    User { text: String },
    Agent { text: String, suggestions: Vec<AgentSuggestion> },
    System { text: String },  // hot reload notifications, errors, etc.
}

pub struct AgentSuggestion {
    pub id: String,
    pub description: String,           // "Add 3 Enemy nodes (hp=50)"
    pub diff: SceneDiff,               // structural diff to apply
    pub status: SuggestionStatus,      // Pending | Accepted | Rejected | Modified
}

pub enum SuggestionStatus {
    Pending,
    Accepted,
    Rejected,
    Modified { original: Box<SceneDiff>, modified: Box<SceneDiff> },
}
```

### Context Injection

Before each agent request, the editor injects the current context:

```rust
impl AgentPanelState {
    fn build_context(&self, state: &EditorState) -> AgentContext {
        AgentContext {
            active_file: state.scene_path.clone(),
            selected_node: state.scene_tree.selected_node,
            selected_node_type: state.selected_node_type(),
            visible_components: state.selected_node_components(),
            recent_changes: state.recent_edits(5),    // last 5 edits
            engine_schema: state.engine.get_schema(), // full API surface
        }
    }
}
```

The agent receives this context with every message, so it always knows what the human is looking at.

### Diff Review Flow

When the agent proposes changes, they appear as a side-by-side diff:

```
┌──────────────────────────────────────────────┐
│ Agent Suggestion: Add shoot behavior to tower│
├──────────────────┬───────────────────────────┤
│ Current          │ Proposed                  │
├──────────────────┼───────────────────────────┤
│ "behaviors": []  │ "behaviors": [            │
│                  │   {                       │
│                  │     "kind": "on_tick",    │
│                  │     "actions": [          │
│                  │       {                   │
│                  │         "kind": "emit",   │
│                  │         "signal": "shoot" │
│                  │       }                   │
│                  │     ]                     │
│                  │   }                       │
│                  │ ]                         │
├──────────────────┴───────────────────────────┤
│ [Accept] [Modify] [Reject]                   │
└──────────────────────────────────────────────┘
```

Accepted diffs are applied to the in-memory `SceneDef`, the editor marks the file dirty, and the change is visible immediately in all panels (scene tree, inspector, behavior editor).

### Agent Capabilities

The agent can:
- **Read** any file, any node, any component (via engine API)
- **Propose** scene modifications (as diffs)
- **Explain** nodes and behaviors (via `engine.explain()`)
- **Dry-run** actions to preview effects
- **Search** LuaRocks for packages

The agent cannot:
- **Write** to disk directly (diffs must be approved)
- **Run** the engine (human presses F5)
- **Access** the file system outside the project directory

### Technical Flow

```
Human types message or clicks "Ask Agent"
     │
     ▼
Editor builds AgentContext {
    active_file, selected_node, visible_components,
    recent_changes, full engine schema
}
     │
     ▼
Context + message + conversation history → LLM API call
     │
     ▼
LLM responds with:
  1. Natural language reply ("I added 3 enemies...")
  2. Structured action proposals: [{ type: "modify_scene", diff: {...} }]
     │
     ▼
Editor parses response:
  - Reply text → displayed in chat bubble
  - Diffs → stored as AgentSuggestion { status: Pending }
     │
     ▼
Human reviews each suggestion:
  - [Preview Diff] → opens side-by-side diff viewer
  - [Accept] → applies to SceneDef, marks dirty
  - [Modify] → opens diff in behavior editor for manual tweaks
  - [Reject] → dismissed
```

### Agent Tool Use (LLM-Side)

The agent (LLM) is given the engine's API as tools:

```json
{
  "tools": [
    // Engine AI-native primitives (ADR 0014)
    {
      "name": "lint",
      "description": "Static analysis of a scene — unreachable states, missing subscribers, broken paths",
      "parameters": { "scene": "Scene" }
    },
    {
      "name": "dry_run",
      "description": "Simulate actions without side effects",
      "parameters": { "node_id": "string", "actions": "Action[]" }
    },
    {
      "name": "explain",
      "description": "Get structured explanation of a node",
      "parameters": { "node_id": "string" }
    },
    {
      "name": "diff",
      "description": "Compare two ticks in a recording",
      "parameters": { "tick_a": "u32", "tick_b": "u32" }
    },
    // Extended scene-authoring tools (editor-specific)
    {
      "name": "read_scene",
      "description": "Read the current scene.json content",
      "parameters": {}
    },
    {
      "name": "read_node",
      "description": "Get a specific node's full state",
      "parameters": { "node_id": "string" }
    },
    {
      "name": "propose_diff",
      "description": "Propose modifications to the scene",
      "parameters": { "description": "string", "diff": "SceneDiff" }
    },
    {
      "name": "search_luarocks",
      "description": "Search LuaRocks for packages",
      "parameters": { "query": "string" }
    }
  ]
}
```

These tools extend the AI-native primitives (ADR 0014: `lint`, `dryRun`, `explain`, `diff`) with additional scene-authoring tools (`read_scene`, `read_node`, `propose_diff`, `search_luarocks`) tailored for the Copilot workflow. The first four are engine primitives; the last four are editor-specific wrappers that present the engine's data in an agent-friendly format.

## Rationale

1. **Copilot model, not chatbot**: The agent doesn't just chat — it produces actionable, reviewable diffs. The human's primary interaction is accept/reject, not copy-paste from a chat window.

2. **Same context the human sees**: The agent knows which file is open, which node is selected, and what changes were just made. This makes suggestions contextual — not generic.

3. **Diff, not direct write**: The agent proposes; the human approves. This is the right safety boundary for an AI tool that modifies game logic.

4. **Editor as the shared workspace**: Both human and agent operate on the same `SceneDef`, visible in the same panels. There's no "agent view" vs "human view" — the scene tree is the source of truth for both.

## Godot Mapping

Godot has no equivalent to this panel. This is Craft's unique differentiator.
