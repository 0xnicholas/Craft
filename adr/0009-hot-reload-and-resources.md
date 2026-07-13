# ADR 0009: Hot Reload & Resource System

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: Godot's script reload and `.tres`/`.res` resource format

## Context

PRD defines hot reload as a default mode: "Editing scene.json during a run applies changes without restart." Godot's hot reload is limited to script files — the editor re-parses `.gd` files and rebinds method pointers. Resource files (`.tres`, `.res`) are managed by an import pipeline that converts source assets.

Craft's hot reload must handle structural changes (new nodes, removed nodes, changed behaviors) and component value changes, all without restarting the engine or invalidating agent subscriptions.

## Decision

**File watcher (notify crate) → parse → validate → diff → hot-patch. Stale loaded resources are NOT retroactively updated.**

### Hot Reload Pipeline

```
notify::Watcher detects scene.json change
     ↓
re-parse + re-validate
     ↓ fail → structured ParseError / ValidationError back to agent
pass ↓
compute diff: SceneTree (current) vs SceneDef (new)
     ↓
apply diff:
  - component changed  → update value (preserve transient counters)
  - component removed  → reset to default
  - new node           → spawn, insert into tree
  - node removed       → despawn subtree, cleanup signal subscriptions
  - behavior changed   → replace Behavior object
  - node type changed  → despawn + respawn (reuse NodeId)
     ↓
emit signal "hot_reload" { diff, affected_nodes }
     ↓
engine continues from current tick
```

### Key Invariants

| Invariant | Reason |
|-----------|--------|
| Node IDs are stable across reloads | Agent subscriptions survive |
| Transient component counters are preserved | Don't reset cooldowns on unrelated changes |
| Observer subscriptions survive | Agent doesn't need to re-subscribe |
| Replay recordings embed frozen scene snapshots | Replays are independent of subsequent hot reloads |

### Resource System

```rust
pub struct ResourceRegistry {
    resources: HashMap<String, ResourceEntry>,
}

pub struct ResourceEntry {
    pub data: serde_json::Value,
    pub schema_type: String,
    pub version: u32,                       // incremented on each registerResource
}

pub struct ResourceRef {
    pub path: String,                        // "res://tables/enemy_stats.json"
    pub snapshot_version: u32,               // version at load time
}
```

**"Loaded instances not retroactively updated" semantics**:

```
Tick 0:  agent registers res://enemy_stats.json { hp: 100 }
Tick 50: agent spawns enemy_1 → enemy_1 gets { hp: 100 }
Tick 60: agent re-registers res://enemy_stats.json { hp: 150 }
         → enemy_1 still has hp: 100 (already loaded)
Tick 70: agent spawns enemy_2 → enemy_2 gets { hp: 150 } (new version)
```

## Rationale

1. **Diff, don't rebuild**: Rebuilding the entire scene tree on every file change would lose all runtime state. Computing a structural diff and applying targeted patches preserves running state.

2. **Resources are immutable per-load**: Godot's live resource updates can cause surprising cascading changes. Craft's explicit "loaded instance keeps old version" semantics give the agent control over when to respawn entities with new data.

3. **Hot reload is the default authoring loop**: The agent doesn't distinguish between "editing" and "playing." Every file save is a potential hot reload. The engine must handle this seamlessly.

4. **JSON, not binary**: Resources are JSON files the agent can read and write. No import pipeline, no binary formats, no asset compression. This keeps the authoring loop simple: edit JSON → see effect.

## Godot Mapping

| Godot | Craft |
|-------|-------|
| Script reload (reparse `.gd`, rebind methods) | Hot reload (diff + hot-patch components/behaviors) |
| `.tres` (INI-like) / `.res` (binary) resources | JSON resources (agent-readable) |
| Resource changes propagate to all references immediately | Stale instances keep old version; new spawns get new version |
| Editor handles file watching + reload triggering | notify crate + `HotReloader::poll()` |
| Import pipeline (`.png` → `.ctex`, etc.) | No import pipeline (v1 ASCII only) |
| No hot reload of node structure (add/remove nodes) | Structural changes handled via diff: spawn/despawn |
| No hot reload of behavior definitions | Behavior objects replaced atomically |

## Resource Path Convention

All paths use the `res://` scheme:
- `res://games/tower_defense/scene.json` — scene file
- `res://games/tower_defense/resources/enemy_stats.json` — resource file
- `res://` prefix is resolved relative to the project root
