# Craft v3.0-GPU: 2D Sprite/GPU Rendering — Design

**Date**: 2025-07-17
**Status**: Draft
**Supersedes**: ADR 0004 §Extension Seam — adds GPU backend without changing the Render trait

## Summary

Add 2D GPU-accelerated sprite rendering to Craft via a new `craft-gpu` crate (wgpu + winit). The existing `Render` trait in `craft-kernel` remains unchanged (5 methods, including `capabilities()`). All GPU-specific logic lives in `craft-gpu`. Sprites are represented as optional components on existing nodes — no new node types.

## Motivation

Craft v1/v2 renders exclusively to ANSI terminals. For games with visual assets (sprites, textures, backgrounds), GPU rendering is essential. The architecture must:

1. Add GPU capability without modifying the kernel's `Render` trait
2. Fit into the property-bag component model (ADR 0002)
3. Support the existing editor workflow (F5 to run)
4. Remain headless-testable (CI, replay)

## Architecture

### Trait Boundaries

```
craft-kernel (unchanged):       craft-gpu (new):
+----------------------+        +-------------------------+
| Render trait         |        | GpuRenderer             |
|   render()           |<-------|   impl Render            |
|   viewport()         |        | (concrete struct)        |
|   resize()           |        |   begin_frame()          |
|   shutdown()         |        |   end_frame()            |
|   capabilities()     |        |   draw_sprite()          |
|                      |        |   set_camera()           |
| ComponentView        |        |   load_texture()         |
| ComponentValue       |        |   unload_texture()       |
+----------------------+        | spawn_game_window()      |
                                +-------------------------+
```

**Render trait** — zero changes. All 5 methods + `capabilities()` (already has `SPRITE` bit defined in `RenderCapabilities`).

**GpuRenderer** — concrete struct implementing `Render`. Its `render()` method internally calls `begin_frame()` → extract sprites from `ComponentView` → `draw_sprite()` per sprite → `end_frame()`. No separate trait — a trait boundary for GPU operations is YAGNI until a second GPU backend exists. All GPU methods are private to `GpuRenderer`.

### Crate Dependencies

```
craft-terminal ---> craft-kernel    (impl Render)
craft-gpu      ---> craft-kernel    (impl Render)
craft-gpu      ---> wgpu, winit, image
craft-editor   ---> craft-kernel + craft-gpu
craft-replay   ---> craft-kernel    (NullRenderer, no GPU dep)
```

The kernel never depends on or references GPU concepts.

## 2D Data Model

### New ComponentValue Variants

Two new variants added to `ComponentValue`:

| Variant | JSON | Rust | Example |
|---------|------|------|---------|
| `Vec3` | `[r, g, b]` | `[f64; 3]` | `[1.0, 0.3, 0.3]` — color tint |
| `Rect` | `[x, y, w, h]` | `[f64; 4]` | `[0, 64, 32, 32]` — sprite source rect |

### Parser Strategy

The existing `json_to_component_value` parser is type-unaware (arrays only accept length 2 for Vec2). Extend with array-length heuristics:

| Array length | Parsed as |
|-------------|-----------|
| 2 floats | `Vec2` (existing behavior) |
| 3 floats | `Vec3` |
| 4 floats | `Rect` |
| other | validation error |

**Texture paths** use the existing `String` variant — no new variant needed. A component of type `String` with key `sprite` is interpreted as a texture path by the renderer. This avoids introducing a separate variant that would be unreachable via the current parser's object-dispatch path.

**ComponentType enum**: The `ComponentType` enum in `scene.rs` gets two new variants: `Vec3` and `Rect`. This is needed for `component_value_matches()` validation and schema generation (`ComponentType::Vec3` → JSON Schema `{"type": "array", "minItems": 3, "maxItems": 3, "items": {"type": "number"}}`, `ComponentType::Rect` → same with minItems/maxItems 4). Exhaustive match sites that need updating: `component_value_matches` (scene.rs), `component_type_name` (lint.rs), `component_to_json_value` (lint.rs), `component_value_to_json` (evaluator.rs). Universal components use these types in the `$defs/universal-components` schema without being declared in any node's `component_specs()` — the validator accepts them via the allowlist regardless.

### Universal Components (Validation)

Rendering components bypass per-node-type schema validation. The validator (`validate_node` in `scene.rs`) maintains a **universal component allowlist** that all node types accept:

```
position, sprite, sprite_rect, modulate, alpha, scale, rotation, z_index, visible
```

These keys never trigger `"unknown component"` errors, regardless of the node's `component_specs()`. This preserves ADR 0008's strict validation for gameplay components while letting any node be renderable. The universal list is defined once in kernel and shared by validation and the renderer.

**Schema generation**: Universal components are emitted as a `"$defs/universal-components"` definition in the generated JSON Schema. Every node type's schema references this definition to add them as optional properties alongside its declared components.

### Standard Rendering Components

Any node gets rendering by adding these optional components. None are required — a node without `sprite` is invisible to the GPU renderer.

| Component | Type | Default | Purpose |
|-----------|------|---------|---------|
| `sprite` | `String` | — | Texture asset path (presence = "this is a sprite") |
| `sprite_rect` | `Rect` | full texture | Source rectangle for sprite sheets |
| `modulate` | `Vec3` | `[1, 1, 1]` | RGB color tint (multiplicative) |
| `alpha` | `Float` | `1.0` | Opacity (<=0 = skipped) |
| `scale` | `Vec2` | `[1, 1]` | Non-uniform scale |
| `rotation` | `Float` | `0.0` | Radians |
| `z_index` | `Int` | `0` | Draw order (higher = on top) |
| `visible` | `Bool` | `true` | Toggle without removing components |

### Camera2D Node

Identified by `type: "Camera2D"`. Only one active camera per scene (first found wins).

| Component | Type | Default | Purpose |
|-----------|------|---------|---------|
| `position` | `Vec2` | `[0, 0]` | Camera center in world space. Used when `follow` target is absent. |
| `zoom` | `Float` | `1.0` | Zoom level (>0). <1 = zoom out, >1 = zoom in |
| `follow` | `String` | — | Node ID to track. Overrides position each tick. If the followed node doesn't exist or is destroyed, falls back to camera's own `position`. |

### Example Scene JSON

```json
{
  "kind": "scene",
  "name": "level_1",
  "nodes": [
    {
      "id": "main_camera",
      "type": "Camera2D",
      "components": {
        "position": [0, 0],
        "zoom": 1.0,
        "follow": "player"
      }
    },
    {
      "id": "player",
      "type": "Player",
      "components": {
        "position": [100, 200],
        "sprite": "sprites/player.png",
        "z_index": 10,
        "health": 100
      },
      "lua_class": "scripts.player"
    },
    {
      "id": "enemy_1",
      "type": "Enemy",
      "components": {
        "position": [300, 200],
        "sprite": "sprites/enemies.png",
        "sprite_rect": [0, 64, 32, 32],
        "modulate": [1.0, 0.3, 0.3],
        "z_index": 5,
        "health": 75
      }
    }
  ]
}
```

## Render Pipeline

### Per-Frame Flow

```
render(&[ComponentView], tick)
  1. Extract Camera2D from components -> build view-projection matrix
  2. For each ComponentView with "sprite":
     - Skip if !visible or alpha <= 0
     - SpriteDrawCommand::from_component_view(view)
  3. Sort commands by z_index
  4. Group by texture_id -> batches
  5. For each batch:
     - Bind texture
     - Draw all sprites in batch (instanced quads)
  6. Submit command buffer, present surface
```

### SpriteDrawCommand

```rust
pub struct SpriteDrawCommand {
    pub texture_id: String,         // key into TextureCache
    pub src_rect: Option<[f32; 4]>, // [x, y, w, h] in texels, None = full
    pub position: [f32; 2],         // world-space
    pub scale: [f32; 2],            // default [1, 1]
    pub rotation: f32,              // radians
    pub modulate: [f32; 4],         // RGBA (alpha baked in from alpha component)
    pub z_index: i32,
}
```

Extracted from `ComponentView` via `SpriteDrawCommand::from_component_view(view) -> Option<Self>`. Returns `None` if no `sprite` component or node is invisible.

### Batching Strategy

- Single static vertex buffer: one unit quad (4 vertices, 6 indices) — created once at startup
- Per-sprite transform: instance buffer (position + scale + rotation + src_rect as 4x4 matrix)
- One bind group per texture atlas -> one draw call per batch
- Expected <=10 batches for tower_defense scene
- Missing textures -> 2x2 magenta placeholder quad generated programmatically

### Camera Transform

4x4 view-projection matrix computed from `Camera2D` node, passed as uniform buffer:

```rust
// view: translate(-camera_pos) * scale(zoom)
// proj: orthographic projection from world coords -> NDC
let view_proj = ortho_matrix * scale_matrix(zoom) * translate_matrix(-camera_pos);
```

## Asset Pipeline

### TextureCache

Internal to `GpuRenderer`. Lives for the lifetime of the renderer.

```rust
struct TextureCache {
    textures: HashMap<String, GpuTexture>,  // path -> GPU resource
    missing: HashSet<String>,                // negative cache
}

struct GpuTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    size: [u32; 2],
}
```

**Loading**: lazy on first `draw_sprite` reference. Loaded textures stay cached until `shutdown()`.

**Missing textures**: logged once, magenta 2x2 placeholder served. Never crashes.

**Formats**: PNG required. JPEG/BMP/GIF/TIFF/WebP supported via `image` crate.

**Asset root**: `games/<name>/assets/`. Texture paths in JSON are relative to this root.

**Hot reload**: deferred to v3.1. v3.0 loads each texture lazily on first use and caches it for the renderer lifetime — but never reloads when source files change. No file watcher attached to `assets/`.

### Atlas Convention

No `.atlas` metadata file. Each node specifies its own `sprite_rect`. External tooling can generate nodes with correct rects. This avoids a file format decision and keeps the scene file self-contained.

## Game Loop & Editor Integration

### Standalone Game Window

```rust
// craft-gpu public API
pub fn spawn_game_window(scene_path: &Path, config: GameWindowConfig) -> EngineResult<()>
```

Creates winit window + engine + GpuRenderer, runs the game loop. Blocking call. Intended to be spawned in a dedicated thread by the editor.

**Fixed timestep loop**:
```
event_loop.run:
  AboutToWait: if elapsed >= tick_duration -> window.request_redraw()
  RedrawRequested: engine.tick() -> engine.render_now()
  CloseRequested: shutdown -> exit
```

Tick rate: `config.tick_hz` (default 60Hz). Rendering is vsync-locked by wgpu.

### Editor Integration

| Action | Behavior |
|--------|----------|
| **F5 (Run)** | Save current scene -> `std::thread::spawn(|| spawn_game_window(...))`. On macOS, winit event loops require the main thread — the editor will use `std::process::Command` to launch a child process instead. |
| **F8 (Stop)** | Send close event to game window, thread joins |
| **Terminal Preview** | Unchanged — still `AnsiRenderer` |
| **Headless** | `NullRenderer` — no winit dependency |

### Three Render Modes

| Mode | Renderer | Window | Used by |
|------|----------|--------|---------|
| `terminal` | AnsiRenderer | — | Editor preview panel, CLI |
| `gpu` | GpuRenderer | winit | Editor F5, standalone builds |
| `headless` | NullRenderer | — | Replay, CI, benchmarks |

## Testing Strategy

Following ADR 0010's four-layer pyramid:

### 1. Unit Tests
- `SpriteDrawCommand::from_component_view()` — extraction from various component configurations
- `TextureCache` — cache hit, cache miss, negative cache, placeholder
- Camera matrix math — translation, zoom, NDC conversion
- New `ComponentValue` variants — serialize/deserialize round-trip
- Schema generation includes new types

All testable without GPU hardware.

### 2. Integration Tests
- Scene with sprite components -> engine.tick() -> GpuRenderer.render() produces correct draw commands
- Camera follow: change player position -> camera tracks
- Missing texture -> magenta fallback, no crash
- Multiple nodes with same atlas -> single batch
- z_index ordering is correct

Use wgpu software adapter for CI. Assert on `SpriteDrawCommand` sequences (collect commands in test mode).

### 3. Replay Regression
- Sprites are just components on nodes. Existing state hash covers them.
- Replay with NullRenderer should produce identical hashes.
- No new replay tests required — existing suite validates determinism.

### 4. Reference Game Integration
- Tower defense scene with sprite assets
- 100+ ticks at 60Hz with GPU rendering
- Tick budget <=8ms maintained (ADR 0015)
- CI runs headless with software adapter; manual testing on real GPU for perf validation

## Milestones

```
G1 -> G2 -> G3 -> G4
```

| # | Deliverable | Crate(s) | Tests | Acceptance |
|---|------------|----------|-------|------------|
| **G1** | Component types + SpriteDrawCommand | `craft-kernel` | Unit tests for new types + extraction | Vec3, Rect in ComponentValue. Schema updated. Scene JSON parses sprite components via String + array-length heuristics. `SpriteDrawCommand::from_component_view` extracts correctly for all valid/invalid component combos. |
| **G2** | craft-gpu crate + wgpu window | `craft-gpu` (new) | Window opens with clear color | winit window creates. Clear color renders. GpuRenderer impl Render — `render()` clears to background. Null texture shows magenta placeholder. `shutdown()` cleans up GPU resources. |
| **G3** | Sprite rendering + camera | `craft-gpu` | Integration: sprites at positions, camera, atlases | Textures load from disk and cache. Sprites draw at correct world positions. modulate/rotation/scale/z_index all work. Camera2D pan and zoom. Atlas source rects crop correctly. |
| **G4** | Editor F5 + game loop + tower defense | `craft-editor` + `craft-gpu` | Reference game: tower defense with sprites | Editor F5 opens GPU window with current scene. Tower defense runs 100+ ticks at 60Hz. Tick budget <=8ms. All existing 346 tests pass. Clippy + fmt clean. |

## Acceptance Criteria

- [ ] `cargo test` passes all 4 test layers (existing + new GPU tests)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] Scene JSON with `sprite`, `sprite_rect`, `modulate`, `z_index`, `scale`, `rotation`, `alpha`, `visible` components parses and validates
- [ ] `Camera2D` node type controls view (position + zoom + follow components)
- [ ] Editor F5 opens winit window showing GPU-rendered scene
- [ ] Tower defense renders at 60Hz with <=8ms tick budget (ADR 0015)
- [ ] Replay regression passes (NullRenderer — no GPU needed)
- [ ] `cargo test` passes in CI (headless wgpu software adapter)
- [ ] Missing textures show magenta fallback, logged once, never crash
- [ ] No new `unwrap()` or `expect()` in production code

## Lua Bindings

Lua scripts can read/write all rendering components via the standard field syntax (`node.modulate`, `node.alpha`, `node.z_index`, etc.). The new `ComponentValue` variants are exposed through mlua as Lua sequences (arrays): `Vec3` → `{r, g, b}` table with 3 elements, `Rect` → `{x, y, w, h}` table with 4 elements. Texture paths use `String` — Lua sets `node.sprite = "sprites/player.png"`. Two functions in `craft-lua/src/runtime.rs` need new match arms: `lua_to_component_value` (add length-3 → Vec3, length-4 → Rect table handling) and `component_value_to_lua` (add Vec3 → 3-element table, Rect → 4-element table serialization).

## Deferred

| Feature | Target | Note |
|---------|--------|------|
| Particle systems | v3.1 | GPU compute particles |
| Tilemaps | v3.1 | Grid-based batched rendering |
| Custom shaders (WGSL) | v3.2 | Shader injection via component |
| Hot-reload textures | v3.1 | File watcher + cache invalidation |
| Physics (collision/rigid body) | v3.1 | Separate design doc |
| Audio | v3.2 | Separate design doc |
| 3D mesh rendering | v3+ | After 2D stable |
| Cross-patch replay | v3+ | ADR 0006 |

## Related Documents

- `docs/adr/0004-render-trait.md` — Render trait design, extension seam
- `docs/adr/0002-node-model.md` — Property-bag Node model
- `docs/adr/0015-performance-budgets.md` — Tick budget <=8ms
- `docs/adr/0010-testing-strategy.md` — 4-layer test pyramid
- `docs/adr/0017-editor-architecture.md` — Editor (egui)
- `ROADMAP.md` — "Beyond v2" section
