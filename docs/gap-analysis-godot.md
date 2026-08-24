# Craft vs Godot — Engine Feature Gap Analysis

A single-file, facts-only inventory of the gap between **Craft** and **Godot 4.x** across the engine feature surface, organized by Godot feature domain. Produced by the wayfinding effort [“Craft vs Godot engine-feature gap analysis”](https://github.com/0xnicholas/Craft/issues/10); assembled from 15 per-domain research tickets (#11–#25).

**This report contains no chase/not-chase decisions.** It is a fact base for future roadmap discussions: every item is graded and cited; nothing is recommended.

## Baselines and method

- **Godot baseline: 4.7.2-stable** (tag `4.7.2-stable`, published 2026-08-18, verified via the GitHub Releases API). Documentation citations are `docs.godotengine.org/en/stable` pages (the 4.7 branch); source paths are cited at tag `4.7.2-stable` (or ref `4.7` where noted). Version-pin drift across the source research is normalized here: ticket #11 pinned 4.7.0 (June 2026) and ticket #16 pinned 4.7.1 (2026-07-14) — all three point releases share the 4.7 docs branch, so no grade is affected; the drift is flagged on those tickets.
- **Craft baseline: `main` @ `245ce9d`** (post-v0.2.0: GPU sprite renderer, editor milestones E1–E4, physics/animation/particles/audio systems shipped). Craft-side claims were verified 2026-08-24 in per-domain research worktrees against this commit, with file:line citations. In-flight uncommitted work on `main` (a GPU bitmap-font text pass) postdates the UI chapter's grades — noted there.
- `_references/godot-master/` was absent from the local checkout during research; Godot-side facts come from official docs and the GitHub source tree (Contents API), cited per claim.
- Evidence standard: documentation + source verification. No hands-on Godot benchmarking.

## Grading legend

Forward chapters (1–14) grade each Godot feature against Craft on a uniform scale:

| Grade | Meaning |
|---|---|
| **missing** | No Craft equivalent. Rows annotated **(deferred)** cite a documented ROADMAP/PRD deferral. |
| **partial** | Craft has a subset or narrowed equivalent. |
| **parity** | Equivalent capability. |
| **divergence** | Deliberate difference, cited to an ADR — not a gap to close. |

Some rows carry a divergence *annotation* alongside a missing/partial primary grade (e.g. “missing — structural divergence: ADR 0002”): the primary grade records implementation state, the annotation records a deliberate architectural bound.

Chapter 15 (reverse gaps) inverts the direction — Craft capabilities measured against Godot — with its own grades: **full gap** (no Godot equivalent), **partial equivalent** (Godot has a scoped-down analog), **designed-but-unimplemented** (Craft-side design exists, the binding is not built; the Godot side may still be a full gap).

## Summary matrix

Per-domain row counts from each chapter's graded table (rows, not weighted by importance):

| # | Domain | missing | partial | parity | divergence | rows |
|---|--------|---------|---------|--------|------------|------|
| 1 | 2D rendering | 5 | 6 | 0 | 0 | 11 |
| 2 | 3D rendering + XR | 9 | 0 | 0 | 3 | 12 |
| 3 | Shaders, materials, particles/VFX | 11 | 6 | 0 | 1 | 18 |
| 4 | Physics 2D/3D | 7 | 7 | 0 | 4 | 18 |
| 5 | Animation | 6 | 5 | 0 | 2 | 13 |
| 6 | Audio | 4 | 4 | 0 | 2 | 10 |
| 7 | UI/GUI | 10 | 2 | 0 | 0 | 12 |
| 8 | Scene system + asset pipeline | 5 | 4 | 2 | 5 | 16 |
| 9 | Scripting & languages | 3 | 3 | 4 | 8 | 18 |
| 10 | Navigation & pathfinding | 5 | 0 | 0 | 1 | 6 |
| 11 | Input + platform export | 10 | 4 | 0 | 4 | 18 |
| 12 | Networking & multiplayer | 16 | 0 | 0 | 1 | 17 |
| 13 | Editor & debugging tools | 6 | 5 | 1 | 1 | 13 |
| 14 | Engine architecture | 2 | 1 | 3 | 15 | 21 |
| | **Forward totals** | **99** | **47** | **10** | **47** | **203** |
| 15 | Reverse gaps (Craft-favorable) | — | — | — | — | 8 |

## Cross-cutting observations (facts)

1. **Three whole-domain absences.** 3D rendering + XR (ch. 2), networking & multiplayer (ch. 12), and navigation (ch. 10) have no Craft counterpart at any level — all documented deferrals (ROADMAP v3+, PRD non-goals, ARCHITECTURE v1 exclusions), not accidental omissions.
2. **Architecture is divergence-heavy by design.** Chapter 14 grades 15 of 21 items as deliberate ADR-backed differences; the forward domains' missing sets are repeatedly *bounded* by those decisions — the 4-method Render trait (ADR 0004), single-threaded tick (ADR 0015), and next-tick signals (ADR 0003) appear as cited bounds in chapters 1–3, 4, 6, 10, and 12.
3. **ADR 0006 (deterministic per-tick replay) is the load-bearing invariant.** Threading, resource snapshotting, physics solver choice, particle simulation, and the absence of networking are all downstream of "per-tick hash must match" (ch. 14 §4); it is also Craft's single largest reverse gap (ch. 15).
4. **Parity is rare and concentrated** in scripting (lifecycle hooks, field syntax, hot-reload state preservation), scene/assets (text formats, structural hot reload), and editor undo/redo — plus save games, where neither engine ships a system.
5. **Craft holds 8 reverse-gap capabilities** (ch. 15): 5 with no Godot equivalent at all, 1 designed-but-unimplemented (NAPI bridge), 2 with partial Godot analogs.

## Contents

1. [2D rendering](#1-2d-rendering-sprites-tilemaps-lighting-post-processing) — ticket #11
2. [3D rendering + XR](#2-3d-rendering--xr) — ticket #12
3. [Shaders, materials, particles/VFX](#3-shaders-materials-visual-shader-editor-gpu-particles) — ticket #13
4. [Physics 2D/3D](#4-physics-2d3d-collision-rigid-bodies-joints) — ticket #14
5. [Animation](#5-animation-frame-skeletal-animation-trees-tweens) — ticket #15
6. [Audio](#6-audio-playback-mixing-buses-spatial-audio) — ticket #16
7. [UI/GUI](#7-uigui-controls-themes-layout-containers) — ticket #17
8. [Scene system + asset pipeline](#8-scene-system--asset-pipeline-instancing-inheritance-import) — ticket #18
9. [Scripting & languages](#9-scripting--languages-gdscriptcgdextension-vs-luajson-two-tier) — ticket #19
10. [Navigation & pathfinding](#10-navigation--pathfinding) — ticket #20
11. [Input + platform export](#11-input-system--platform-export) — ticket #21
12. [Networking & multiplayer](#12-networking--multiplayer) — ticket #22
13. [Editor & debugging tools](#13-editor--debugging-tools) — ticket #23
14. [Engine architecture](#14-engine-architecture-concurrency-node-model-persistence) — ticket #24
15. [Reverse gaps: what Craft has that Godot lacks](#15-reverse-gaps-what-craft-has-that-godot-lacks) — ticket #25
## 1. 2D rendering (sprites, tilemaps, lighting, post-processing)

> Graded evidence preserved from ticket [#11 — Gap analysis: 2D rendering (sprites, tilemaps, lighting, post-processing)](https://github.com/0xnicholas/Craft/issues/11) (findings: `docs/research/gap-2d-rendering.md`, branch `research/2d-rendering`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Craft render baseline (what exists today)

- `Render` trait — 4 core methods (`render(&[ComponentView], tick)`, `viewport`,
  `resize`, `shutdown`) plus defaulted `capabilities()`; deliberately minimal per
  [ADR 0004](../adr/0004-render-trait.md). Source: `crates/craft-kernel/src/render.rs:76`.
- `RenderCapabilities` flags: `TEXT | SPRITE | SHADER | MESH`. `SHADER` and `MESH` are
  declared but no backend reports them — `GpuRenderer` reports `TEXT | SPRITE`
  (`crates/craft-gpu/src/lib.rs:401`), `AnsiRenderer` reports `TEXT`
  (`crates/craft-terminal/src/lib.rs:156`).
- Scene-side render surface is a fixed component vocabulary. `SpriteDrawCommand`
  reads only `sprite`, `sprite_rect`, `position`, `scale`, `rotation`, `modulate`
  (Vec3+alpha), `alpha`, `z_index`, `visible`
  (`crates/craft-kernel/src/render.rs:121-160`). The load-time allowlist matches:
  `UNIVERSAL_COMPONENTS` at `crates/craft-kernel/src/scene.rs:462-475`.
- `craft-gpu`: wgpu/winit sprite renderer. One render pass, fixed clear color
  (0.1/0.1/0.15), alpha blending, per-texture batches with one `draw_indexed` call
  per sprite instance, hard cap of 1024 instances (excess truncated with a log
  warning). Source: `crates/craft-gpu/src/lib.rs:232-383`. Nearest-filter
  `ClampToEdge` sampler only, RGBA8 sRGB textures, magenta placeholder for missing
  files (`crates/craft-gpu/src/texture.rs`). Fixed WGSL sprite shader compiled into
  the binary (`crates/craft-gpu/src/shaders.rs`).
- `Camera2D` node type: `position`, `zoom`, `follow` extracted
  (`crates/craft-kernel/src/render.rs:220`), but `GpuRenderer` consumes only
  position+zoom — `follow` is dropped at `crates/craft-gpu/src/lib.rs:233-241`.
- `craft-terminal`: ANSI glyph grid, glyph chosen by `type_name` string match;
  ignores camera, z_index, modulate entirely (`crates/craft-terminal/src/lib.rs`).
- `craft-animation`: tick-based frame stepping writes `sprite_rect` from a resource's
  `frames`/`fps`/`loop` fields — Craft's equivalent of sprite-sheet animation
  (`crates/craft-animation/src/lib.rs:1-100`).

### Godot 4.7 2D rendering inventory, graded

Legend: **missing** = no Craft equivalent · **partial** = exists, narrowed ·
**parity** = equivalent · **divergence** = deliberate difference (ADR cited).

#### Sprite workflows

| Godot 4.7 | Craft | Grade |
|---|---|---|
| `Sprite2D`: texture, `region_rect` atlas cropping, `flip_h`/`flip_v`, `offset`, `centered`, `hframes`/`vframes` sprite-sheet frames ([class_sprite2d](https://docs.godotengine.org/en/stable/classes/class_sprite2d.html)) | `SpriteDrawCommand` covers texture+region+transform+modulate; no flip, offset, centered, or frame-index fields (`render.rs:121`) | **partial** |
| `AnimatedSprite2D` + `SpriteFrames` (named animations, per-frame speed, signals) | `craft-animation` advances `sprite_rect` via resource frames; no named animations or signals (`crates/craft-animation/src/lib.rs`) | **partial** |
| Per-node `texture_filter` (7 modes incl. mipmaps/aniso) and `texture_repeat` ([class_canvasitem](https://docs.godotengine.org/en/stable/classes/class_canvasitem.html)) | One global sampler: Nearest + ClampToEdge (`crates/craft-gpu/src/texture.rs:100-106`) | **partial** |
| `modulate` (propagates to children) vs `self_modulate` (node only); visibility propagates down the tree | Flat modulate per node; `ComponentView` slices are hierarchy-flattened, so parent→child modulate/visibility inheritance is inexpressible (`render.rs:116-118`) | **partial** |
| `NinePatchRect`, `Polygon2D`, `Line2D`, `MeshInstance2D`, `MultiMeshInstance2D` | None | **missing** |
| Immediate-mode custom drawing (`CanvasItem._draw()`, 30+ `draw_*` primitives: lines, polygons, text, meshes) | None; renderer only consumes `SpriteDrawCommand`s | **missing** |

#### Tilemaps and terrains

Godot's `TileMapLayer` (the `TileMap` node is deprecated since 4.3) provides: shared
`TileSet` resources with atlas and scene-collection sources; terrain sets with
Connect/Path autotiling; alternative tiles (flip/rotate/transpose); saved patterns;
scattered painting; per-tile physics, occlusion, navigation, and custom data layers;
y-sorted tiles and quadrant draw batching.
Sources: [using_tilemaps](https://docs.godotengine.org/en/stable/tutorials/2d/using_tilemaps.html),
[class_tilemaplayer](https://docs.godotengine.org/en/stable/classes/class_tilemaplayer.html),
GitHub `scene/2d/tile_map_layer.cpp`.

Craft: nothing. No tile/grid component vocabulary, no TileSet resource type, no
autotiling. `games/platformer/assets/levels/level1.json` exists but has no renderer
counterpart. Grade: **missing**.

#### 2D lighting and shadows

Godot ships a full real-time 2D lighting pipeline: `PointLight2D` and
`DirectionalLight2D` (texture-shaped, energy, height for normal mapping,
Add/Subtract/Mix blend modes, z/layer ranges, cull masks), `LightOccluder2D` shadow
casters feeding a real-time signed distance field usable from custom shaders,
PCF5/PCF13 soft-shadow filters, `CanvasModulate` ambient color, and normal+specular
maps on any 2D element via `CanvasTexture`.
Source: [2d_lights_and_shadows](https://docs.godotengine.org/en/stable/tutorials/2d/2d_lights_and_shadows.html),
GitHub `scene/2d/light_2d.cpp`, `scene/2d/light_occluder_2d.cpp`.

Craft: no lighting pass of any kind. The single sprite pass renders directly to the
surface (`craft-gpu/src/lib.rs:348-380`); there are no render targets, no second
pass, no shadow/SDF machinery, and no light/occluder components in the allowlist.
Grade: **missing**.

#### Canvas layers and draw ordering

Godot: `CanvasLayer` nodes create independent render layers with their own transform
(UI pinned to screen, parallax backgrounds via layer -1 / `Parallax2D`); per-item
`z_index` (relative or global), y-sorting, `visibility_layer` bitmasks culled per
Viewport, `top_level` transform escape, `clip_children` masking, `CanvasGroup`
(composite children then apply effects as one).
Sources: [canvas_layers](https://docs.godotengine.org/en/stable/tutorials/2d/canvas_layers.html),
[class_canvasitem](https://docs.godotengine.org/en/stable/classes/class_canvasitem.html),
[class_canvaslayer](https://docs.godotengine.org/en/stable/classes/class_canvaslayer.html).

Craft: a single global `z_index` integer sort (`craft-gpu/src/lib.rs:252`). No layer
concept, no per-layer transforms (no way to keep a HUD fixed while the camera moves),
no y-sort, no visibility masks, no clipping, no compositing. Grade: **partial**
(z_index only).

#### Post-processing and shaders

Godot's 2D shader story: `canvas_item` shaders per node via `CanvasItem.material`
with per-instance shader uniforms (added in 4.4), the `hint_screen_texture` +
`SCREEN_UV` screen-reading mechanism with automatic back-buffer copies, and explicit
`BackBufferCopy` nodes (whole screen or region) — together these enable full 2D
post-processing (blur, color grading, distortion) without leaving the scene tree.
Sources: [screen-reading_shaders](https://docs.godotengine.org/en/stable/tutorials/shaders/screen-reading_shaders.html),
[canvas_item_shader](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/canvas_item_shader.html),
[Godot 4.4 release notes](https://godotengine.org/releases/) (2D shader instance uniforms, 2D batching on all backends).

Craft: one hard-coded WGSL shader pair compiled into `craft-gpu`
(`crates/craft-gpu/src/shaders.rs:3-58`). No user shaders, no render-to-texture, no
back-buffer access, no multi-pass. The `SHADER` capability bit exists as the planned
seam but is reported by no backend. Grade: **missing** (seam reserved;
**divergence** that the trait stays minimal is deliberate — ADR 0004).

#### Camera2D

Godot `Camera2D`: position smoothing, drag margins, limit left/top/right/bottom,
rotation, anchor modes, per-camera offset. Craft: `position`, `zoom`, plus a
`follow` field that is extracted but never consumed by `GpuRenderer`
(`render.rs:220-233`, `craft-gpu/src/lib.rs:51-59`). Grade: **partial**.

### What the current `Render` trait + component model cannot express

Structural limits, independent of any single feature:

1. **No hierarchy-aware draw state.** `views_for_scene` flattens nodes into a slice
   (`render.rs:116-118`). Godot's propagated modulate/visibility, relative z_index,
   y-sort groups, and `top_level` all rely on parent→child chains that are erased
   before the renderer sees the data.
2. **No material/shader channel.** `Render.render` receives component snapshots only;
   there is no way to name a shader, pass uniforms, or request a blend mode other
   than alpha (fixed `BlendState::ALPHA_BLENDING`, `shaders.rs:226`). Godot's
   per-CanvasItem `material` + `CanvasItemMaterial` blend modes have no counterpart.
3. **No multi-pass / render-target vocabulary.** The trait has no frame graph: one
   `render()` call, one implicit surface. Lighting, shadows, screen-reading shaders,
   and `BackBufferCopy` all require intermediate textures the interface cannot name.
4. **A closed render vocabulary.** `SpriteDrawCommand` is a fixed struct and the
   scene validator rejects unknown component keys outside `UNIVERSAL_COMPONENTS`
   (`scene.rs:462-475`, `505-510`). New Godot-style properties (flip, offset, light
   masks, tile layers) require extending both the allowlist and the struct.
5. **No layer/mask addressing.** A single global z sort cannot express canvas layers,
   visibility_layer bitmasks, or light cull masks — there is no second axis of
   draw-order or visibility selection.
6. **Fixed batch cap.** 1024 instances per frame with truncation
   (`craft-gpu/src/lib.rs:287-293`); Godot's 2D batching (all backends since 4.4)
   scales to scene size.

Deliberate divergences (not gaps): the 4-method trait itself and "no GPU abstraction
in v1" (ADR 0004); no CanvasItem-style inheritance, property-bag components instead
(ADR 0002); single-threaded render on the tick path (ADR 0015); GPU window as an
editor subprocess (ADR 0020).

### Sources

Godot (docs track 4.7 stable):

- <https://docs.godotengine.org/en/stable/about/release_policy.html>
- <https://docs.godotengine.org/en/stable/classes/class_sprite2d.html>
- <https://docs.godotengine.org/en/stable/classes/class_canvasitem.html>
- <https://docs.godotengine.org/en/stable/classes/class_canvaslayer.html>
- <https://docs.godotengine.org/en/stable/classes/class_tilemaplayer.html>
- <https://docs.godotengine.org/en/stable/tutorials/2d/using_tilemaps.html>
- <https://docs.godotengine.org/en/stable/tutorials/2d/2d_lights_and_shadows.html>
- <https://docs.godotengine.org/en/stable/tutorials/2d/canvas_layers.html>
- <https://docs.godotengine.org/en/stable/tutorials/shaders/screen-reading_shaders.html>
- <https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/canvas_item_shader.html>
- <https://godotengine.org/releases/> (4.4: 2D batching on all backends, 2D shader instance uniforms)
- GitHub source paths referenced: `scene/2d/sprite_2d.cpp`, `scene/2d/tile_map_layer.cpp`, `scene/2d/light_2d.cpp`, `scene/2d/light_occluder_2d.cpp` (godotengine/godot)

Craft (paths relative to repo root, branch `research/2d-rendering`):

- `crates/craft-kernel/src/render.rs` (Render trait, ComponentView, SpriteDrawCommand, CameraInfo)
- `crates/craft-kernel/src/scene.rs:462-475` (UNIVERSAL_COMPONENTS allowlist)
- `crates/craft-gpu/src/lib.rs`, `crates/craft-gpu/src/texture.rs`, `crates/craft-gpu/src/shaders.rs`
- `crates/craft-terminal/src/lib.rs`
- `crates/craft-animation/src/lib.rs`
- `docs/adr/0004-render-trait.md`, `docs/adr/0002-node-model.md`, `docs/adr/0015-performance-budgets.md`, `docs/adr/0020-gpu-subprocess-boundary.md`
- `docs/CHANGELOG.md` (v0.2.0 GPU rendering), `ROADMAP.md` (GPU rendering v3.0 row)

## 2. 3D rendering + XR

> Graded evidence preserved from ticket [#12 — Gap analysis: 3D rendering + XR](https://github.com/0xnicholas/Craft/issues/12) (findings: `docs/research/gap-3d-rendering.md`, branch `research/3d-rendering`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Craft-side state (verified in this worktree)

- `Render` trait is 4 methods + defaulted `capabilities()` (`crates/craft-kernel/src/render.rs:76-85`). Input is a flat `&[ComponentView]` of typed component snapshots; positions are `Option<(f64, f64)>` — 2D-only (`render.rs:36-61`).
- `RenderCapabilities` declares `MESH = 1 << 3` but nothing in the repo produces or consumes it (`render.rs:20-34`; only other `MESH`/`3D` hits in `crates/` are `wgpu::Extent3d` texture descriptors and a Lua test module named `lib.math3d`).
- Only GPU backend is `GpuRenderer` (`crates/craft-gpu/src/lib.rs`): single instanced 2D sprite pipeline, orthographic 2D camera (`Camera2D` extraction, `render.rs:213-234`), no depth attachment, no lighting, one WGSL sprite shader pair (`craft-gpu/src/shaders.rs`). Capabilities: `TEXT | SPRITE` (`lib.rs:401-403`).
- No 3D node types, transforms, cameras, lights, materials, meshes, sky, or XR anywhere in the scene/component model; no XR/OpenXR dependency in any crate.
- Deliberate divergences already on record: minimal trait vs Godot's `RenderingServer` (ADR 0004, `docs/adr/0004-render-trait.md`), single-threaded engine, no render thread (ADR 0015), GPU renderer isolated as a subprocess (ADR 0020).
- 3D mesh rendering is explicitly deferred to "v3+" with the note `RenderCapabilities::MESH` (`ROADMAP.md:95`).

### Godot 4.7 inventory vs Craft

| Godot subsystem | Godot scope (4.7) | Craft | Grade |
|---|---|---|---|
| Renderer abstraction | `RenderingServer` + `RenderingDevice`; core code in `servers/rendering/renderer_rd/`, drivers in `drivers/{vulkan,d3d12,metal,gles3}` ([internal architecture](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/internal_rendering_architecture.html)) | 4-method `Render` trait; `Renderer3D` named as future sibling trait | **divergence** (ADR 0004 — explicit rejection of RenderingServer-style abstraction) |
| Rendering methods | 3 renderers: Forward+ (clustered forward, desktop), Mobile (single-pass forward, tile/sub-pass optimized), Compatibility (OpenGL ES 3.0 / WebGL 2, low-end + web) ([renderers overview](https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html)) | One wgpu 2D sprite path | **missing** |
| Rendering drivers | Vulkan, D3D12, Metal (all via RenderingDevice), OpenGL; runtime fallback chain since 4.4 ([renderers overview](https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html)) | wgpu only (which abstracts Vulkan/Metal/D3D12/WebGL itself) | **divergence** (ADR 0004 defers multi-backend; wgpu covers driver diversity for free) |
| Mesh pipeline | `Mesh`/`ArrayMesh`/`ImporterMesh`/`SurfaceTool`, `MeshInstance3D`, `MultiMeshInstance3D` instancing, `Skeleton3D` skinning, morph targets; glTF/.blend/FBX/Collada/OBJ import, Mikktspace tangents, VRAM compression (BPTC/ASTC/ETC2/S3TC/BasisU) ([feature list](https://docs.godotengine.org/en/stable/about/list_of_features.html)) | Nothing — no mesh resource type, no 3D import, no instancing beyond 2D sprite instances | **missing** |
| 3D lighting | `DirectionalLight3D`/`OmniLight3D`/`SpotLight3D`; clustered lighting (512 clustered elements/view, Forward+); 8 omni+8 spot per mesh (Mobile/Compat); shadow mapping (PSSM 2/4-split, dual-paraboloid/cube omni, PCF vogel disk, PCSS); projector textures; distance fade ([lights & shadows](https://docs.godotengine.org/en/stable/tutorials/3d/lights_and_shadows.html)) | No light concept at all; sprite `modulate` color is the only shading input | **missing** |
| Global illumination | `LightmapGI` (GPU compute baking, up to 16 bounces, SH directional, denoising), `VoxelGI` (semi-real-time probes), `SDFGI` (real-time, open worlds, cascades), `SSIL`, `ReflectionProbe` (baked/real-time, parallax correction) ([GI introduction](https://docs.godotengine.org/en/stable/tutorials/3d/global_illumination/introduction_to_global_illumination.html)) | Nothing | **missing** |
| Environment / sky / post | `Environment` + `WorldEnvironment`; `Sky` with Panorama/Procedural/Physical sky materials, real-time radiance map for IBL; tonemapping (Linear/Reinhard/Filmic/ACES/AgX), auto exposure, glow, SSR, SSAO, depth/height fog, volumetric fog + `FogVolume`, DoF, adjustments ([environment & post-processing](https://docs.godotengine.org/en/stable/tutorials/3d/environment_and_post_processing.html)) | Fixed clear color only (`lib.rs:354-359`) | **missing** |
| LOD & culling | Automatic mesh LOD on import via meshoptimizer + shadow meshes, screen-coverage LOD selection per viewport, `LOD Bias` per instance; manual HLOD via visibility ranges; CPU occlusion culling via Embree with baked `OccluderInstance3D`; frustum culling; Forward+ depth prepass ([mesh LOD](https://docs.godotengine.org/en/stable/tutorials/3d/mesh_lod.html), [occlusion culling](https://docs.godotengine.org/en/stable/tutorials/3d/occlusion_culling.html)) | Nothing — no depth buffer, no culling of any kind | **missing** |
| Materials / shaders | Disney PBR `BaseMaterial3D` (Burley/GGX, ORM, SSS, refraction, parallax, detail), visual + text shader language with 3D vertex/fragment/light/sky/fog shader types, compute shaders (Forward+/Mobile) ([feature list](https://docs.godotengine.org/en/stable/about/list_of_features.html)) | `SHADER` capability bit declared, unused; one hardcoded WGSL sprite shader | **missing** |
| Antialiasing / scaling | MSAA 2D/3D, TAA, FSR 2.2, FXAA, SSAA, 3D resolution scaling, variable-rate shading | None (nearest-sampled sprites) | **missing** |
| XR | `XRServer` central hub; `XRInterface` per platform; OpenXR in core (desktop headsets, Android standalone via plugin, action map, composition layers, hand/body tracking, render models, spatial entities, AR passthrough); limited visionOS; XR nodes `XROrigin3D`/`XRCamera3D`/`XRController3D`; stereoscopic rendering overrides; Mobile renderer recommended for XR ([XR docs](https://docs.godotengine.org/en/stable/tutorials/xr/index.html), [setting up XR](https://docs.godotengine.org/en/stable/tutorials/xr/setting_up_xr.html), [feature list](https://docs.godotengine.org/en/stable/about/list_of_features.html)) | Nothing — no XR seam in `Render`, scene model, or any crate | **missing** |
| Render threading | Dedicated render thread option; multi-threaded RenderingServer | Single-threaded by design | **divergence** (ADR 0015) |

### Bounding the gap: what `Render` would need to grow

Craft's Render surface would need, at minimum, to approach even core 3D:

1. **Component model**: `Vec3`-based 3D transform components (position exists only as 2D `(f64, f64)`), plus component schemas for mesh instance, camera (projection/fov), light, and environment. `ComponentView::from_node` and `extract_camera` are 2D-hardcoded (`render.rs:44-61`, `220-234`).
2. **Resource types**: mesh (vertex/index buffers, surfaces, materials), material/shader references, textures already exist (`craft-gpu/src/texture.rs`). Today only `sprite` string component → texture (`render.rs:131-160`).
3. **Render trait**: `MESH` capability must actually gate 3D draw data; per ADR 0004's extension seam, a sibling `Renderer3D`-style trait alongside `Render` rather than new methods inside it. Depth attachment, perspective projection, per-instance 3D transforms in the GPU backend.
4. **Subsystems with no Craft anchor at all**: lighting/shadows, GI, sky/IBL, post-processing, LOD/occlusion — each is a standalone Godot module with no corresponding hook in the Craft scene model or tick loop.
5. **XR**: no existing seam; would require stereo viewport output, tracked-pose input into the tick loop, and an OpenXR-style interface layer — all net-new.

Net: confirmed whole-domain **missing** (13 of 13 functional areas missing; 2 deliberate **divergences** at the architecture layer, both ADR-backed). The only 3D artifact in Craft is the unimplemented `RenderCapabilities::MESH` bit reserved per ROADMAP v3+.

### Claims not verified

- Godot source-tree line counts in ADR 0004 (66+ files / 180K+ lines for `servers/rendering/`) come from the ADR itself; `_references/godot-master/` is absent locally and GitHub source was not re-measured.
- 4.7-specific rendering deltas beyond the docs pages above (e.g. any new 4.7 renderer features) were not separately inventoried; `en/stable` docs were taken as the 4.7 surface.

## 3. Shaders, materials, visual shader editor, GPU particles

> Graded evidence preserved from ticket [#13 — Gap analysis: shaders, materials, visual shader editor, GPU particles](https://github.com/0xnicholas/Craft/issues/13) (findings: `docs/research/gap-shaders-vfx.md`, branch `research/shaders-vfx`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Craft inventory (verified in repo)

#### `craft-gpu` — no user-authored shader path

- One hard-coded WGSL sprite pipeline, compiled from two string constants
  (`SPRITE_VERTEX_SHADER`, `SPRITE_FRAGMENT_SHADER`) in `crates/craft-gpu/src/shaders.rs:3-58`.
  Fragment shader is `textureSample(...) * modulate`; no user input enters the pipeline.
- `GpuRenderer` (`crates/craft-gpu/src/lib.rs:62-404`) batches `SpriteDrawCommand`s by
  `texture_id` into a 1024-instance buffer (`crates/craft-gpu/src/lib.rs:287-293`).
- No material abstraction: a drawable is texture + `src_rect` + `modulate` + transform
  (`SpriteDrawCommand`, `crates/craft-kernel/src/render.rs:121-`).
- `RenderCapabilities::SHADER` bit exists in the kernel (`crates/craft-kernel/src/render.rs:25`)
  but no backend sets it: `GpuRenderer::capabilities()` returns `TEXT | SPRITE`
  (`crates/craft-gpu/src/lib.rs:401-403`).
- Dependencies are `wgpu 0.20` + `winit 0.30` (`crates/craft-gpu/Cargo.toml`); GPU window runs
  as a subprocess per ADR 0020.

#### `craft-particles` — CPU, tick-based emitter system

- Single file: `craft_system!(ParticleSystem, phase: PostTick)` in
  `crates/craft-particles/src/lib.rs:3-241` (~160 lines of logic).
- `ParticleEmitter` nodes are plain property-bag nodes; recognized components:
  `emit_rate`, `particles_per_burst`, `particle_lifetime`, `emitter_lifetime`,
  `cooldown`, `position`, `velocity`, `modulate`, `size` (`lib.rs:37-99`).
- Emission is a **radial burst only**: fixed speed 1.5, angles spread evenly by burst index
  (`lib.rs:114-118`). Integration uses a hard-coded `dt = 1.0` (`lib.rs:7`).
- Particles are regular `Node`s (type `Particle`) parented to the emitter, with
  `position`/`velocity`/`lifetime`/`max_lifetime`/`modulate`/`size`; alpha fades linearly
  via `lifetime / max_lifetime` (`lib.rs:216-237`); expiry is `mark_destroyed` + purge
  (`lib.rs:205-240`).
- Fully deterministic: no RNG anywhere in the system (compatible with ADR 0006 replay by
  construction).
- Registered into games by re-export: `games/tower_defense/src/lib.rs:4`,
  `games/dungeon_crawler/src/lib.rs:4`.
- Particles carry no `sprite` component by default; they flow through the normal
  `ComponentView` render path, so only the terminal renderer's text/modulate path or a
  sprite component added by the game makes them visible.

#### Design docs

- ADR 0004: 4-method `Render` trait; "no GPU abstraction in v1"; `SHADER` capability reserved
  as the extension seam (`docs/adr/0004-render-trait.md:78-88`).
- v3 GPU spec defers: "Custom shaders (WGSL) — v3.2 — shader injection via component" and
  "Particle systems — v3.1 — GPU compute particles"
  (`docs/superpowers/specs/2026-07-17-v3-gpu-rendering-design.md`, Deferred table).
- v3.1 shipped the CPU emitter above instead of GPU compute particles
  (`ROADMAP.md:85`, `docs/CHANGELOG.md:30-32`).
- Editor is v2 and has no shader/material panel; editor scope is fixed by ADR 0017-0019.

### 2. Godot inventory (4.7.2-stable)

#### 2.1 Shading language

- GLSL ES 3.0-like language: full scalar/vector/matrix/sampler types, structs, arrays,
  swizzling, precision qualifiers, function overloading, `in`/`out`/`inout`
  ([Shading language](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/shading_language.html)).
- Six shader types selected by `shader_type`: `spatial`, `canvas_item`, `particles`, `sky`,
  `fog`, `texture_blit`; seven processor functions (`vertex`, `fragment`, `light`, `start`,
  `process`, `sky`, `fog`) with per-type built-ins and render modes
  ([Introduction to shaders](https://docs.godotengine.org/en/stable/tutorials/shaders/introduction_to_shaders.html)).
- Uniform system with editor hints (`source_color`, `hint_range`, `hint_enum`, filter/repeat),
  uniform groups, project-wide global uniforms, and per-instance uniforms (limit 16)
  ([Shading language §Uniforms](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/shading_language.html)).
- Escape hatch for raw GPU work: `RenderingDevice` compute shaders written in GLSL
  ([Introduction to shaders, note](https://docs.godotengine.org/en/stable/tutorials/shaders/introduction_to_shaders.html)).

#### 2.2 Material system

- `Material` is a `Resource` base class; subclasses: `BaseMaterial3D` (`StandardMaterial3D`,
  `ORMMaterial3D`), `ShaderMaterial`, `CanvasItemMaterial`, `ParticleProcessMaterial`,
  `FogMaterial`, sky materials (`Panorama`/`Physical`/`Procedural`), `PlaceholderMaterial`,
  `BlitMaterial`; custom material types can be added in script/GDExtension
  ([Material class ref](https://docs.godotengine.org/en/stable/classes/class_material.html)).
- `Material.next_pass` chains multi-pass rendering; `render_priority` sorts draws
  ([Material class ref](https://docs.godotengine.org/en/stable/classes/class_material.html)).
- `StandardMaterial3D` is a full PBR material (albedo, metallic, roughness, emission, normal,
  rim, clearcoat, subsurface scattering, AO/ORM maps)
  ([StandardMaterial3D](https://docs.godotengine.org/en/stable/classes/class_standardmaterial3d.html),
  [Standard Material 3D tutorial](https://docs.godotengine.org/en/stable/tutorials/3d/standard_material_3d.html)).
- `ShaderMaterial` wraps a user `Shader` and exposes its uniforms as material parameters;
  `set_shader_parameter` / per-instance uniforms drive them at runtime
  ([ShaderMaterial class ref](https://docs.godotengine.org/en/stable/classes/class_shadermaterial.html)).

#### 2.3 Visual shader editor

- `VisualShader` resource, authored inside any `ShaderMaterial`; node-graph editor with live
  material preview, per-stage tabs (Vertex/Fragment/Light), add-node popup, varyings manager,
  and a "show generated code" view (graph compiles to the text shading language)
  ([Using VisualShaders](https://docs.godotengine.org/en/stable/tutorials/shaders/visual_shaders.html)).
- Special nodes: `Expression` (arbitrary shading-language code inline), `Fresnel`, `If`,
  `Switch`, `Reroute`, and `Mesh Emitter` (particles-mode only, emits from mesh vertices)
  ([Using VisualShaders](https://docs.godotengine.org/en/stable/tutorials/shaders/visual_shaders.html)).
- Explicitly not a full replacement: "VisualShaders do not expose all features of the shader
  script" (same page).

#### 2.4 GPU particles

- Two node families: `GPUParticles2D/3D` (GPU-simulated) and `CPUParticles2D/3D` (CPU,
  near-feature parity, frozen feature set); GPU systems scale to hundreds of thousands of
  particles ([2D tutorial](https://docs.godotengine.org/en/stable/tutorials/2d/particle_systems_2d.html),
  [3D tutorial index](https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html)).
- Node-level features: `amount`, `lifetime`, `explosiveness`, `randomness`, `one_shot`,
  `preprocess`, `speed_scale`, `fixed_fps`, `fract_delta`, `local_coords`, `draw_order`
  (index/lifetime/reverse-lifetime/view-depth in 3D), `visibility_aabb`/`visibility_rect`,
  `sub_emitter`, trails (`RibbonTrailMesh`/`TubeTrailMesh` in 3D; section/subdivision counts
  in 2D), and `use_fixed_seed` for reproducible simulation
  ([GPUParticles3D class ref](https://docs.godotengine.org/en/stable/classes/class_gpuparticles3d.html),
  [GPUParticles2D class ref](https://docs.godotengine.org/en/stable/classes/class_gpuparticles2d.html)).
- `ParticleProcessMaterial`: ~100 properties — 7 emission shapes (point, sphere,
  sphere-surface, box, points, directed-points, ring), min/max ranges plus `CurveTexture` /
  `CurveXYZTexture` / `GradientTexture1D` per parameter (velocity, acceleration, damping,
  scale, hue, alpha, color ramp), gravity, turbulence (noise field with scale/speed/strength),
  orbit/radial/tangential velocity, sub-emitter modes (constant / at start / at end /
  at collision), collision modes (disabled / rigid / hide-on-contact with bounce/friction),
  flipbook animation params
  ([ParticleProcessMaterial class ref](https://docs.godotengine.org/en/stable/classes/class_particleprocessmaterial.html)).
- Custom **particle shaders** (`shader_type particles`): `start()` + `process()` processor
  functions, per-particle state kept across frames, built-ins (`TRANSFORM`, `VELOCITY`,
  `COLOR`, `CUSTOM`, `MASS`, `USERDATA1-6`, `COLLIDED`, `COLLISION_NORMAL`,
  `ATTRACTOR_FORCE`, restart flags), and `emit_subparticle()` for shader-driven sub-emission
  ([Particle shaders](https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/particle_shader.html)).
- Environment interaction nodes: attractors (box, sphere, vector field) and colliders
  (box, sphere, baked `SDF3D`, `HeightField3D`); 2D particles collide with
  `LightOccluder2D` ([3D tutorial index](https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html),
  [GPUParticles2D class ref](https://docs.godotengine.org/en/stable/classes/class_gpuparticles2d.html)).

### 3. Graded comparison

| # | Godot feature (4.7.2) | Craft status | Grade |
|---|------------------------|--------------|-------|
| 1 | Text shading language, user-authored (6 shader types, 7 processor functions) | No path from game content to GPU code; WGSL exists only as engine-internal constants in `crates/craft-gpu/src/shaders.rs` | **missing** — deliberate scope cut: ADR 0004 ("no GPU abstraction in v1"); v3 spec defers WGSL shader injection to v3.2 |
| 2 | Shader type system (`spatial`/`canvas_item`/`particles`/`sky`/`fog`/`texture_blit`) | N/A — no shader language | **missing** (follows #1) |
| 3 | Uniforms + hints + global/per-instance uniforms | No parameter surface beyond `modulate`/`src_rect` baked into the fixed pipeline (`crates/craft-gpu/src/shaders.rs:41-58`) | **missing** (follows #1); `RenderCapabilities::SHADER` reserved as the seam (`crates/craft-kernel/src/render.rs:25`, ADR 0004) |
| 4 | Material resource abstraction + built-in PBR materials | No material concept; drawable = texture + modulate (`SpriteDrawCommand`, `crates/craft-kernel/src/render.rs:121`) | **missing** — structural **divergence**: property-bag components, no resource-class hierarchy (ADR 0002); a Craft "material" would be components + shader injection, not a Resource subclass tree |
| 5 | Multi-pass materials (`next_pass`), render priority | `z_index` sort only (`crates/craft-gpu/src/lib.rs:252`); single pass | **partial** — draw ordering exists (z-index, analogous to render priority); multi-pass absent |
| 6 | Visual shader editor (node graph → shader code) | No visual authoring of any kind for shaders; editor is v2 egui with 7 fixed panels (CHANGELOG v0.2.0 E1) | **missing** — **divergence** by policy: editor scope frozen by ADR 0017-0019; also blocked by #1 |
| 7 | GPU particle nodes (2D + 3D) | `ParticleEmitter`/`Particle` nodes, CPU-simulated in `PostTick` system (`crates/craft-particles/src/lib.rs`) | **partial** — emitter-node concept parity (2D only, no 3D); simulation is scene-node based, not GPU compute (GPU compute particles were spec'd for v3.1 but the CPU emitter shipped instead, ROADMAP:85) |
| 8 | Emission shapes (7 kinds) | Radial burst only, fixed speed, even angular spread (`crates/craft-particles/src/lib.rs:114-118`) | **partial** — point emission exists; no sphere/box/ring/points/ring-cone shapes |
| 9 | Per-parameter min/max + randomness + curves/gradients over lifetime | Fixed defaults (`emit_rate`, `particles_per_burst`, lifetimes as ints); linear alpha fade only (`lib.rs:225-237`) | **partial** — lifetime + fade parity at the basic level; no min/max spread, no CurveTexture/Gradient equivalents |
| 10 | Forces: gravity, damping, turbulence, orbit/radial/tangential | Constant-velocity integration with `dt = 1.0` (`lib.rs:7,195-197`); gravity would come from `craft-physics` on normal nodes, not the particle system | **partial** — linear motion only; no acceleration/damping/turbulence |
| 11 | Sub-emitters (constant / start / end / collision) | None | **missing** |
| 12 | Particle collision (rigid / hide-on-contact; SDF/heightfield/occluder colliders) | None in `craft-particles`; `craft-physics` collides ordinary nodes via AABB/circle but is not wired into particles | **missing** |
| 13 | Attractors | None | **missing** |
| 14 | Particle trails (ribbon/tube) | None | **missing** |
| 15 | Flipbook / animated particle textures | None (particles have no sprite component by default) | **missing** |
| 16 | One-shot, explosiveness, preprocess, speed_scale, fixed_fps, local_coords | `emitter_lifetime` self-destruct approximates one-shot; none of the others exist | **partial** |
| 17 | Deterministic particle seed (`use_fixed_seed`) | Always deterministic — no RNG used at all; survives ADR 0006 replay hashing | **divergence** — Craft is deterministic-by-default per ADR 0006, stronger than Godot's opt-in seed |
| 18 | Custom particle process shaders (`start`/`process`, cross-frame state, `emit_subparticle`) | None — particle behavior is the closed Rust system; game-side customization would be Lua behaviors on `Particle` nodes | **missing** — partial philosophical **divergence**: Craft's extension point is Lua behaviors (ADR 0003/0016), not GPU-side process code |

### 4. Structural divergences to keep in view

- **Resource model**: Godot's materials/shaders are `Resource` subclasses attached to nodes.
  Craft's ADR 0002 forbids class inheritance and uses property-bag components, so any future
  shader/material surface lands as components + (per the v3 spec) component-based WGSL
  injection, not a Material resource tree.
- **Determinism**: ADR 0006 requires per-tick hash-stable replay. Craft's particle system is
  RNG-free; any future min/max randomness must route through `engine.rng()` (ADR 0016).
  Godot's GPU turbulence/noise and sub-emitter randomness have no replay contract.
- **Tick budget**: ADR 0015 caps the tower_defense tick at 8 ms, single-threaded. Craft's
  scene-node particles cost one `Node` each in the scene graph — a deliberate trade vs.
  Godot's GPU buffers, aligned with v1/v3.1 scope but a hard ceiling on particle counts.
- **Renderer boundary**: ADR 0004's 4-method trait + `RenderCapabilities` is the only
  sanctioned route for shader support; `SHADER` is already reserved in the bitflags.

### 5. Sources

Godot (all verified 2026-08-24 against `en/stable` = 4.7):

- Release pin: <https://api.github.com/repos/godotengine/godot/releases/latest> (4.7.2-stable, published 2026-08-18)
- <https://docs.godotengine.org/en/stable/tutorials/shaders/introduction_to_shaders.html>
- <https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/shading_language.html>
- <https://docs.godotengine.org/en/stable/tutorials/shaders/shader_reference/particle_shader.html>
- <https://docs.godotengine.org/en/stable/tutorials/shaders/visual_shaders.html>
- <https://docs.godotengine.org/en/stable/classes/class_material.html>
- <https://docs.godotengine.org/en/stable/classes/class_shadermaterial.html>
- <https://docs.godotengine.org/en/stable/classes/class_standardmaterial3d.html>
- <https://docs.godotengine.org/en/stable/tutorials/3d/standard_material_3d.html>
- <https://docs.godotengine.org/en/stable/classes/class_particleprocessmaterial.html>
- <https://docs.godotengine.org/en/stable/classes/class_gpuparticles3d.html>
- <https://docs.godotengine.org/en/stable/classes/class_gpuparticles2d.html>
- <https://docs.godotengine.org/en/stable/tutorials/2d/particle_systems_2d.html>
- <https://docs.godotengine.org/en/stable/tutorials/3d/particles/index.html>

Craft (worktree `research/shaders-vfx`):

- `crates/craft-particles/src/lib.rs` (entire system, 401 lines incl. tests)
- `crates/craft-gpu/src/lib.rs`, `crates/craft-gpu/src/shaders.rs`, `crates/craft-gpu/Cargo.toml`
- `crates/craft-kernel/src/render.rs:20-27,121-`
- `docs/adr/0002-node-model.md`, `0003-behavior-runtime.md`, `0004-render-trait.md`,
  `0006-replay-system.md`, `0015-performance-budgets.md`, `0016-lua-scripting.md`,
  `0017-editor-architecture.md`–`0019`, `0020-gpu-subprocess-boundary.md`
- `docs/superpowers/specs/2026-07-17-v3-gpu-rendering-design.md` (Deferred table)
- `docs/CHANGELOG.md`, `ROADMAP.md`, `games/tower_defense/src/lib.rs`, `games/dungeon_crawler/src/lib.rs`

## 4. Physics 2D/3D (collision, rigid bodies, joints)

> Graded evidence preserved from ticket [#14 — Gap analysis: physics 2D/3D (collision, rigid bodies, joints)](https://github.com/0xnicholas/Craft/issues/14) (findings: `docs/research/gap-physics.md`, branch `research/physics`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Godot physics inventory

#### 1.1 Engines

- **Godot Physics 2D / Godot Physics 3D**: built-in engines behind `PhysicsServer2D`/`PhysicsServer3D`. 2D has no built-in alternative.
- **Jolt Physics 3D**: added in 4.4 as a built-in module, default for new 3D projects since 4.6; selectable per project via `Physics > 3D > Physics Engine`. Notable differences from Godot Physics: joint soft-limit properties unsupported, positional-only Baumgarte stabilization, collision margins shrink shapes, ghost-collision mitigation. ([Using Jolt Physics](https://docs.godotengine.org/en/stable/tutorials/physics/using_jolt_physics.html))
- Physics runs at a fixed rate (default 60 Hz) with `_physics_process(delta)` called before each step; may optionally run on a separate thread. ([Physics introduction](https://docs.godotengine.org/en/stable/tutorials/physics/physics_introduction.html))
- **Non-determinism is official**: "Physics in Godot, regardless of physics engine, is **not** deterministic." (Physics introduction, warning box)

#### 1.2 Collision objects (2D; 3D mirrors)

Four `CollisionObject2D` subclasses (Physics introduction):

- **Area2D** — detection + influence; `body_entered`/`body_exited` signals; can override gravity/damping in a region.
- **StaticBody2D** — participates in collision but never moves; can impart motion via `constant_linear_velocity`/`constant_angular_velocity`.
- **RigidBody2D** — full simulation: mass, friction, bounce, forces/impulses via `_integrate_forces` + `PhysicsDirectBodyState2D`; sleep/wake; opt-in contact reporting (`contact_monitor`, `max_contacts_reported`, `get_contact_count()`).
- **CharacterBody2D** — detection only, no response; scripted movement via `move_and_collide()` (returns `KinematicCollision2D`) and `move_and_slide()` (sliding, `is_on_floor()` etc.).

PhysicsMaterial (friction, bounce, rough, absorbent) applies to static and rigid bodies; TileSet physics layers also carry a PhysicsMaterial ([TileSet class ref](https://docs.godotengine.org/en/stable/classes/class_tileset.html)).

#### 1.3 Shapes

`Shape2D` subclasses: Rectangle, Circle, Capsule, Segment, WorldBoundary, ConvexPolygon, ConcavePolygon, SeparationRay; bodies hold any number via `CollisionShape2D`/`CollisionPolygon2D` children. Shapes also expose direct `collide`/`collide_with_motion` query methods. ([Shape2D class ref](https://docs.godotengine.org/en/stable/classes/class_shape2d.html))

#### 1.4 Joints

`Joint2D` subclasses: **PinJoint2D, GrooveJoint2D, DampedSpringJoint2D** — constrain two bodies (`node_a`/`node_b`), optional collision disable between them. ([Joint2D class ref](https://docs.godotengine.org/en/stable/classes/class_joint2d.html)) 3D adds PinJoint3D, HingeJoint3D, SliderJoint3D, ConeTwistJoint3D, Generic6DOFJoint3D (enumerated on the Jolt page).

#### 1.5 Ray / shape casting

- Nodes: `RayCast2D`/`RayCast3D` (per-frame result), `ShapeCast2D`/`ShapeCast3D` (sweep a shape along `target_position`, multiple hits). ([ShapeCast2D class ref](https://docs.godotengine.org/en/stable/classes/class_shapecast2d.html))
- Direct space queries via `PhysicsDirectSpaceState2D/3D`: `intersect_ray`, `intersect_shape`, `cast_motion`, `collide_shape`, `get_rest_info`; support `exclude` lists and `collision_mask` filtering; only safe inside `_physics_process`. ([Ray-casting tutorial](https://docs.godotengine.org/en/stable/tutorials/physics/ray-casting.html), [PhysicsDirectSpaceState2D class ref](https://docs.godotengine.org/en/stable/classes/class_physicsdirectspacestate2d.html))

#### 1.6 Collision layers and masks

Every collision object has 32 `collision_layer` bits (what it is **in**) and 32 `collision_mask` bits (what it **scans**); queryable per-bit in code, nameable in project settings, honored by all queries. (Physics introduction §Collision layers and masks)

#### 1.7 Tilemap collision

`TileSet` supports multiple **physics layers**, each with its own collision layer, mask, priority, and PhysicsMaterial; per-tile collision polygons are drawn on the atlas. `TileMapLayer` turns painted cells into static collision bodies. ([TileSet class ref](https://docs.godotengine.org/en/stable/classes/class_tileset.html))

### 2. Craft physics inventory

All engine physics lives in one file, `crates/craft-physics/src/lib.rs` (~270 lines of logic), as `craft_system!(PhysicsSystem, PreTick)`:

- **Velocity integration** (`lib.rs:9-27`): `position += velocity * dt` with `dt = 1.0` hardcoded, for any node with `velocity: Vec2` + `position: Vec2` components. No acceleration, gravity, mass, friction.
- **Tile collision** (`lib.rs:29-174`): a `Level` node carries `tilemap` (a `res://` JSON resource `{width, height, tiles}`) + `tile_size`. Tiles are binary (0 = empty, non-0 = solid). Resolution is swept (old edge → new edge, tunneling-immune) and axis-separated (Y then X, platformer order), zeroing the velocity component on contact. Used by `games/platformer` (`games/platformer/scene.json:26-33`, level data `games/platformer/assets/levels/level1.json`).
- **AABB/circle broad-phase** (`lib.rs:176-247`): O(n²) pairwise test over all nodes with `hitbox` (AABB half-extents) or `hitbox_radius` (circle) components; covers AABB-AABB, circle-circle, AABB-circle. **Detection only** — no positional response between node pairs; overlaps emit a targeted `collide` signal with payload `{a, b}` pushed to `pending_signals`, delivered next tick per ADR 0003 (asserted in `lib.rs:386-389`).
- **Game-side use**: `games/tower_defense/scene.json` wires `collide` → damage/destroy rules (bullet vs creep); `games/platformer/scripts/player_controller.lua` implements gravity (`GRAVITY = 0.2`), jump, and grounded-ness (`vel[2] == 0.0`) in Lua, and consumes `collide` for the goal check; `games/dungeon_crawler` uses `collide` for combat (ROADMAP.md:87).
- **No ray cast, shape cast, joint, layer/mask, or physics material API exists** — grep over `crates/` for `raycast|joint|collision_layer|collision_mask|shape_cast|gravity|impulse|friction|restitution` returns no matches.

Constraints on record:

- ROADMAP.md:84 — physics v3.1 acceptance is exactly "velocity integration; AABB/circle collision; targeted `collide` signal" (shipped in `v0.2.0`).
- ROADMAP.md:96 — **"Rigid-body physics — v3+ — Extends `craft-physics` beyond AABB/circle"** under "Explicitly deferred".
- ADR 0002 — no node-type inheritance: single `Node` struct + property-bag components, so no `RigidBody`/`CharacterBody`/`StaticBody` node classes.
- ADR 0003 — signals delivered next tick; two-tier behavior (Lua direct mutation, JSON command buffer).
- ADR 0006 — replay determinism is non-negotiable (per-tick state hash).
- ADR 0015 — engine is single-threaded; tick budget ≤8ms for tower_defense.

### 3. Graded comparison

| # | Godot feature | Grade | Craft status / evidence |
|---|---------------|-------|-------------------------|
| 1 | Pluggable physics backends (Godot Physics 2D/3D, Jolt) | **divergence** | One hardcoded 2D solver in `craft-physics`; determinism (ADR 0006) and single-thread (ADR 0015) rule out swapping in Jolt-class engines. Deliberate. |
| 2 | Fixed-rate physics step + `_physics_process(delta)` | **divergence** | Craft has one fixed tick for everything with `dt = 1.0` (lib.rs:7); no real-time delta. Consequence of the deterministic-tick model (ADR 0003, 0006). |
| 3 | Deterministic physics | **divergence** | Godot officially non-deterministic (Physics introduction warning); Craft mandates per-tick hash equality (ADR 0006). |
| 4 | RigidBody2D/3D (mass, forces, impulses, bounce, sleep) | **missing (deferred)** | Explicitly deferred, ROADMAP.md:96 ("v3+", extends beyond AABB/circle). No mass/impulse/gravity in the engine. |
| 5 | CharacterBody2D (`move_and_slide`, `is_on_floor`, `KinematicCollision`) | **partial** | Engine-side swept tile resolution (lib.rs:112-159) gives slide-free stop + landing; grounded/jump/gravity are hand-rolled per game in Lua (`player_controller.lua:8-33`). No collision info object (normal, point) is returned. |
| 6 | StaticBody2D (immovable world geometry, moving platforms) | **partial** | The tile grid plays the static-body role for platformer levels (lib.rs:64-75); there is no node-level static body — node-node overlaps pass through with only a signal. No `constant_linear_velocity`-style moving platforms. |
| 7 | Area2D (overlap detection, enter/exit, gravity/damping override) | **partial** | `collide {a, b}` is detection-only, like Area, but fires per-tick while overlapping (no enter/exit edge distinction) and carries no region-influence semantics. Delivery is next-tick (ADR 0003) vs Godot's same-step signals. |
| 8 | Collision shapes (8 Shape2D types + 3D set; multiple shapes per body) | **partial** | Two primitives: AABB (`hitbox`) and circle (`hitbox_radius`), one per node (lib.rs:188-206). No capsule/polygon/segment/world-boundary, no rotation. |
| 9 | Shape-level query methods (`Shape2D.collide`, `collide_with_motion`) | **missing** | Collision math (lib.rs:250-271) is private to the system; not exposed to Lua/JSON/bridge. |
| 10 | Joints (Pin/Groove/DampedSpring 2D; 5 joint types 3D) | **missing (deferred)** | Rides on the rigid-body deferral, ROADMAP.md:96. |
| 11 | Ray casting (`RayCast2D/3D`, `intersect_ray`) | **missing** | No query API; games currently fake line-of-sight in game logic (tower_defense uses range checks, `games/tower_defense/src/lib.rs`). |
| 12 | Shape casting (`ShapeCast2D/3D`, `intersect_shape`, `cast_motion`) | **missing** | Swept math exists internally for tiles only (lib.rs:29-36) but is not exposed as a space query. |
| 13 | Collision layers & masks (32-bit layer/mask) | **missing** | Every hitbox node is tested against every other (lib.rs:208-239); no filtering. Filtering today is per-handler ID checks (`player_controller.lua:39`). |
| 14 | PhysicsMaterial (friction, bounce) | **missing** | Zeroed velocity on tile contact is the only material behavior; bounce/friction absent. |
| 15 | Contact reporting (`contact_monitor`, contact points/impulses) | **partial** | The `collide` signal reports pair IDs only — no normal, point, depth, or impulse; no `max_contacts` throttling (payload is always emitted). |
| 16 | Gravity (project-level, per-area override) | **partial** | No engine gravity; each game defines its own constant in Lua (`player_controller.lua:8`). Deterministic and cheap, but not an engine feature. |
| 17 | Tilemap physics (TileSet physics layers, per-tile polygons, one-way, materials) | **partial** | Craft: binary solid grid + swept AABB resolution (lib.rs:52-174) — covers "solid tile platformer" fully and deterministically. Godot: multiple physics layers per TileSet, arbitrary polygons per tile, layer/mask + PhysicsMaterial per layer. No one-way/semi-solid tiles, slopes, or per-tile materials in Craft. |
| 18 | Physics on separate thread | **divergence** | Godot optionally runs physics on its own thread; Craft is single-threaded by ADR 0015 (≤8ms tick budget instead). |

### 4. Deliberate divergences (recorded, not gaps)

Per the ticket, deferrals backed by ROADMAP/ADRs are logged as divergence rather than missing:

- **Rigid-body simulation** (item 4, and joints at 10): deferred to "v3+" by ROADMAP.md:96; every rigid-body-adjacent absence (impulses, materials, joints) inherits this.
- **Engine pluggability / 3D physics** (item 1): Craft is 2D-only for v1–v3 (3D mesh rendering itself is deferred, ROADMAP.md:95) and deterministic (ADR 0006), which excludes Godot/Jolt-style engines by design.
- **Fixed dt / no physics thread** (items 2, 18): single fixed tick, single thread — ADR 0003, 0006, 0015.

### Sources

- [Godot Docs — Physics introduction](https://docs.godotengine.org/en/stable/tutorials/physics/physics_introduction.html) (collision objects, layers/masks, fixed-rate step, non-determinism warning) — stable branch, verified 2026-08-24
- [Godot Docs — Using Jolt Physics](https://docs.godotengine.org/en/stable/tutorials/physics/using_jolt_physics.html) (Jolt added 4.4, default for new projects, engine differences)
- [Godot Docs — Ray-casting](https://docs.godotengine.org/en/stable/tutorials/physics/ray-casting.html) (space queries, exclude/mask)
- [Godot Docs — ShapeCast2D](https://docs.godotengine.org/en/stable/classes/class_shapecast2d.html), [PhysicsDirectSpaceState2D](https://docs.godotengine.org/en/stable/classes/class_physicsdirectspacestate2d.html)
- [Godot Docs — Joint2D](https://docs.godotengine.org/en/stable/classes/class_joint2d.html), [Shape2D](https://docs.godotengine.org/en/stable/classes/class_shape2d.html), [TileSet](https://docs.godotengine.org/en/stable/classes/class_tileset.html)
- [Godot download page](https://godotengine.org/download/windows/) (4.7.2 current stable, 2026-08-18)
- Craft: `crates/craft-physics/src/lib.rs`, `games/platformer/` (scene.json, scripts/player_controller.lua, assets/levels/level1.json), `games/tower_defense/scene.json`, `ROADMAP.md:77-101`, `docs/adr/0002-node-model.md`, `0003-behavior-runtime.md`, `0006-replay-system.md`, `0015-performance-budgets.md`

## 5. Animation (frame, skeletal, animation trees, tweens)

> Graded evidence preserved from ticket [#15 — Research: animation systems — Godot inventory vs Craft gap analysis](https://github.com/0xnicholas/Craft/issues/15) (findings: `docs/research/gap-animation.md`, branch `research/animation`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Godot inventory (4.7)

#### 1.1 `Animation` (Resource) — the data container

An `Animation` holds **tracks**; each track binds a `NodePath` to timed keys. Nine track types ([class_animation](https://docs.godotengine.org/en/stable/classes/class_animation.html)):

- `TYPE_VALUE` — animate any interpolatable property
- `TYPE_POSITION_3D` / `TYPE_ROTATION_3D` / `TYPE_SCALE_3D` — dedicated transform tracks (Quaternion rotations; also used for `Skeleton3D` bone tracks, path e.g. `"character/skeleton:ankle"`)
- `TYPE_BLEND_SHAPE` — morph targets
- `TYPE_METHOD` — call functions per key
- `TYPE_BEZIER` — value via custom bezier curve handles
- `TYPE_AUDIO` — play trimmed `AudioStream`s, with optional blend-aware volume
- `TYPE_ANIMATION` — play animations in other AnimationPlayers (sub-animation)

Per-track interpolation: nearest / linear / cubic / linear-angle / cubic-angle; per-key easing transition; update modes continuous / discrete / capture; loop modes none / linear / pingpong; named **markers**; lossy in-place `compress()` for imported 3D cutscenes.

#### 1.2 `AnimationPlayer` / `AnimationMixer` — playback

([class_animationplayer](https://docs.godotengine.org/en/stable/classes/class_animationplayer.html); `AnimationPlayer` inherits `AnimationMixer`, which owns blending/capture and the `animation_finished` signal.)

- Holds a dictionary of `AnimationLibrary` resources; keys `"library/animation"` (default library omits the prefix).
- Playback: `play` / `play_backwards` / `pause` / `stop` / `seek` / `advance`, sections (`play_section`, marker-bounded), `queue` + `animation_set_next` chaining, `speed_scale` (negative = reverse).
- Crossfade: per-pair `set_blend_time` + `playback_default_blend_time`; `play_with_capture` / `playback_auto_capture` interpolate from the node's *current* value into the animation (capture tracks, `UPDATE_CAPTURE`).
- Signals: `animation_finished`, `animation_changed`, `current_animation_changed`.
- Process callback: physics frame, idle frame, or manual.

#### 1.3 `AnimationTree` — blending + state machines

([Using AnimationTree](https://docs.godotengine.org/en/stable/tutorials/animation/animation_tree.html).) Contains no animations itself; wraps an `AnimationPlayer`. Root node is one of:

- `AnimationNodeBlendTree` — 2D graph of blend nodes: `Blend2`/`Blend3` (per-track filters), `OneShot` (fade-in/out + abort), `TimeSeek`, `TimeScale`, `Transition` (indexed crossfade switch).
- `AnimationNodeBlendSpace1D` / `BlendSpace2D` — parametric blending between points in 1D/2D space (2D uses Delaunay triangles); blend modes continuous / discrete / carry; **sync modes** none / independent / cyclic-mutable / cyclic-constant for phase alignment.
- `AnimationNodeStateMachine` — states are animations/blend spaces/nested machines; transitions have **switch modes** (immediate / sync / at-end), crossfade time + curve, reset, priority, advance mode (disabled / enabled / auto), **advance condition** (bool variable) or **advance expression** (arbitrary GDScript expression, e.g. `velocity > 0`). `travel()` routes through intermediate states via **A\***.

Root motion: designate a bone as root-motion track; delta retrieved via `get_root_motion_position/rotation/scale()` and fed to e.g. `CharacterBody3D.move_and_slide`. Blending determinism relies on initial values (RESET animation / bone rest).

#### 1.4 `Tween` — script-side interpolation

([class_tween](https://docs.godotengine.org/en/stable/classes/class_tween.html).) In 4.x a `RefCounted` object from `SceneTree.create_tween()` / `Node.create_tween()`, not a node. Sequential chain of **Tweeners** (`tween_property`, `tween_method`, `tween_callback`, `tween_interval`, `tween_await(signal)`, `tween_subtween`) with `parallel()`/`set_parallel()`/`chain()` grouping; 12 transition types × 4 ease types; loops, speed scale, pause modes; signals `finished` / `step_finished` / `loop_finished`. Suited to unknown-in-advance targets; "fire-and-forget."

#### 1.5 Frame-based 2D sprite animation

`AnimatedSprite2D` + `SpriteFrames` resource ([class_animatedsprite2d](https://docs.godotengine.org/en/stable/classes/class_animatedsprite2d.html)): multiple **named** animations in one resource, per-animation fps and loop flag, `frame` + `frame_progress` properties, `play`/`pause`/`stop`/`play_backwards`/`speed_scale`, signals `frame_changed`, `animation_finished` (not emitted when looping), `animation_looped`. Alternative workflow: plain `Sprite2D` with an `AnimationPlayer` keying `frame` and `region_rect` over a spritesheet ([2D sprite animation tutorial](https://docs.godotengine.org/en/stable/tutorials/animation/2d_sprite_animation.html)).

#### 1.6 2D skeletal animation

([2D skeletons](https://docs.godotengine.org/en/stable/tutorials/animation/2d_skeletons.html).) `Skeleton2D` + `Bone2D` hierarchy with rest poses; `Polygon2D` vertices weighted to bones (weight painting) deform meshes. `SkeletonModificationStack2D` ([class ref](https://docs.godotengine.org/en/stable/classes/class_skeletonmodificationstack2d.html), marked **experimental**) stacks procedural modifiers — 2D IK (CCD, FABRIK), LookAt, jiggle, physical bones.

#### 1.7 3D skeletal animation

`Skeleton3D` bone hierarchy animated via POSITION/ROTATION/SCALE 3D tracks keyed to bone paths; blend shapes; `BoneAttachment3D`. **Retargeting** at import time via `BoneMap` + `SkeletonProfile` (humanoid preset) ([retargeting tutorial](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/retargeting_3d_skeletons.html)). Procedural modifiers: `SkeletonModifier3D` framework added in **4.4** (`LookAtModifier3D`, spring bones — [4.4 release notes](https://godotengine.org/releases/4.4/)); runtime 3D **IK returned in 4.6** after its 4.0 removal ([IK blog post](https://godotengine.org/article/inverse-kinematics-returns-to-godot-4-6/)). Root motion as in §1.3.

#### 1.8 `AnimationLibrary`

([class_animationlibrary](https://docs.godotengine.org/en/stable/classes/class_animationlibrary.html).) Named container of `Animation` resources (Godot 4.0+); AnimationPlayer holds a dictionary of libraries, enabling modular/reusable animation sets with `animation_added/changed/removed/renamed` signals.

---

### 2. Craft inventory (as built)

- **`craft-animation::AnimationSystem`** (`crates/craft-animation/src/lib.rs`): a `craft_system!` system in phase **PreTick**. A node's `animation` component holds a `res://` URI to a JSON resource `{frames: [[x,y,w,h]...], fps, loop}`. Each tick it advances integer `frame` / float `frame_time` components on a fixed **60 ticks/s** timebase (`ticks_per_frame = (60/fps).ceil()`, lib.rs:42) and writes the current rect into the `sprite_rect` component (lib.rs:89-98), which the render layer reads (`crates/craft-kernel/src/render.rs:149`). Non-looping animations freeze on the last frame (lib.rs:61-70). No signals are emitted on finish/loop. Determinism proven by byte-equal replay test (`crates/craft-animation/tests/animation_integration.rs:155-189`).
- **`animate` verb** (ADR 0003, `Action::Animate` in `crates/craft-kernel/src/behavior.rs:191-197`): declarative "interpolate any component to `to` over `duration` ticks," buffered as `ActionCommand::StartAnimation` and ticked by `tick_animations` (`crates/craft-kernel/src/engine.rs:407-439`). **However the interpolation is stubbed**: `interpolate()` ignores `from` and `remaining` and returns `to.clone()` (engine.rs:441-448) — the value snaps to target on the first tick while the countdown runs. No easing, sequencing, or callbacks.
- **State machines**: JSON `Behavior::StateMachine` (`behavior.rs:223-252`) — states with `on_enter`/`on_tick` action lists and `transitions: [{to, when}]` guarded by the 7-operator expression vocabulary (ADR 0003 Appendix A). Transition emits `state:exit:<name>` / `state:enter:<name>` signals (`crates/craft-kernel/src/evaluator.rs:93-105`). Not animation-specific; coupling to animation is by convention (a state's `on_enter` does `set_state` on the `animation` component to swap resources).
- **Lua tier** (ADR 0016): `engine` API is `emit`, `spawn`, `call_system`, `rng`, `get_node`, `log` (`crates/craft-lua/src/runtime.rs:136-159`) plus direct component get/set via field syntax (`crates/craft-lua/src/bindings.rs:45-115`). **No animation-specific Lua API**; scripts can hand-roll frame stepping by writing `frame`/`animation` components. Reference games do not yet wire the animation system (`grep "animation" games/platformer/scene.json` → 0 hits, despite the v0.3 spec calling for it — `docs/specs/2026-08-06-v3-2d-playable-surface.md:51-75`).
- **Transient components** (`ComponentKind::Transient`, `crates/craft-kernel/src/scene.rs:27-30`): engine-managed countdown for time-bounded effects (hit-flash, damage numbers — PRD §"Transient", `docs/superpowers/specs/2026-07-09-craft-prd.md:208`).

---

### 3. Graded comparison

| Godot feature | Craft counterpart | Grade | Notes |
|---|---|---|---|
| Frame-based sprite animation (`AnimatedSprite2D` + `SpriteFrames`) | `AnimationSystem` + animation JSON resource | **partial** | Parity: fps pacing, loop flag, atlas-rect frames, deterministic advance. Missing: multiple named animations per resource (Craft: one resource per animation, swapped by rewriting the component), play/pause/stop/`speed_scale`/reverse, `frame_changed`/`animation_finished`/`animation_looped` signals, `frame_progress` fractional stepping (Craft rounds up to whole ticks, lib.rs:42). |
| Keyframed property animation (`Animation` value/3D/bezier tracks, interpolation types, markers) | — | **missing** | No multi-key, multi-track resource; no keyframe interpolation (nearest/linear/cubic), no per-key easing, no method/audio/sub-animation tracks. |
| Tween | `animate` verb | **partial** (deliberate shape: **divergence**, ADR 0003 §"animate is a first-class verb" / Godot-mapping row) | Single declarative action replaces Tween's multi-call setup by design. But easing types, sequencing/parallel, callbacks, loops, and `finished` signal are absent — and interpolation itself is stubbed (snap-to-target, engine.rs:441-448). |
| Playback control (play/pause/seek/queue/sections/blend times) | — | **missing** | `AnimationSystem` has no control surface; playback is implicit and always-on once `animation` is set. |
| Animation state machine (`AnimationNodeStateMachine`) | JSON `state_machine` behavior + Lua hooks | **partial** | Parity: named states, guard expressions on transitions (`when`, ADR 0003 App. A vs Godot advance conditions/expressions), enter actions, change notifications (`state:enter/exit` signals). Missing: animation-coupled states, crossfade on transition, sync/at-end switch modes, A* `travel()`. Godot's machine plays animations; Craft's machine can only swap the `animation` resource. |
| Blend trees / blend spaces (`Blend2/3`, `OneShot`, `BlendSpace1D/2D`, `TimeScale`, `TimeSeek`) | — | **missing** | No animation blending of any kind (no crossfades either; `AnimationPlayer`-style fixed blend times also absent). |
| 2D skeletal (`Skeleton2D`, `Bone2D`, weighted `Polygon2D`, modification stack / 2D IK) | — | **missing** | No skeleton/bone/mesh-deform concepts. Structural constraint: single property-bag `Node`, no specialized node types (ADR 0002). |
| 3D skeletal (`Skeleton3D`, bone tracks, blend shapes, retargeting, `SkeletonModifier3D`/IK, physical bones) | — | **missing** | Craft v1 has no 3D scene/render surface at all (ADR 0004 render trait; `docs/ARCHITECTURE.md:149-155`). Out of v1 scope rather than rejected. |
| Root motion | — | **missing** | No notion of animation-driven locomotion; movement is `velocity`/physics or `move` verb. |
| Animation libraries | `ResourceRegistry` `res://` animation JSON | **partial** | Animation-as-shareable-resource with URI addressing exists (`crates/craft-kernel/src/resource.rs`); no named grouping of multiple animations into one library. |
| Editor: animation track editor, SpriteFrames panel, blend-tree graph | — | **divergence** (deliberate) | Editor is v2 by decree (ADR 0017–0019); v1 authoring is agent-written JSON/Lua. |
| Frame-rate/delta-time playback | Fixed 60 Hz tick timebase, byte-equal replay | **divergence** (deliberate, ADR 0006, ADR 0015) | Godot animation advances on frame delta (process/physics/manual callbacks); Craft advances per deterministic tick and proves hash equality (`tests/animation_integration.rs:155-189`). |
| Signals on animation events | `state:enter`/`state:exit` only (from state machine) | **partial** | No `animation_finished`/`looped`/`frame_changed` equivalent from `AnimationSystem`. |

---

### 4. Headline observations

1. **Craft's only working animation primitive is the sprite-frame advancer** — and it is solid within its contract (deterministic, tested), but it has zero runtime control surface: no play/pause/speed, no named animation sets, no completion signals. Everything beyond "advance rect frames at fps" is absent.
2. **The `animate` verb — Craft's Tween replacement — is a stub.** The command pipeline, countdown, and state storage exist (`evaluator.rs:19-25,772-789`; `engine.rs:407-439`), but `interpolate()` (engine.rs:441-448) ignores `from`/`remaining` and snaps to the target immediately. The ADR 0003 promise ("engine manages interpolation internally across ticks") is not yet honored by the implementation.
3. **Craft's state machine is game-logic-first, not animation-first.** It matches Godot's advance-expression model in spirit (guards + enter actions) but lacks every animation-specific affordance: crossfades, sync/at-end transitions, travel, and per-state animation playback.
4. **The entire blending/skeletal tier (blend trees, blend spaces, 2D/3D skeletons, IK, retargeting, root motion) is absent**, with no ADR weighing in either way — the v1 2D-terminal/sprite scope (ADR 0004, 0015) and the no-specialized-node-types rule (ADR 0002) implicitly exclude it.

### 5. Sources

Godot (4.7 stable docs unless noted):

- [Animation class](https://docs.godotengine.org/en/stable/classes/class_animation.html) — 9 track types, interpolation/loop/update modes, markers, compression
- [AnimationPlayer class](https://docs.godotengine.org/en/stable/classes/class_animationplayer.html) — libraries, playback API, blend times, capture, sections/queue
- [Using AnimationTree](https://docs.godotengine.org/en/stable/tutorials/animation/animation_tree.html) — blend trees/spaces, state machine transitions/travel, root motion
- [Tween class](https://docs.godotengine.org/en/stable/classes/class_tween.html) — tweeners, easing, sequencing, signals
- [AnimatedSprite2D class](https://docs.godotengine.org/en/stable/classes/class_animatedsprite2d.html) — SpriteFrames playback, signals
- [2D skeletons](https://docs.godotengine.org/en/stable/tutorials/animation/2d_skeletons.html), [SkeletonModificationStack2D class](https://docs.godotengine.org/en/stable/classes/class_skeletonmodificationstack2d.html)
- [Retargeting 3D skeletons](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/retargeting_3d_skeletons.html)
- [Godot 4.4 release](https://godotengine.org/releases/4.4/) (SkeletonModifier3D), [IK returns in 4.6](https://godotengine.org/article/inverse-kinematics-returns-to-godot-4-6/)
- [AnimationLibrary class](https://docs.godotengine.org/en/stable/classes/class_animationlibrary.html)

Craft:

- `crates/craft-animation/src/lib.rs`, `crates/craft-animation/tests/animation_integration.rs`
- `crates/craft-kernel/src/behavior.rs` (`Action::Animate`, `Behavior::StateMachine`), `crates/craft-kernel/src/engine.rs:407-448`, `crates/craft-kernel/src/evaluator.rs:19-29,71-105,772-789`, `crates/craft-kernel/src/scene.rs:27-30,460-473`, `crates/craft-kernel/src/render.rs:149`
- `crates/craft-lua/src/runtime.rs:136-159`, `crates/craft-lua/src/bindings.rs`
- `docs/adr/0002-node-model.md`, `docs/adr/0003-behavior-runtime.md`, `docs/adr/0004-render-trait.md`, `docs/adr/0006-replay-system.md`, `docs/adr/0015-performance-budgets.md`, `docs/adr/0016-lua-scripting.md`, `docs/adr/0017-0019` (editor v2)
- `docs/specs/2026-08-06-v3-2d-playable-surface.md` (animation contract), `docs/superpowers/specs/2026-07-09-craft-prd.md` (`animate` verb, transient components)

## 6. Audio (playback, mixing buses, spatial audio)

> Graded evidence preserved from ticket [#16 — Gap analysis: audio (playback, mixing buses, spatial audio)](https://github.com/0xnicholas/Craft/issues/16) (findings: `docs/research/gap-audio.md`, branch `research/audio`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Craft audio surface today

`crates/craft-audio` is a single-file crate (`src/lib.rs`, ~99 lines) plus two integration tests.

- `craft_system!(AudioSystem, phase: PostTick)` — a PostTick system that reads the signal bus's
  delivered payloads (`ctx.bus.delivered_payloads()`) each tick (`crates/craft-audio/src/lib.rs:45-86`).
- File convention: a delivered signal named `foo` resolves to `$CRAFT_ASSET_ROOT/foo.wav`, else
  `foo.ogg`; no match → no sound (`src/lib.rs:19-43`, `docs/AGENT_GUIDE.md:219-228`).
- Per-signal control is limited to two payload keys: `volume` (linear 0.0–1.0, clamped) and `loop`
  (bool), defaulting to `1.0`/`false` (`src/lib.rs:29-37`; CHANGELOG v3.2, `docs/CHANGELOG.md:34-36`).
- Playback: `rodio::Decoder` → `rodio::Sink` on a fresh `rodio::OutputStream::try_default()` per
  signal, `sink.detach()` — fire-and-forget, no handle retained (`src/lib.rs:66-84`).
  Dedup: one playback per unique signal name per tick (`HashSet`, `src/lib.rs:54-60`).
- Dependency: `rodio = "0.19"` (`crates/craft-audio/Cargo.toml:9`). rodio's decoder supports more
  formats than the `.wav`/`.ogg` probe exposes (MP3, FLAC, …), but the filename convention gates
  discovery to two extensions.
- Tests: payload parsing, clamping, format fallback, Lua `engine.emit` → bus → audio chain
  (`crates/craft-audio/tests/audio_payload_integration.rs`, `audio_chain_integration.rs`).
  **Note**: the tests reference `tests/testdata/{jump.wav, coin.ogg}` fixtures that are not committed
  (`git ls-tree` shows only 4 files in the crate); `cargo test -p craft-audio` on this branch fails
  1 of 3 integration tests (`lua_emit_reaches_the_audio_chain` panics on the missing fixture).
- Games: `tower_defense` and `dungeon_crawler` link `craft-audio`
  (`games/tower_defense/Cargo.toml:15`, `games/dungeon_crawler/Cargo.toml:14`) but ship no audio
  assets; `games/tower_defense/assets/README.md` documents sprites only.

### Comparison inventory

#### 1. Non-positional playback — `AudioStreamPlayer` — **partial**

Godot: a `Node` playing an `AudioStream` non-positionally, with `volume_db`/`volume_linear`,
`pitch_scale`, `bus`, `autoplay`, `stream_paused`, `max_polyphony`, `mix_target` (stereo/surround/
center), and methods `play(from_position)`, `seek`, `stop`, `get_playback_position`, plus a
`finished` signal.
Source: https://docs.godotengine.org/en/stable/classes/class_audiostreamplayer.html;
`scene/audio/audio_stream_player.cpp` in godotengine/godot.

Craft: equivalent capability exists only as "signal delivered → file played at given volume".
There is no player/node abstraction, no handle, no `stop`/`seek`/`pause`, no `pitch_scale`,
no `finished`-equivalent feedback signal. Loops started via `loop: true` can never be stopped
(sink is detached). Grade **partial**: simple SFX triggering works; every form of playback
control after start is missing.

#### 2. Positional audio — `AudioStreamPlayer2D` / `AudioStreamPlayer3D` — **missing**

Godot: 2D variant pans by screen position; `Area2D`/`Area3D` can divert enclosed players to
specific buses. 3D variant adds four attenuation models (inverse, inverse-square, logarithmic,
disabled), `unit_size`, `max_distance`, `max_db`, distance low-pass filter
(`attenuation_filter_cutoff_hz`/`_db`), emission-angle directionality, `panning_strength`,
Doppler tracking (idle/physics step), and listener routing via `AudioListener3D`.
Sources: https://docs.godotengine.org/en/stable/tutorials/audio/audio_streams.html (page banner:
not yet re-verified for 4.7);
https://docs.godotengine.org/en/stable/classes/class_audiostreamplayer3d.html;
`scene/2d/audio_stream_player_2d.cpp`, `scene/3d/audio_stream_player_3d.cpp`.

Craft: nothing positional — playback has no notion of a node, position, or listener. **Missing.**
(Consistent with Craft v1 being a grid/terminal-scoped engine, but no ADR explicitly scopes
positional audio out; the general server-elimination decision is ADR 0001, see §5.)

#### 3. Volume model — **partial**

Godot: decibel scale everywhere (`volume_db`, bus faders, effect gains); `volume_linear` provided
as a convenience conversion (`db_to_linear`). Per-player, per-bus, and per-effect gain stack
multiplicatively. Source: https://docs.godotengine.org/en/stable/tutorials/audio/audio_buses.html
("Decibel scale" section); class docs above.

Craft: single linear `volume` float per signal payload, clamped to `[0.0, 1.0]`, applied via
`Sink::set_volume` (`src/lib.rs:29-33`, `src/lib.rs:80`). **Partial**: per-trigger gain exists;
no dB scale, no gain above unity, no grouping/bus gain.

#### 4. Looping — **partial**

Godot: WAV loop metadata detection with override modes Forward / Ping-Pong / Backward and
sample-accurate loop begin/end points; Ogg/MP3 get `loop` + `loop_offset` (seconds);
BPM/beat-count/bar-beats import metadata for interactive music.
Source: https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_audio_samples.html.

Craft: boolean `loop` → `source.repeat_infinite()` (`src/lib.rs:72-75`). **Partial**: infinite
full-file forward looping only; no loop points, no ping-pong, no metadata, no way to stop.

#### 5. Audio bus layout & routing — **divergence** (deliberate)

Godot: `AudioServer` manages an arbitrary number of named buses; flexible left-to-right routing
into the Master bus, per-bus solo/mute/bypass, automatic disabling of silent buses, layouts
saved as `default_bus_layout.tres` (`AudioBusLayout`).
Sources: https://docs.godotengine.org/en/stable/tutorials/audio/audio_buses.html;
https://docs.godotengine.org/en/stable/classes/class_audioserver.html;
`servers/audio_server.cpp`, `servers/audio/audio_bus_layout.cpp` (godotengine/godot).

Craft: no bus concept at all — every signal opens the default output device and plays.
**Divergence**: ADR 0001 explicitly eliminates Godot's audio server from Craft v1
(`docs/adr/0001-crate-structure.md:91`, "audio server (~14K)" among eliminated subsystems);
`docs/ARCHITECTURE.md:159` lists `servers/...audio/...` as v1-excluded non-goals (~60K → 0).

#### 6. Bus effects (DSP) — **missing**

Godot: 25 documented built-in `AudioEffect` types insertable per bus and applied in order —
Amplify, filters (BandLimit/BandPass/HighPass/HighShelf/LowPass/LowShelf/Notch), Capture, Chorus,
Compressor (with sidechain ducking), Delay, Distortion, EQ/EQ6/EQ10/EQ21, HardLimiter, Limiter
(deprecated), Panner, Phaser, PitchShift, Record, Reverb, SpectrumAnalyzer, StereoEnhance.
Sources: https://docs.godotengine.org/en/stable/tutorials/audio/audio_effects.html;
`servers/audio/effects/` in godotengine/godot.

Craft: none. **Missing** (falls under the same ADR 0001 scope-out, but unlike buses there is no
craft-side analog at any level, not even a single global effect).

#### 7. Interactive music — **missing**

Godot: dedicated `interactive_music` module with three stream types —
`AudioStreamInteractive` (clip set + transition table; transitions quantized to
immediate/beat/bar/end, fades in beats incl. crossfade, filler clips, hold/auto-advance),
`AudioStreamPlaylist`, `AudioStreamSynchronized` (parallel stems with per-stream fade).
Ogg/MP3 import captures BPM/beat/bar metadata to drive beat-aligned transitions.
Sources: https://docs.godotengine.org/en/stable/classes/class_audiostreaminteractive.html;
verified source listing `modules/interactive_music/{audio_stream_interactive,audio_stream_playlist,audio_stream_synchronized}.{cpp,h}` (godotengine/godot@master);
import metadata: importing_audio_samples.html (BPM / Beat Count / Bar Beats).

Craft: `loop: true` on a music file is the entire music feature. **Missing.**

#### 8. Streaming formats & import pipeline — **partial**

Godot: imports WAV (PCM/IMA-ADPCM/QOA compression after import, 8-bit/mono/rate forcing, trim,
normalize, loop modes), Ogg Vorbis, MP3; trade-off guidance (WAV for short SFX, Ogg for music,
MP3 for CPU-constrained targets). Source: importing_audio_samples.html; importers in
`editor/import/resource_importer_wav.cpp`, `modules/vorbis/`, `modules/minimp3/`.

Craft: decoder-agnostic-by-convention — only `<signal>.wav` / `<signal>.ogg` are probed
(`src/lib.rs:20-28`); rodio decodes them directly from disk at play time, no import/transcode
step, no MP3 despite rodio supporting it. **Partial** on format support; the import pipeline
itself is **divergence**: editor/asset-import is v2 by ADR 0017
(`docs/adr/0017-editor-architecture.md`; AGENTS.md constraint table).

#### 9. Voice management / polyphony — **missing**

Godot: `max_polyphony` per player caps simultaneous voices and steals the oldest
(class_audiostreamplayer.html); the server mixes all voices on a dedicated audio thread.

Craft: one sink per unique signal per tick, deduped by name (`src/lib.rs:54-60`) — the opposite
of polyphony control: repeated same-name signals within a tick are collapsed, and across ticks
voices accumulate unbounded (a new `OutputStream` per signal; no cap, no stealing). **Missing.**

#### 10. Trigger model — **divergence** (deliberate)

Godot: scripts call `play()` on a player node, often from signal callbacks; playback is an
imperative node operation.

Craft: audio is purely signal-bus-driven — Lua `engine.emit("jump", {...})` is delivered
next-tick and resolved to a file by name convention; no agent/Lua-visible audio API exists
beyond signal payloads (`audio_chain_integration.rs:18-21`). This is a deliberate consequence of
the two-tier behavior model and next-tick signal delivery: ADR 0003
(`docs/adr/0003-behavior-runtime.md`; AGENTS.md constraint table). **Divergence.**

### Grade summary

| # | Godot feature | Grade | Notes |
|---|---------------|-------|-------|
| 1 | AudioStreamPlayer (non-positional) | partial | trigger+volume only; no control after start |
| 2 | AudioStreamPlayer2D/3D positional | missing | no position/listener/attenuation/doppler |
| 3 | Volume model | partial | linear payload float vs dB-scale stack |
| 4 | Looping | partial | bool infinite loop vs modes + loop points |
| 5 | Bus layout & routing | divergence | ADR 0001 / ARCHITECTURE v1 non-goal |
| 6 | Bus effects (25 DSP types) | missing | no effect surface at all |
| 7 | Interactive music | missing | no clips/transitions/BPM |
| 8 | Formats & import pipeline | partial | WAV/OGG by convention; import is v2 (ADR 0017) |
| 9 | Polyphony/voice management | missing | per-tick dedup, unbounded across ticks |
| 10 | Trigger model | divergence | signal-bus-native by ADR 0003 |

### Known Craft-side defect surfaced during this research

`crates/craft-audio/tests/testdata/` fixtures (`jump.wav`, `coin.ogg`) are referenced by both
integration test files but absent from git; `cargo test -p craft-audio` fails on
`audio_chain_integration::lua_emit_reaches_the_audio_chain` on a clean checkout of
`research/audio` (verified 2026-08-24; binary fixtures were presumably never committed).

### Sources

Godot (verified against stable = 4.7.x, 4.7.1 released 2026-07-14):

- https://godotengine.org/article/maintenance-release-godot-4-7-1/
- https://docs.godotengine.org/en/stable/tutorials/audio/audio_buses.html
- https://docs.godotengine.org/en/stable/tutorials/audio/audio_streams.html (banner: not yet updated for 4.7)
- https://docs.godotengine.org/en/stable/tutorials/audio/audio_effects.html
- https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/importing_audio_samples.html
- https://docs.godotengine.org/en/stable/classes/class_audiostreamplayer.html
- https://docs.godotengine.org/en/stable/classes/class_audiostreamplayer3d.html
- https://docs.godotengine.org/en/stable/classes/class_audiostreaminteractive.html
- https://docs.godotengine.org/en/stable/classes/class_audioserver.html
- GitHub godotengine/godot@master: `modules/interactive_music/` (listing verified via API),
  `scene/audio/audio_stream_player.cpp`, `scene/2d/audio_stream_player_2d.cpp`,
  `scene/3d/audio_stream_player_3d.cpp`, `servers/audio_server.cpp`, `servers/audio/effects/`,
  `editor/import/resource_importer_wav.cpp`, `modules/vorbis/`, `modules/minimp3/`

Craft (branch `research/audio`):

- `crates/craft-audio/src/lib.rs`, `crates/craft-audio/Cargo.toml`
- `crates/craft-audio/tests/audio_payload_integration.rs`, `audio_chain_integration.rs`
- `docs/CHANGELOG.md` (Audio v3.2), `docs/AGENT_GUIDE.md` §11, `docs/ARCHITECTURE.md:159`
- `docs/adr/0001-crate-structure.md`, `0003-behavior-runtime.md`, `0017-editor-architecture.md`
- `games/tower_defense/Cargo.toml`, `games/dungeon_crawler/Cargo.toml`, `games/tower_defense/assets/README.md`

## 7. UI/GUI (controls, themes, layout containers)

> Graded evidence preserved from ticket [#17 — Gap analysis: Control-based GUI (UI / GUI)](https://github.com/0xnicholas/Craft/issues/17) (findings: `docs/research/gap-ui-gui.md`, branch `research/ui-gui`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Craft current state (verified in this worktree)

- No UI node model exists. The scene tree has a single `Node` struct with a property-bag component model (`docs/adr/0002-node-model.md`); there are no Control-equivalent node types, no layout properties, and no GUI input routing in `crates/craft-kernel/src/`.
- Text rendering is specced but not committed. The v0.3.0 spec defines `text` / `text_pos` / `text_scale` components drawn by a craft-gpu bitmap-font text pass in screen space (`docs/specs/2026-08-06-v3-2d-playable-surface.md` §3 "HUD text"). In the committed tree, `crates/craft-gpu/src/lib.rs` implements only the instanced sprite pipeline; no `text_pos`/`text_scale` identifiers exist anywhere under `crates/`. Note `GpuRenderer::capabilities()` already advertises `TEXT | SPRITE` (`crates/craft-gpu/src/lib.rs:402`) although the committed code has no text pass. (A text pass is in progress as uncommitted work on the main working tree; it is not visible from this branch.)
- A built-in bitmap font asset exists: 5×7 px glyphs, 54-character uppercase-ASCII subset (`GLYPH_ORDER`, `crates/craft-assets/src/font.rs:6-335`), packed into `font.png` + `font.json` metadata by the deterministic asset generator (`builtin_font()`, `crates/craft-assets/src/font.rs:352`).
- `craft-terminal` renders the scene as an ANSI glyph grid (one char per node type, `glyph_for_type`, `crates/craft-terminal/src/lib.rs:70`) and reports `RenderCapabilities::TEXT` (`crates/craft-terminal/src/lib.rs:157`). This is scene debug visualization, not authored UI.
- The `craft-editor` egui app has a hand-rolled dark theme (`crates/craft-editor/src/theme.rs`) and docked panels, but per ADR 0017–0019 it is v2 tooling, not in-game UI, and ADR 0017 explicitly chose egui "instead of building a custom GUI toolkit" (`docs/adr/0017-editor-architecture.md:11`).

### Godot inventory and grades

#### 1. Control node model & scene-tree UI — **missing**

Godot: `Control` (extends `CanvasItem`) is the base of every GUI node; GUI elements are ordinary scene-tree nodes with anchors, offsets, minimum size, focus modes, mouse filter, and theme hooks. Source: `scene/gui/control.cpp/.h`; class ref <https://docs.godotengine.org/en/stable/classes/class_control.html>.

Craft: no Control-equivalent node type or component set exists in the kernel. The closest surface is the `Render` trait's component view extraction (`crates/craft-kernel/src/render.rs`). Craft would also never replicate the class hierarchy itself — the single-`Node` property-bag model is a locked decision (ADR 0002), so any UI surface would arrive as components/resources, not a Control subclass tree. Grade: **missing**, with the hierarchy shape itself a **divergence** (ADR 0002).

#### 2. Anchors, offsets, size flags — **missing**

Godot: every Control has four anchors (0.0–1.0 relative to parent) plus pixel offsets, grow direction, and container sizing options (Fill, Expand, Shrink Begin/Center/End, Stretch Ratio) for resolution-independent layout. Docs: <https://docs.godotengine.org/en/stable/tutorials/ui/size_and_anchors.html> and <https://docs.godotengine.org/en/stable/tutorials/ui/gui_containers.html> ("Sizing options").

Craft: the specced HUD text uses absolute screen pixels (`text_pos` in "screen pixels", `docs/specs/2026-08-06-v3-2d-playable-surface.md` §3). No anchoring or relative-layout concept exists.

#### 3. Layout containers — **missing**

Godot ships ~13 container types: `HBoxContainer`/`VBoxContainer`, `GridContainer`, `MarginContainer`, `TabContainer`, `HSplitContainer`/`VSplitContainer`, `PanelContainer`, `FoldableContainer`, `ScrollContainer`, `AspectRatioContainer`, `HFlowContainer`/`VFlowContainer`, `CenterContainer`, `SubViewportContainer`, plus custom containers via `NOTIFICATION_SORT_CHILDREN`. Docs: <https://docs.godotengine.org/en/stable/tutorials/ui/gui_containers.html>. Source: `scene/gui/container.cpp`, `box_container.cpp`, `grid_container.cpp`, `margin_container.cpp`, `tab_container.cpp`, `split_container.cpp`, `panel_container.cpp`, `foldable_container.cpp`, `scroll_container.cpp`, `aspect_ratio_container.cpp`, `flow_container.cpp`, `center_container.cpp`, `subviewport_container.cpp`.

Craft: nothing. There is no parent-driven child layout anywhere in `crates/craft-kernel/src/scene/`.

#### 4. Themes / skinning — **missing**

Godot: the `Theme` resource stores typed items (Color, Constant, Font, Font size, Icon, StyleBox) per theme type, with type variations, per-node local overrides, per-branch `theme` properties, a project-wide custom theme, and the engine default theme; lookup cascades local override → nearest ancestor theme → project theme → default theme. Docs: <https://docs.godotengine.org/en/stable/tutorials/ui/gui_skinning.html>. Source: `scene/resources/theme.cpp/.h`.

Craft: no theme resource type exists in `crates/craft-kernel/src/resource/`. The only theming in the repo is the editor's egui `Visuals` setup (`crates/craft-editor/src/theme.rs`) — tooling, not engine. Grade: **missing** (engine surface). Choosing egui for the editor rather than dogfooding an engine GUI is a **divergence** (ADR 0017).

#### 5. StyleBox — **missing**

Godot: `StyleBox` (base), `StyleBoxFlat` (procedural rounded/beveled boxes), `StyleBoxTexture` (texture-based, see §6), `StyleBoxLine`, `StyleBoxEmpty`; drawn as control backgrounds/overlays with content margins. Source: `scene/resources/style_box*.cpp/.h`; referenced throughout <https://docs.godotengine.org/en/stable/tutorials/ui/gui_skinning.html>.

Craft: no panel/background primitive. The sprite renderer can draw a textured quad (`SpriteDrawCommand`, `crates/craft-kernel/src/render.rs:121`), which is the only building block available.

#### 6. Nine-patch — **missing**

Godot: `NinePatchRect` splits a texture into a 3×3 grid — corners preserved, edges tiled on one axis, center tiled on both — with per-side `patch_margin_*`, `region_rect`, `draw_center`, and three axis stretch modes (Stretch / Tile / TileFit). The same 9-slice logic backs `StyleBoxTexture`, so themed panels scale without distortion. Class ref: <https://docs.godotengine.org/en/stable/classes/class_ninepatchrect.html>. Source: `scene/gui/nine_patch_rect.cpp`, `scene/resources/style_box_texture.cpp`.

Craft: sprites support `src_rect` sub-texture sampling (`crates/craft-gpu/src/lib.rs:257-272`) but there is no 9-slice draw mode, no patch-margin metadata in the atlas format, and no tiled-center mode. Grade: **missing**.

#### 7. Rich text — **missing**

Godot: `RichTextLabel` parses BBCode with ~30 built-in tags (bold/italic/underline, color/bgcolor/fgcolor, font/font_size/OpenType features, tables, lists, images, URLs with `meta_clicked`, dropcaps, alignment, BiDi control tags), animated text effects (pulse, wave, tornado, shake, fade, rainbow), custom effects via `RichTextEffect`, escaping via `[lb]`/`[rb]`, incremental `append_text()`, and an optional threaded parse. Docs: <https://docs.godotengine.org/en/stable/tutorials/ui/bbcode_in_richtextlabel.html>. Source: `scene/gui/rich_text_label.cpp`, `scene/gui/rich_text_effect.cpp`.

Craft: the committed font is a 54-glyph monochrome uppercase-ASCII bitmap (`crates/craft-assets/src/font.rs:6`); the v0.3.0 spec explicitly scopes out "rich text (only digits + minimal ASCII in the font)" (`docs/specs/2026-08-06-v3-2d-playable-surface.md` "Non-goals"). No markup, no inline styling, no embedded images. Grade: **missing** (scoped out for v0.3.0, not an ADR-level divergence).

#### 8. Plain text / HUD rendering — **partial**

Godot: `Label` node with theme-driven font/color/size, alignment, wrapping, autowrap, clipping; `scene/gui/label.cpp`.

Craft: the bitmap font asset and its `font.json` char→src-rect metadata are committed and tested (`crates/craft-assets/src/font.rs`, `crates/craft-assets/tests/font_integration.rs`); the consuming GPU text pass and the `text`/`text_pos`/`text_scale` components are specced but absent from the committed tree (see "Craft current state"). The terminal renderer's ANSI grid is debug visualization, not authored HUD. Grade: **partial** — asset side done, render side in flight, no layout/wrapping/alignment even in the spec.

#### 9. Font system — **partial**

Godot: `Font` resource abstraction (FontFile TTF/OTF/WOFF with OpenType features and variation coordinates, BitmapFont, SystemFont), font fallbacks per language script, embolden/slant transforms, MSDF rendering, and a project default font. See Font options in <https://docs.godotengine.org/en/stable/tutorials/ui/bbcode_in_richtextlabel.html> and the localization notes on fallback fonts in <https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html>.

Craft: one hard-coded 5×7 bitmap font baked into the asset crate (`crates/craft-assets/src/font.rs`); no font resource type, no fallback chain, no lowercase or non-Latin glyphs. Grade: **partial** (a working bitmap-font path exists, nothing general).

#### 10. Localization-aware UI — **missing**

Godot: `TranslationServer` with runtime locale switching and `tr()` / `tr_n()` (pluralization) / translation contexts; Button/Label auto-translate their text against translation keys with a per-node Auto Translate opt-out; resource remaps per locale (images, audio); pseudolocalization project setting to stress-test UI length; per-locale font fallbacks; localized number formatting via `TextServer.format_number`. Docs: <https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html>. Source: `core/string/translation_server.cpp`, `core/string/translation.cpp`, `core/string/translation_domain.cpp`.

Craft: no translation/localization surface anywhere; the only user-visible strings are Lua/JSON data. The 54-glyph uppercase-ASCII font cannot render accented capitals from pseudolocalization, let alone CJK/Cyrillic. Grade: **missing**.

#### 11. BiDi / RTL & UI mirroring — **missing**

Godot: transparent bidirectional text and automatic UI mirroring for RTL locales — anchors/margins mirror, text alignment swaps, container child order and internal control elements mirror, with per-control `text_direction`, `language`, `layout_direction`, and `structured_text_bidi_override` overrides. Docs: <https://docs.godotengine.org/en/stable/tutorials/i18n/internationalizing_games.html> ("Bidirectional text and UI mirroring").

Craft: nothing — no text shaping, no directionality, no layout to mirror. Grade: **missing**.

#### 12. GUI input, focus, tooltips — **missing**

Godot: Controls participate in mouse/keyboard/gamepad input via `mouse_filter` (Stop/Pass/Ignore), focus neighbors and grab_focus, `gui_input` signals, hover/tooltip, and drag-and-drop notifications. Source: `scene/gui/control.cpp`; class ref <https://docs.godotengine.org/en/stable/classes/class_control.html>.

Craft: input is a global directional/action state fed from the window event loop (`engine.set_input_direction`, `crates/craft-gpu/src/lib.rs:522-523`); there is no hit-testing against UI elements, no focus chain, no tooltips. Grade: **missing**.

### Summary table

| # | Capability | Grade |
|---|------------|-------|
| 1 | Control node model | missing (hierarchy shape: divergence, ADR 0002) |
| 2 | Anchors / size flags | missing |
| 3 | Layout containers | missing |
| 4 | Theme resource / skinning | missing (editor toolkit choice: divergence, ADR 0017) |
| 5 | StyleBox | missing |
| 6 | Nine-patch | missing |
| 7 | Rich text / BBCode | missing (v0.3.0 non-goal) |
| 8 | HUD text | partial (asset committed, text pass uncommitted) |
| 9 | Font system | partial (single 5×7 ASCII bitmap) |
| 10 | Localization | missing |
| 11 | BiDi / RTL mirroring | missing |
| 12 | GUI input / focus | missing |

### Unverified / caveats

- The craft-gpu text pass is uncommitted on the main working tree; this doc grades only what is committed at `245ce9d`. Item 8's "partial" may already be stale against the working tree.
- `GpuRenderer::capabilities()` reporting `TEXT` (`crates/craft-gpu/src/lib.rs:402`) without a committed text pass looks like a capability-flag inconsistency; flagged, not resolved here.
- Godot class-reference pages for `Control`, `Label`, `Theme`, and `StyleBoxTexture` were cited from the stable docs index and cross-checked against verified source paths, but those individual pages were not fetched in full; statements about them rely on the fetched tutorial pages plus the `NinePatchRect` class page, which was fetched.

## 8. Scene system + asset pipeline (instancing, inheritance, import)

> Graded evidence preserved from ticket [#18 — Gap analysis: scene instancing, PackedScene, resource formats & import pipeline](https://github.com/0xnicholas/Craft/issues/18) (findings: `docs/research/gap-scene-assets.md`, branch `research/scene-assets`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Godot inventory (4.7.2-stable)

#### Scenes and PackedScene
- A scene is a node tree with exactly one root; scenes can be saved to disk and instanced into other scenes ([scene_tree.html](https://docs.godotengine.org/en/stable/tutorials/scripting/scene_tree.html), [class_node.html](https://docs.godotengine.org/en/stable/classes/class_node.html)).
- Saved scenes are resources of type `PackedScene` — "an abstraction of a serialized scene"; `instantiate()` builds the node hierarchy (recursively instantiating child scenes), `pack()` serializes a node plus its owned sub-nodes ([class_packedscene.html](https://docs.godotengine.org/en/stable/classes/class_packedscene.html)).
- Runtime instancing is a two-step API: `load("res://x.tscn")` returns the PackedScene; `PackedScene.instantiate()` returns the root node, fast enough to spawn bullets/enemies repeatedly ([nodes_and_scene_instances.html](https://docs.godotengine.org/en/stable/tutorials/scripting/nodes_and_scene_instances.html)).
- Resource loading is cached: a resource path loads **once**; repeated loads return the same copy, and resources are shared across scene instances ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html)).

#### Scene instancing, overrides, inheritance
- Scenes are templates; instancing nests one scene inside another. Instances support per-instance property overrides and "editable children" ([nodes_and_scene_instances.html](https://docs.godotengine.org/en/stable/tutorials/scripting/nodes_and_scene_instances.html)).
- Scene **inheritance** ("New Inherited Scene"): a derived scene adds nodes and overrides properties of a base scene; base-scene nodes cannot be removed ([import_configuration.rst](https://github.com/godotengine/godot-docs/blob/master/tutorials/assets_pipeline/importing_3d_scenes/import_configuration.rst) §"Inherited scenes"). `PackedScene.instantiate(GEN_EDIT_STATE_MAIN_INHERITED)` and `EditorInterface.open_scene_from_path(..., set_inherited)` expose this programmatically ([class_packedscene.html](https://docs.godotengine.org/en/stable/classes/class_packedscene.html), [class_editorinterface.html](https://docs.godotengine.org/en/stable/classes/class_editorinterface.html)). Note: the manual has no dedicated scene-inheritance tutorial page in current stable docs; godot-docs issue [#5529](https://github.com/godotengine/godot-docs/issues/5529) tracks this gap.
- In the text scene format this surfaces as `[node ... instance=ExtResource("...")]`, `instance_placeholder=`, and `[editable path="..."]` tags, parsed in `scene/resources/resource_format_text.cpp` (tags handled at lines ~191, 234, 243, 311, 391 of that file at tag 4.7.2-stable) and state stored via `SceneState` in `scene/resources/packed_scene.cpp`.

#### Resource formats
- Everything saved/loaded is a `Resource`: scenes (`.tscn`/`.scn`), textures, scripts, meshes, audio, etc. ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html)).
- Text formats (`.tscn`, `.tres`) are version-control friendly; binary formats (`.scn`, `.res`) are used on export for speed/compression ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html)). Loaders: `scene/resources/resource_format_text.cpp`, `core/io/resource_format_binary.cpp`, dispatch via `core/io/resource_loader.cpp` (paths at tag 4.7.2-stable).
- Resources can be external files or built-in (embedded in the scene file); user-defined Resource subclasses (ScriptableObject-style) serialize custom exported properties and sub-resources recursively ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html)).

#### Import pipeline
- Non-native assets (PNG, WAV, glTF, fonts…) are copied into the project and auto-imported by the **editor**; imported artifacts live hidden in `res://.godot/imported/` (e.g. `.ctex`), with a per-asset `<asset>.import` config file meant for VCS ([import_process.html](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/import_process.html)).
- Automatic reimport triggers on MD5 change of the source asset; importers are extensible via `EditorImportPlugin` (`editor/import/editor_import_plugin.cpp`, `core/io/resource_importer.cpp` at tag 4.7.2-stable).
- Godot-native `.tscn/.scn/.tres/.res` have **no** import options; imported assets must be accessed through `ResourceLoader`, not `FileAccess`, or exports break ([import_process.html](https://docs.godotengine.org/en/stable/tutorials/assets_pipeline/import_process.html)).
- 3D scenes (glTF/COLLADA/OBJ) import as generated scene trees and can be opened as inherited scenes with limited modification ([import_configuration.rst](https://github.com/godotengine/godot-docs/blob/master/tutorials/assets_pipeline/importing_3d_scenes/import_configuration.rst)).

### Craft inventory (this worktree)

- **Scene format**: single JSON document — `{"kind": "scene", "name", "nodes": [...], "spawn_counter"}`; nodes are a **flat list** with `id`, `type`, `parent` (string id), `components` (property bag), `behaviors`, optional `lua_class`/`active_state` (`crates/craft-kernel/src/scene.rs:200-209, 45-72`; schema in `docs/SCHEMA.md:5-50`). Unknown fields/types/components produce structured `ValidationError`s with json_path + suggestion (`crates/craft-kernel/src/scene.rs:226-285, 477-572`).
- **Node model**: one `Node` struct; type = `type_name` + per-type component specs registered via `NodeDef`/`NodeRegistry` (inventory crate), validated at load, not compile time (`crates/craft-kernel/src/scene.rs:680-763`; ADR `docs/adr/0002-node-model.md`).
- **"Instancing"**: the `spawn` action verb creates a node from `{type, components, behaviors}` inline — no reference to another scene file, no template resource (`crates/craft-kernel/src/behavior.rs:168-177`; example: `games/tower_defense/scene.json` spawner). IDs are flat strings; spawns get reserved `__spawn_<type>_<counter>` ids (`crates/craft-kernel/src/scene.rs:313-318`).
- **Resources**: `ResourceRegistry` maps `res://` URIs to **JSON-only** data with monotonically increasing `version`s; `ResourceRef` snapshots the version at load ("loaded instances not retroactively updated") (`crates/craft-kernel/src/resource.rs:9-171`; ADR `docs/adr/0009-hot-reload-and-resources.md` §Resource System).
- **Hot reload**: `notify` watcher → re-parse/validate → `compute_scene_diff` → targeted patch (add/remove/type-change/behavior/component) preserving node identity; emits affected ids (`crates/craft-kernel/src/hot_reload.rs:63-329`; ADR 0009 pipeline).
- **Schema**: compile-time JSON Schema via schemars + `craft-schema`, generating TS SDK types (ADR `docs/adr/0005-schema-pipeline.md`; `docs/SCHEMA.md`).
- **craft-assets**: deterministic generator, not an importer — JSON pixel-map (palette + ASCII rows + scale) → PNG atlas + `.atlas.json` (`crates/craft-assets/src/pixel_map.rs:70-275`); built-in 5x7 bitmap font (`font.rs`); PCM16 WAV tone generator (`wav.rs`). CLI: `crates/craft-assets/src/main.rs`. No `.import` metadata, no MD5 reimport, no binary cache.

### Graded comparison

| # | Godot feature | Craft status | Grade | Notes / sources |
|---|---------------|--------------|-------|-----------------|
| 1 | Scene = saved node tree, single root | Single JSON file, flat node list, no root-node concept | **divergence** | Deliberate: flat list + `parent` ids is the agent-facing model (ADR 0002). `crates/craft-kernel/src/scene.rs:200-209` |
| 2 | PackedScene as loadable/serializable resource with `instantiate()`/`pack()` | `Scene::load`/`to_value` only; no scene-as-resource type, no runtime pack | **missing** | [class_packedscene.html](https://docs.godotengine.org/en/stable/classes/class_packedscene.html); `crates/craft-kernel/src/scene.rs:214-289` |
| 3 | Runtime instancing from a scene template (spawn N copies of a saved scene) | `spawn` verb with inline components; no file-backed template | **partial** | Closest equivalent exists (behavior.rs:168-177) but no "instance of scene X" — templates would have to be JSON resources + user code |
| 4 | Per-instance property overrides on instanced scenes | Component values are per-node by construction | **divergence** | No shared template means no override mechanism needed (ADR 0002: property bag is the whole representation) |
| 5 | Editable children / instance placeholders | None | **missing** | `[editable path=...]` in `resource_format_text.cpp`; no Craft analog |
| 6 | Scene inheritance (derived scene extends base scene) | None; hot reload diffs a scene against its own next version | **missing** | Only adjacent mechanism: `compute_scene_diff` (`hot_reload.rs:63-103`) computes overrides-like deltas between two full scenes |
| 7 | Text scene format, VCS-friendly | JSON scenes | **parity** | Both human-readable/diffable text (`.tscn` vs `scene.json`); ADR 0009 §"JSON, not binary" |
| 8 | Binary scene/resource format (`.scn`/`.res`) for export | None | **divergence** | Deliberate: no binary formats (ADR 0009). Cost: larger files, parse-per-load |
| 9 | Resource cache: load once, share across instances | Registry is idempotent per URI (`load` returns same `ResourceId`) | **partial** | `crates/craft-kernel/src/resource.rs:87-116`; but data is copied per spawn — no shared reference semantics |
| 10 | Resource live-update: reimport propagates to all references | Explicit opposite: stale instances keep old version | **divergence** | Deliberate per ADR 0009 ("loaded instances not retroactively updated"), implemented via `version`/`snapshot_version` (`resource.rs:24-40`) |
| 11 | External vs built-in (embedded) resources | JSON resources always external files or inline component values | **missing** | No sub-resource embedding in Craft scenes |
| 12 | User-defined Resource subclasses with typed properties + methods | JSON `Value` data only, schema via craft-schema generation | **divergence** | Deliberate: no runtime reflection (ADR 0005 vs ClassDB); typed access via compile-time schema, not class methods |
| 13 | Import pipeline (source asset → engine artifact, `.import` files, MD5 auto-reimport, `EditorImportPlugin`) | None | **missing** | `crates/craft-assets` is a manual CLI generator; no watcher, no cache dir, no plugin API |
| 14 | Rich asset types (PNG/WAV/glTF/BMFont importers) | Deterministic generators: pixel-map→PNG atlas, bitmap font, WAV tones; games ship plain PNGs (`games/tower_defense/assets/README.md`) | **partial** | Craft covers sprites/fonts/SFX only through authored-generation; no arbitrary image/audio/model ingestion |
| 15 | Hot reload of scenes incl. structural changes (add/remove nodes) | Full: notify watcher + diff + hot-patch, ids stable | **parity** (Craft exceeds Godot) | `crates/craft-kernel/src/hot_reload.rs:210-329`; ADR 0009 notes Godot reloads only scripts at runtime — structural scene reload is editor-only in Godot |
| 16 | Hot reload of resource data | Version bump on re-register; not retroactive | **partial** | Deliberate semantics (ADR 0009), but no file watcher on resource JSONs — reload is API-driven, not automatic |

### Deliberate divergences (do not treat as gaps)

- Property-bag `Node` instead of class hierarchy / `cast_to` — ADR 0002 (supersedes GDCLASS model).
- JSON-only scenes and resources, no binary export format, no import pipeline for v1 — ADR 0009 ("No import pipeline (v1 ASCII only)").
- Snapshot-versioned resources, no retroactive live update — ADR 0009.
- Compile-time schema generation instead of runtime ClassDB/MethodBind reflection — ADR 0005.

### Claims not fully verified

- The exact removal/location of a dedicated "scene inheritance" tutorial page in stable docs: the page is absent from the current `tutorials/scripting` toctree (verified against godot-docs `master` tree); historical versions linked it from class pages. Inheritance behavior above is sourced from `import_configuration.rst`, class reference, and engine source instead.
- Line numbers cited inside `resource_format_text.cpp` were taken from the raw file at tag `4.7.2-stable` via grep (tag existence and file presence verified via GitHub API); full function-level review of that file was not performed.

## 9. Scripting & languages (GDScript/C#/GDExtension vs Lua/JSON two-tier)

> Graded evidence preserved from ticket [#19 — Gap analysis: Godot scripting surface vs Craft's two-tier behavior model](https://github.com/0xnicholas/Craft/issues/19) (findings: `docs/research/gap-scripting.md`, branch `research/scripting`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Godot Scripting Inventory

Godot 4.7 exposes four scripting surfaces:

#### 1.1 GDScript

Custom, high-level, object-oriented, imperative, gradually typed language built for and tightly integrated with the engine (docs: `tutorials/scripting/gdscript/gdscript_basics.html`).

- **OOP with inheritance**: every file is a class; `extends`, `super()`, inner classes, `class_name` global registration, `static` vars/functions, abstract classes/methods, variadic functions (rest parameter, since 4.5). *(docs: gdscript_basics.html §Keywords, §Static variables, §Variadic functions)*
- **Gradual typing**: optional type hints, inference (`:=`), typed arrays and typed dictionaries (4.4+), static analyzer with warnings that can be treated as errors. *(docs: gdscript_basics.html §Variables, §Typed arrays, §Typed dictionaries)*
- **Signals first-class**: `signal` keyword, `Callable` type, `await` on signals/coroutines/timers. *(docs: gdscript_basics.html §Keywords (`await`, `signal`), §Built-in types (`Signal`, `Callable`))*
- **Editor metadata annotations**: `@export`, `@onready`, `@icon`, `@static_unload`, `breakpoint` keyword. *(docs: gdscript_basics.html §Annotations)*

#### 1.2 C# / .NET

Implemented on the modern .NET runtime. *(docs: `tutorials/scripting/c_sharp/c_sharp_basics.html`)*

- Requires the .NET SDK installed separately (Godot 4.5: .NET 8+; Android export needs .NET 9+); the .NET-enabled Godot build is required.
- Platform limits: **web unsupported**; Android/iOS available since 4.2 but **experimental**; desktop export (Linux/Windows/macOS) since 4.0.
- Built-in script editor support is minimal; external IDE (VS, Rider, VS Code) expected.
- **Hot reload caveat**: "State is currently not saved and restored when hot-reloading, with the exception of exported variables." *(c_sharp_basics.html §Current gotchas)*
- NuGet packages usable via `.csproj`.

#### 1.3 GDExtension

Native shared libraries loaded at runtime without recompiling the engine. Three pieces: `gdextension_interface.h` (C functions for engine↔extension), `extension_api.json` (exposed engine API), and the `*.gdextension` loader file. *(docs: `tutorials/scripting/gdextension/what_is_gdextension.html`)*

- The `.gdextension` file's `reloadable` flag reloads the extension on recompilation (godot-cpp binding, Godot 4.2+, dev/debug oriented); `compatibility_minimum`/`maximum` pin engine version ranges. *(docs: `tutorials/scripting/gdextension/gdextension_file.html` §Configuration)*

#### 1.4 Tool scripts

`@tool` runs the same script inside the editor, gated by `Engine.is_editor_hint()`. Caveats documented: the editor "does not include protections for potential misuse"; other nodes' scripts must also be `@tool` to interact; modifications have no undo/redo. `EditorScript` (with `_run()`) provides one-off editor scripts; C# EditorScripts exist but cannot run from the built-in script editor. *(docs: `tutorials/plugins/running_code_in_the_editor.html`)*

#### 1.5 No sandboxing

Godot scripts have full host access: `OS.execute` / `OS.create_process` spawn processes, `OS.set_environment` mutates the environment, `FileAccess`/`DirAccess` read/write the filesystem. *(docs: `classes/class_os.html`)* GDExtension is native code — unsandboxed by construction. `@tool` scripts run in the editor with no protections (see 1.4). `OS.is_sandboxed()` refers to the macOS/Linux app sandbox, not a script sandbox.

### 2. Craft's Scripting Surface (as built)

- **Tier 1: Lua 5.4** via mlua 0.12, `lua_class` binding, hooks `on_spawn`/`on_signal`/`on_tick`, direct mutation in a pre-pass. (ADR 0016; `crates/craft-lua/src/runtime.rs` — `LuaRuntime`, `bind_node` runtime.rs:225, `tick_pre_pass` runtime.rs:285, `dispatch_signal` runtime.rs:330)
- **Tier 2: JSON behaviors** — 3 primitives (state machine, on_tick, on_signal) over 9 closed verbs, evaluate-phase read-only → command buffer → apply-phase write. (ADR 0003 `Action` enum; `crates/craft-kernel/src/behavior.rs`; tick loop ADR 0003 §Decision)
- **Engine API in Lua**: `engine.emit/spawn/call_system/rng/get_node/log`. (runtime.rs:118–163 `install_engine_api`)
- **Sandbox**: VM created with only COROUTINE/TABLE/STRING/UTF8/MATH/PACKAGE stdlibs (runtime.rs:43–50); `sandbox_vm` nils `io`, `os`, `debug`, `dofile`, `loadfile`, `load`, `loadstring`, `package.loadlib`, `package.cpath`; `math.random`/`randomseed` nil'd outside determinism lock (runtime.rs:848–869). `require` resolves only inside the host-set `modules_dir` via a custom searcher (runtime.rs:539–581).
- **Determinism modes**: Development/Recording/Replay over three switches (rng/float/order). (`crates/craft-lua/src/determinism.rs:9–48`)
- **Hot reload**: scene diff/apply preserves NodeId and component state (`crates/craft-kernel/src/hot_reload.rs:63–230`); Lua class edits re-execute source and refresh the class def while existing instance `self` tables survive (`reload_class`, runtime.rs:193–211 — ADR 0016 §Hot Reload mitigation #1).

### 3. Graded Comparison

| # | Capability | Godot 4.7.2 | Craft | Grade |
|---|------------|-------------|-------|-------|
| 1 | Primary authoring language | GDScript (custom) | Lua 5.4 (standard, mlua 0.12) | **divergence** (ADR 0016 — standard language over custom) |
| 2 | Second managed language | C#/.NET with NuGet | none | **missing** (no ADR decision recorded) |
| 3 | Runtime native plugins | GDExtension shared libs, `reloadable` since 4.2 | `call_system` = Rust fns in same binary, compile-time registration, no FFI/ABI | **divergence** (ADR 0003 App B); authors cannot load native code at runtime |
| 4 | Class inheritance (`extends`, `super`, inner/abstract classes) | full | forbidden; table-prototype OOP in Lua, `lua_class` binding | **divergence** (ADR 0002, ADR 0016) |
| 5 | Lifecycle hooks | `_ready`/`_process`/`_physics_process`, `_on_*` signal handlers | `on_spawn`/`on_tick`/`on_signal` | **parity** (ADR 0016 §Godot Mapping) |
| 6 | Signal delivery timing | synchronous, same frame | queued, delivered next tick | **divergence** (ADR 0003 — determinism, no reentrancy) |
| 7 | Coroutines / await | `await` on signals/timers/coroutines | coroutine stdlib enabled (runtime.rs:44); `engine.wait_ticks`/`start_coroutine` spec'd in ADR 0016 but **not implemented** (no occurrence in `crates/`) | **partial** |
| 8 | Field-syntax property access | native (`node.position`) | `__index`/`__newindex` on `NodeRef` userdata | **parity** (ADR 0016 §Node Binding; `crates/craft-lua/src/bindings.rs`) |
| 9 | Script sandboxing | none (OS.execute, FileAccess, native GDExtension) | sandboxed VM: no io/os/debug/load/loadlib; require restricted to `modules_dir` | **divergence** (ADR 0016 §Sandboxing) — Craft-only capability for agent-authored code |
| 10 | Script resource limits (memory/instruction caps) | n/a (no sandbox) | spec'd (`lua_setmemorylimit`, `lua_sethook` per ADR 0016 §Sandboxing) but **not implemented** (no occurrence in `crates/`) | **missing** |
| 11 | Static typing / pre-run analysis | gradual typing + analyzer, warnings-as-errors | JSON tier is schema-validated (ADR 0005); Lua tier has no type checking or lint | **partial** |
| 12 | Hot reload: state preservation | editor reloads scripts; exported vars preserved (GDScript, C#) | NodeId + component state preserved via diff/apply (hot_reload.rs:63) | **parity** |
| 13 | Hot reload: script edit keeps instance state | GDScript yes (exported vars); C# exported vars only | `self` tables survive class reload (runtime.rs:193); coroutines/closures in `self` preserved | **parity** |
| 14 | Editor-time scripting | `@tool`, `EditorScript`, editor plugins | no tool-script concept; editor deferred to v2 | **divergence** (ADR 0017–0019) |
| 15 | Closed action vocabulary for machine authors | no analog (free-form GDScript) | 9 closed verbs, versioned vocabulary per schema version | **divergence** (ADR 0003 + App D) — Craft-only |
| 16 | Determinism controls | none documented | rng/float/order switches; Recording/Replay modes (determinism.rs:33) | **divergence** (ADR 0003/0016) — Craft-only; note float/order are scaffolded, not enforced (ADR 0016 L3 notes) |
| 17 | Package ecosystem | GDScript: closed; C#: NuGet | LuaRocks-style modules spec'd; implemented as `modules_dir` + TOML lockfile (`crates/craft-lua/src/modules.rs`; ADR 0016 L3 notes) | **partial** — no network package resolution as spec'd |
| 18 | Debugger integration | `breakpoint` keyword, editor debugger | none in engine crates | **missing** |

### 4. Where Godot's Model Gives Authors Capabilities Craft's Does Not

Honest gaps, engine-feature scope only:

1. **Inheritance with `super` calls and abstract classes** — large Godot codebases share behavior via class hierarchies; Craft forbids this by design (ADR 0002), pushing reuse into Lua mixins/composition, which is weaker tooling-wise (no analyzer-verified contracts).
2. **Gradual static typing with a static analyzer** — GDScript catches type errors before run; Craft's Lua tier has no type checking (JSON tier is schema-validated).
3. **`await` on signals/timers** — sequenced async gameplay code (cutscenes, timed flows) is one keyword in GDScript; Craft spec'd `engine.wait_ticks` but has no shipped equivalent.
4. **Runtime-loadable native plugins (GDExtension)** — perf-critical paths and third-party native libraries can be dropped in without recompiling the engine; Craft requires adding a Rust system and rebuilding.
5. **Editor-time scripting (`@tool`, EditorScript, EditorPlugin)** — level-design visualization and automation inside the editor; deferred to v2 (ADR 0017–0019).
6. **Full OS/filesystem access from scripts** — deliberately denied in Craft's sandbox (ADR 0016); a real authoring constraint for e.g. procedural asset tooling.
7. **C# performance tier + NuGet** — JIT-compiled managed code and a large package ecosystem; Craft has Lua + same-binary Rust only.
8. **Debugger/breakpoint tooling** — `breakpoint` keyword and editor debugger; Craft has no script debugging surface.

### 5. Craft-Side Spec-vs-Implementation Drift (observed while grading)

- ADR 0016 §Engine API specs `engine.wait_ticks` / `engine.start_coroutine`; not present in `crates/craft-lua` (grep across `crates/` finds only the stdlib flag).
- ADR 0016 §Sandboxing specs memory (`lua_setmemorylimit`) and instruction (`lua_sethook`) caps; not implemented in `crates/craft-lua`.
- `float` and `order` determinism switches are scaffolded types; only `rng` is auto-imposed in Recording mode (ADR 0016 L3 Implementation Notes; determinism.rs).
- `engine.log` writes via `eprintln!` (runtime.rs:972–977), which conflicts with ADR 0008's "never print to stderr" constraint.
- ADR 0016's LuaRocks CLI was replaced by a workspace-relative searcher + TOML lockfile (documented in ADR 0016 L3 notes; `crates/craft-lua/src/modules.rs`).

### 6. Sources

**Godot (baseline 4.7.2-stable, published 2026-08-18)**:

- GitHub Releases API: `https://api.github.com/repos/godotengine/godot/releases/latest` (tag `4.7.2-stable`, `published_at: 2026-08-18T16:12:28Z`)
- GDScript basics: https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html
- C# basics: https://docs.godotengine.org/en/stable/tutorials/scripting/c_sharp/c_sharp_basics.html
- What is GDExtension: https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/what_is_gdextension.html
- .gdextension file: https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/gdextension_file.html
- Running code in the editor (`@tool`, EditorScript): https://docs.godotengine.org/en/stable/tutorials/plugins/running_code_in_the_editor.html
- OS class reference: https://docs.godotengine.org/en/stable/classes/class_os.html

**Craft (branch `research/scripting`)**:

- `docs/adr/0003-behavior-runtime.md` (tick loop, 9 verbs, signal timing, system registry, verb versioning)
- `docs/adr/0016-lua-scripting.md` (two-tier model, sandboxing table, hot-reload semantics, L3 notes)
- `crates/craft-lua/src/runtime.rs` (VM setup, sandbox_vm, engine API, reload_class)
- `crates/craft-lua/src/determinism.rs` (DeterminismSwitches/Mode)
- `crates/craft-lua/src/modules.rs`, `crates/craft-lua/src/bindings.rs`
- `crates/craft-kernel/src/behavior.rs`, `crates/craft-kernel/src/hot_reload.rs`, `crates/craft-kernel/src/hook.rs`
- `docs/AGENT_GUIDE.md` (§4 verbs, §6 signals, §8 Lua)
- `games/platformer/scripts/player_controller.lua`, `games/dungeon_crawler/scripts/enemy.lua`

## 10. Navigation & pathfinding

> Graded evidence preserved from ticket [#20 — Gap analysis: navigation](https://github.com/0xnicholas/Craft/issues/20) (findings: `docs/research/gap-navigation.md`, branch `research/navigation`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Godot inventory

#### 1. NavigationServer2D / NavigationServer3D

Singleton servers with equivalent 2D/3D functionality. All internal objects (maps, regions, agents, links, obstacles) are referenced by `RID`. The server queues mutating API calls and applies them in a synchronization step at the end of the physics frame; the API is thread-safe and a threadpool backs some work (e.g. avoidance). Path queries are direct calls, e.g. `NavigationServer3D.map_get_path(map, start, target, optimize)`. `NavigationServer2D` and `NavigationServer3D` are functionally equivalent for their dimension.

- Sources: [Using NavigationServers](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationservers.html), [class NavigationServer2D](https://docs.godotengine.org/en/stable/classes/class_navigationserver2d.html), [class NavigationServer3D](https://docs.godotengine.org/en/stable/classes/class_navigationserver3d.html); source: `servers/navigation_2d/`, `servers/navigation_3d/`, `servers/nav_heap.h`, `modules/navigation_2d/`, `modules/navigation_3d/` @ `4.7.2-stable`.

#### 2. Navigation meshes and baking

Resources: `NavigationPolygon` (2D) / `NavigationMesh` (3D). Nodes: `NavigationRegion2D` / `NavigationRegion3D` combine parse+bake+region-update into one call; `bake_navigation_polygon(on_thread)` works at runtime. The server exposes the finer-grained pipeline: `parse_source_geometry_data()` → `bake_from_source_geometry_data()` / `_async()`, with reusable serializable `NavigationMeshSourceGeometryData2D/3D`. Source geometry can be parsed from visual meshes, physics colliders (filtered by `collision_mask`), or supplied procedurally. Bake controls include `cell_size` (rasterization), `agent_radius` (mesh shrink for agent size), `parsed_geometry_type`, `source_geometry_mode`; large worlds use baking bounds plus `border_size` for aligned chunks. A navmesh describes only the traversable surface for an agent's center — visuals and physics shapes are ignored unless baked in.

- Sources: [Using NavigationMeshes](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationmeshes.html), [class NavigationPolygon](https://docs.godotengine.org/en/stable/classes/class_navigationpolygon.html), [class NavigationMesh](https://docs.godotengine.org/en/stable/classes/class_navigationmesh.html), [class NavigationRegion2D](https://docs.godotengine.org/en/stable/classes/class_navigationregion2d.html), [class NavigationRegion3D](https://docs.godotengine.org/en/stable/classes/class_navigationregion3d.html).

#### 3. Agents and path following

`NavigationAgent2D` / `NavigationAgent3D` are optional convenience nodes wrapping the server API. Setting `target_position` triggers a path query on the agent's navigation map; `get_next_path_position()` advances an internal path index, gated by `path_desired_distance`, `target_desired_distance`, `path_max_distance`. Tunables: `navigation_layers` bitmask, `pathfinding_algorithm`, `path_postprocessing`, `path_metadata_flags`, `simplify_path`/`simplify_epsilon`. The navigation system never moves the parent node — movement is user code.

- Sources: [Using NavigationAgents](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationagents.html), [class NavigationAgent2D](https://docs.godotengine.org/en/stable/classes/class_navigationagent2d.html), [class NavigationAgent3D](https://docs.godotengine.org/en/stable/classes/class_navigationagent3d.html).

#### 4. Avoidance (RVO)

Opt-in per agent via `avoidance_enabled`; the agent reports `velocity` each physics frame and receives a safe velocity via the `velocity_computed` signal, computed just before PhysicsServer synchronization. Properties: `radius`, `height` (3D), `neighbor_distance`, `max_neighbors`, `time_horizon_agents`/`time_horizon_obstacles`, `max_speed`, `use_3d_avoidance` (separate 2D/3D simulations), `avoidance_layers`/`avoidance_mask`, `avoidance_priority`. `NavigationObstacle2D/3D` add environment constraints. Avoidance has no knowledge of navmeshes or physics collision and does not affect pathfinding.

- Sources: [Using NavigationAgents §Avoidance](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationagents.html), [Using NavigationObstacles](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationobstacles.html), [class NavigationObstacle2D](https://docs.godotengine.org/en/stable/classes/class_navigationobstacle2d.html).

#### 5. Navigation links

`NavigationLink2D` / `NavigationLink3D` connect arbitrary positions across a navigation map (teleports, jumps, off-mesh traversal).

- Sources: [Using NavigationLinks](https://docs.godotengine.org/en/stable/tutorials/navigation/navigation_using_navigationlinks.html), [class NavigationLink2D](https://docs.godotengine.org/en/stable/classes/class_navigationlink2d.html), [class NavigationLink3D](https://docs.godotengine.org/en/stable/classes/class_navigationlink3d.html).

#### 6. A* (server-independent)

`AStar2D` / `AStar3D`: manual point-graph A* — `add_point`/`connect_points` with per-point `weight_scale`, virtual `_compute_cost`/`_estimate_cost`, `allow_partial_path`. `AStarGrid2D`: grid-specialized variant with `region`/`cell_size`, solid cells and weight regions, four heuristics (Euclidean, Manhattan, Octile, Chebyshev), four diagonal modes, jump-point skipping, isometric cell shapes. These are plain `RefCounted` classes, independent of the NavigationServer.

- Sources: [class AStar2D](https://docs.godotengine.org/en/stable/classes/class_astar2d.html), [class AStar3D](https://docs.godotengine.org/en/stable/classes/class_astar3d.html), [class AStarGrid2D](https://docs.godotengine.org/en/stable/classes/class_astargrid2d.html).

### Craft status (verified in repo)

- **No navigation crate or system.** Workspace members are 15 `craft-*` crates, none navigation-related (`Cargo.toml`). The four registered `craft_system!` systems are Particle, Animation, Physics, Audio (`crates/craft-particles/src/lib.rs:3`, `crates/craft-animation/src/lib.rs:3`, `crates/craft-physics/src/lib.rs:3`, `crates/craft-audio/src/lib.rs:45`). A repo-wide search for `navigation|navmesh|pathfind|astar|avoidance|waypoint` matches only docs, no engine code or game code.
- **Deliberate v1 exclusion.** `docs/ARCHITECTURE.md:159` maps Godot's `servers/physics*/`, `audio/`, `navigation*/`, `xr/` to "v1 excluded (non-goals)" (~60K lines → 0). (Note: physics and audio have since been added as crates; navigation has not.)
- **No navigation verb.** The closed 9-verb action set (`SetState`, `Emit`, `Destroy`, `Spawn`, `If`, `Move`, `Animate`, `Log`, `CallSystem`; `docs/adr/0003-behavior-runtime.md:55`) contains nothing spatial; `move` adds a numeric delta to any component, it is not movement-along-a-path (`docs/adr/0003-behavior-runtime.md:97-103`).
- **Designed hook, unimplemented.** The PRD shows `call_system` invoking a hypothetical `pathfinding::astar` system as the escape hatch when the verb set is insufficient (`docs/superpowers/specs/2026-07-09-craft-prd.md:320-325`; also `docs/SCHEMA.md:197`, `docs/AGENT_GUIDE.md:112`, and ADR 0016's `pathfinding::direction_to`/`pathfinding::astar` examples at `docs/adr/0016-lua-scripting.md:58,132`). The `SystemRegistry`/`craft_system!` mechanism exists (`crates/craft-kernel/src/system.rs:42-64`), but no such system is registered.
- **Script-level escape hatch, unused.** ADR 0016 sanctions LuaRocks packages via `require` (a `lua-pathfinding` example appears at `docs/adr/0016-lua-scripting.md:301,330`); no game in `games/` uses it.
- **Games hand-roll movement.**
  - `games/tower_defense/scene.json:40` — enemies spawn with a fixed `velocity: [-0.3, 0.0]` vector and drift in a straight line until lifetime expiry; no steering toward a goal.
  - `games/dungeon_crawler/scripts/enemy.lua:3-22` — greedy axis-aligned chase: pick the axis with the larger delta to the player, set a unit velocity; no obstacle handling.
  - `games/platformer/scripts/player_controller.lua` — input-driven player movement only.

### Grading

| Godot feature | Grade | Basis |
|---|---|---|
| NavigationServer2D/3D (maps, regions, RID API, sync model) | **divergence** | Deliberate v1 non-goal: `docs/ARCHITECTURE.md:159` excludes `servers/navigation*/`. No equivalent server or RID layer exists. |
| Navmesh resources + baking (`NavigationPolygon/Mesh`, regions, source-geometry pipeline) | **missing** | No navmesh representation anywhere in `crates/`. Baking is editor-adjacent, and editor work is v2-scoped (ADR 0017-0019). |
| NavigationAgent nodes + path following | **missing** | No agent node type; games set `velocity` components directly (paths above). |
| RVO avoidance + `NavigationObstacle` | **missing** | Nothing in `crates/`; also architecturally tense with Craft's single-threaded determinism (ADR 0015) and per-tick hash replay (ADR 0006), where Godot computes avoidance on a threadpool. |
| NavigationLink2D/3D | **missing** | No link concept in scene schema. |
| A* (`AStar2D/3D`, `AStarGrid2D`) | **missing** (with designed seam) | No pathfinding algorithm in engine or games. The `call_system` verb + `craft_system!` registry is the documented integration point (`docs/adr/0003-behavior-runtime.md:236`; PRD §`pathfinding::astar` example), so the surface for adding one exists without touching the 9-verb closed set. |

Every graded item is absent from Craft's engine feature surface; the only **divergence** (a documented deliberate difference rather than an unbuilt gap) is the v1 exclusion of Godot's navigation servers themselves.

### Unverified / caveats

- Godot's internal avoidance solver is described by docs as "RVO avoidance"; the docs pages fetched do not name the underlying library, so no specific library (e.g. RVO2) is claimed here.
- `navigation_introduction` / `navigation_overview_2d|3d` doc URLs 404 on the stable track; the tutorial content was sourced from the topic pages listed above instead.

## 11. Input system + platform export

> Graded evidence preserved from ticket [#21 — Gap Analysis: Input & Export Pipeline](https://github.com/0xnicholas/Craft/issues/21) (findings: `docs/research/gap-input-export.md`, branch `research/input-export`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Godot input inventory

#### 1.1 InputEvent class hierarchy

`InputEvent` is an abstract `Resource`-derived base class carrying a `device` ID
(with reserved IDs for keyboard `16`, mouse `32`, and emulation `-1`).
Direct subclasses: `InputEventAction`, `InputEventFromWindow`, `InputEventJoypadButton`,
`InputEventJoypadMotion`, `InputEventMIDI`, `InputEventShortcut`. The concrete set used
in games includes `InputEventKey` (with echo handling), `InputEventMouseButton`,
`InputEventMouseMotion` (accumulable via `accumulate()`), `InputEventScreenTouch`,
`InputEventScreenDrag`, and gesture events (`InputEventMagnifyGesture`,
`InputEventPanGesture`). Events support action queries (`is_action`,
`is_action_pressed/released`, `get_action_strength` in 0..1).
Source: [class_inputevent.html](https://docs.godotengine.org/en/stable/classes/class_inputevent.html)

#### 1.2 Event propagation

Events originate in the platform `DisplayServer`, are fed to the root `Window`, and
each `Viewport` dispatches in a fixed order: embedded-window management →
`Node._input()` → GUI `Control._gui_input()` → `Node._shortcut_input()` →
`Node._unhandled_key_input()` → `Node._unhandled_input()` → physics object picking
(`CollisionObject._input_event()`). Any stage can consume the event
(`Viewport.set_input_as_handled()`, `Control.accept_event()`). Game code can also
inject synthetic events with `Input.parse_input_event()`.
Source: [tutorials/inputs/inputevent.html](https://docs.godotengine.org/en/stable/tutorials/inputs/inputevent.html)

#### 1.3 InputMap (action abstraction)

`InputMap` is a singleton managing named **actions** — groupings of zero or more
`InputEvent`s (e.g. `ui_left` = arrow key + joypad D-pad left). Actions are configured
in Project Settings → Input Map, persisted in `project.godot`, and fully editable at
runtime: `add_action(action, deadzone=0.2)`, `action_add_event`, `action_erase_events`,
`action_set_deadzone`, `has_action`, `get_actions`, `load_from_project_settings`.
This is what enables device-agnostic game code and user key-rebinding.
Source: [class_inputmap.html](https://docs.godotengine.org/en/stable/classes/class_inputmap.html)

#### 1.4 Input singleton (polling + device APIs)

Polled state API: `is_action_pressed`, `is_action_just_pressed/released`,
`get_action_strength` (analog 0..1, deadzone-aware), `get_action_raw_strength`,
`get_axis(neg, pos)`, `get_vector(neg_x, pos_x, neg_y, pos_y, deadzone)` returning a
length-1-clamped `Vector2` with circular deadzone. Programmatic simulation:
`action_press`/`action_release`. Joypad: `get_connected_joypads`,
`joy_connection_changed` signal, `is_joy_button_pressed`, `get_joy_axis`, SDL2
controller-database naming/GUID mapping (`add_joy_mapping`), dual-motor vibration
(`start_joy_vibration`), LED control, and experimental motion sensors
(gyro/accelerometer with calibration). Mouse: 5 `MouseMode`s (visible/hidden/
captured/confined/confined-hidden), 18 cursor shapes, custom cursors, `warp_mouse`.
Touch: `emulate_mouse_from_touch` / `emulate_touch_from_mouse` cross-emulation.
Mobile sensors: `get_accelerometer`/`get_gravity`/`get_gyroscope`/`get_magnetometer`
(Android/iOS only), `vibrate_handheld` (Android/iOS/Web).
Source: [class_input.html](https://docs.godotengine.org/en/stable/classes/class_input.html)

#### 1.5 Joypad & touch event types

Joypad input arrives as `InputEventJoypadButton` and `InputEventJoypadMotion`
(axis value −1..1); touch as `InputEventScreenTouch`/`InputEventScreenDrag` with
multi-touch index support. Both families are bindable to InputMap actions, so game
logic is device-agnostic by construction.
Source: [tutorials/inputs/inputevent.html](https://docs.godotengine.org/en/stable/tutorials/inputs/inputevent.html),
[class_inputevent.html](https://docs.godotengine.org/en/stable/classes/class_inputevent.html)

### 2. Godot export pipeline inventory

- **Export presets** per platform, stored in `export_presets.cfg` (committable) plus
  `.godot/export_credentials.cfg` for secrets. Each preset targets one platform.
- **Export templates**: prebuilt engine binaries per platform, version-matched to the
  editor and installed separately (TPZ); exporting binds the project PCK/ZIP into/with
  a template.
- **First-party export platforms** (output formats from the stable docs):
  Windows (`.exe`), macOS (`.app`/`.zip`/`.dmg`), Linux (any ext), Web/HTML5 (`.zip`),
  Android (`.apk`), iOS (`.zip`). Mobile export additionally deals with per-platform
  SDK setup and texture-compression differences (ETC1/ETC2 vs S3TC/BC).
- **CLI export**: `--export-release "<preset>" <out>`, `--export-debug`,
  `--export-pack` (PCK-only, one pack reusable across executables).
  Source: [tutorials/export/exporting_projects.html](https://docs.godotengine.org/en/stable/tutorials/export/exporting_projects.html)
- **Consoles**: the only console Godot officially supports is Steam Deck (via the
  Linux export templates). Switch/PlayStation/Xbox require console SDKs and are served
  by third-party porting houses and middleware (e.g. W4 Games official ports for
  Switch, Xbox Series X/S, PS5) — no engine vendor may legally redistribute console
  templates without licensed-developer proof.
  Source: [tutorials/platform/consoles.html](https://docs.godotengine.org/en/stable/tutorials/platform/consoles.html)

### 3. Craft input inventory (verified in worktree)

- **Single polled struct**: `InputState { direction: [f64; 2], action: bool }` at
  `crates/craft-kernel/src/engine.rs:19-22`. Setters `set_input_direction` /
  `set_input_action` (`engine.rs:137-143`). That is the entire input abstraction.
- **Scene exposure**: each `tick()` copies the struct into a synthetic `__input` node's
  `direction`/`action` components (`engine.rs:145-167`, called at `engine.rs:170`).
  Lua reads it via `engine.get_node("__input")` then field access
  (`games/platformer/scripts/player_controller.lua:15-28`); the node is declared in
  `games/*/scene.json` (e.g. `games/platformer/scene.json:6`).
- **Key capture is hardcoded in the window process**: `crates/craft-gpu/src/lib.rs:496-516`
  maps W/S/A/D + arrow keys to `direction` and Space to `action` via winit
  `WindowEvent::KeyboardInput` (physical keycodes). No mouse, no joypad, no touch,
  no text input, no remapping table anywhere.
- **No input events**: there is no event type, queue, or `_input`-style callback.
  `crates/craft-terminal` contains no input handling (grep for `input|key`: no matches).
  Input is sampled state read once per redraw tick (`craft-gpu/src/lib.rs:517-526`).
- **Bridge/SDK**: `crates/craft-bridge` is currently a pure-Rust JSON-RPC dispatcher;
  its `Cargo.toml` has **no napi-rs dependency** and no `#[napi]` functions exist in
  any `.rs` file (workspace-wide grep). The NAPI layer is specced in
  `docs/adr/0007-bridge-layer.md` (sync NAPI, in-process, V8 main thread) but the
  binding crate is not implemented in-tree; `sdk/src/` contains only `generated.ts`
  (action-verb types) and no `package.json` exists in the repo. No `set_input`
  method is exposed over the bridge — the only live input path is the craft-gpu window.

### 4. Craft platform/export story (verified in worktree)

- **Hosts**: (a) Node.js agent host via sync NAPI — designed in `docs/adr/0007-bridge-layer.md`
  ("Godot has no equivalent... Craft needs a bridge because the agent runs in Node.js"),
  binding not yet implemented (§3); (b) desktop game window — wgpu + winit 2D renderer,
  `crates/craft-gpu/src/lib.rs` (`spawn_game_window`), launched as a **subprocess**
  from the editor with a temp scene JSON (`docs/adr/0020-gpu-subprocess-boundary.md`);
  (c) ANSI terminal renderer (`crates/craft-terminal`); (d) egui editor embeds an engine
  instance directly (v2, `docs/adr/0017-editor-architecture.md`).
- **Export targets: none.** No export presets, templates, packaging, or CLI export
  anywhere in the repo. `ROADMAP.md:99` explicitly defers "Mobile/web platform
  targets" to v3+ with note "Requires platform abstraction layer". No console mention
  anywhere. The engine is single-threaded by invariant (`docs/adr/0007-bridge-layer.md`
  "no async reentrancy"; AGENTS.md constraint table, ADR 0015), and the v1 tick budget
  is ≤8 ms (`docs/adr/0015-performance-budgets.md`).

### 5. Comparison and grades

Legend: **missing** (absent in Craft, no stated alternative) · **partial** ·
**parity** · **divergence** (deliberate difference, ADR cited).

#### Input

| # | Godot capability | Craft status | Grade | Evidence |
|---|------------------|--------------|-------|----------|
| 1 | InputEvent type hierarchy (key/mouse/joy/touch/gesture/MIDI) | No event types at all | **divergence** (polled state, not events) | Craft: `engine.rs:19-22`; Godot: class_inputevent.html |
| 2 | Event propagation + consumption (`_input`, `_unhandled_input`, GUI routing) | None; single `__input` node written per tick | **missing** | Craft: `engine.rs:145-167`; Godot: inputevent.html §propagation |
| 3 | InputMap named actions + runtime remapping + deadzones | None; one fixed 2-axis + 1-button semantic | **missing** | Craft: `engine.rs:19`; Godot: class_inputmap.html |
| 4 | Keyboard | WASD/arrows + Space, hardcoded physical keycodes | **partial** | Craft: `craft-gpu/src/lib.rs:496-516` |
| 5 | Mouse (buttons, motion, modes, cursors) | None | **missing** | Craft: no mouse in `craft-gpu` grep; Godot: class_input.html |
| 6 | Joypad (buttons, axes, vibration, hotplug, SDL mapping) | None | **missing** | Godot: class_input.html, class_inputevent.html |
| 7 | Touch / multi-touch / emulation | None | **missing** | Godot: class_inputevent.html (ScreenTouch/Drag), class_input.html |
| 8 | Analog strength / `get_vector` with circular deadzone | `direction: [f64;2]` carries analog magnitude, but producer is digital keys (values only −1/0/1) | **partial** | Craft: `engine.rs:20`, `craft-gpu/src/lib.rs:521-523`; Godot: class_input.html `get_vector` |
| 9 | Synthetic input injection (`parse_input_event`, `action_press`) | `set_input_direction`/`set_input_action` setters serve the same role for tests/replay | **partial** (no bridge exposure) | Craft: `engine.rs:137-143`; Godot: class_input.html |
| 10 | Deterministic polled-state input suitable for replay hashing | `InputState` is plain data, copied into the scene each tick — hash/replay friendly | **divergence** (deliberate; determinism is a v1 invariant) | ADR 0006 (`docs/adr/0006-replay-system.md`), ADR 0003 (`docs/adr/0003-behavior-runtime.md`) |

#### Export / platform

| # | Godot capability | Craft status | Grade | Evidence |
|---|------------------|--------------|-------|----------|
| 11 | Export presets + version-matched export templates | None | **missing** | Craft: no matches repo-wide; Godot: exporting_projects.html |
| 12 | Desktop export (Windows/macOS/Linux binaries) | Runs as dev-launched wgpu/winit window or subprocess; no packaged export | **partial** | Craft: `crates/craft-gpu/src/main.rs`, ADR 0020; Godot: exporting_projects.html |
| 13 | Mobile export (Android `.apk`, iOS) | None; deferred v3+ | **missing** (explicitly deferred) | `ROADMAP.md:99`; Godot: exporting_projects.html |
| 14 | Web export (HTML5/WASM) | None; deferred v3+ | **missing** (explicitly deferred) | `ROADMAP.md:99`; Godot: exporting_projects.html |
| 15 | Console export | None. (Godot itself: only Steam Deck via Linux templates; rest via third parties) | **missing** | Godot: consoles.html |
| 16 | CLI export (`--export-release`/`--export-pack`) | None | **missing** | Godot: exporting_projects.html |
| 17 | Node.js in-process agent host (sync NAPI) | Specced; binding crate not implemented (no napi-rs dep, no `#[napi]` in tree) | **divergence** (deliberate platform choice) — **partial** implementation | ADR 0007 (`docs/adr/0007-bridge-layer.md`); `crates/craft-bridge/Cargo.toml` |
| 18 | GPU window as replayable subprocess (file-based handoff) | Implemented; no Godot equivalent (Godot game is same-process or exported binary) | **divergence** | ADR 0020 (`docs/adr/0020-gpu-subprocess-boundary.md`) |

### 6. Unverified / caveats

- Godot-side facts are from stable docs pages, not from reading Godot source
  (`_references/godot-master/` absent locally); page contents were fetched 2026-08-24
  and reflect the 4.7 stable branch.
- The InputEvent direct-subclass list (§1.1) is from the class reference "Inherited By"
  line; intermediate classes (`InputEventWithModifiers`, `InputEventMouse`, etc.)
  exist between the base and concrete key/mouse/touch events — exact chain not
  re-verified against source.
- Whether the NAPI binding lives in an unpublished package outside this repo cannot
  be determined from the worktree; in-tree, `craft-bridge` has no NAPI code and there
  is no `package.json` anywhere in the repo.
- Godot docs pages on export templates describe TPZ installation; the dedicated
  `export_templates.html` URL 404'd, so template details are cited from
  `exporting_projects.html` only.

## 12. Networking & multiplayer

> Graded evidence preserved from ticket [#22 — Gap analysis: high-level multiplayer, ENet/WebSocket transports, scene replication](https://github.com/0xnicholas/Craft/issues/22) (findings: `docs/research/gap-networking.md`, branch `research/networking`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Godot inventory (4.7.2-stable)

#### High-level multiplayer API
- `MultiplayerAPI` is the high-level interface; `SceneMultiplayer` is the default implementation, managed by SceneTree. Per-branch override via `SceneTree.set_multiplayer()` lets one process run server and client simultaneously ([class_multiplayerapi.html](https://docs.godotengine.org/en/stable/classes/class_multiplayerapi.html), [high_level_multiplayer.html](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html)).
- Peer identity: server is always peer ID 1; clients get unique random positive IDs. Connection lifecycle via signals `peer_connected`, `peer_disconnected`, `connected_to_server`, `connection_failed`, `server_disconnected` ([high_level_multiplayer.html](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html)).
- Built-in authentication handshake: `auth_callback`, `send_auth`, `complete_auth`, `peer_authenticating` signal, `auth_timeout` ([high_level_multiplayer.html](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html) §Authentication).
- Authority model: per-node multiplayer authority, server by default, changeable via `Node.set_multiplayer_authority()`; gates `"authority"` RPCs ([high_level_multiplayer.html](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html), [class_node.html](https://docs.godotengine.org/en/stable/classes/class_node.html)).
- Extensibility: `MultiplayerAPIExtension` allows replacing/extending the default implementation ([class_multiplayerapiextension.html](https://docs.godotengine.org/en/stable/classes/class_multiplayerapiextension.html)).

#### RPC system
- `@rpc(mode, sync, transfer_mode, transfer_channel)` annotation; called via `Callable.rpc()` / `rpc_id()`. Modes `authority`/`any_peer`; sync `call_local`/`call_remote`; transfer modes `unreliable`/`unreliable_ordered`/`reliable`; multi-channel support. RPC signature is checksummed across all `@rpc` functions in a script and must match on both peers ([high_level_multiplayer.html](https://docs.godotengine.org/en/stable/tutorials/networking/high_level_multiplayer.html)).

#### Transports (mid-level `MultiplayerPeer`)
- `MultiplayerPeer` (extends `PacketPeer`) abstracts network layers; server/client created by `create_server`/`create_client` on the concrete peer, then assigned to `multiplayer.multiplayer_peer` ([class_multiplayerpeer.html](https://docs.godotengine.org/en/stable/classes/class_multiplayerpeer.html), source `scene/main/multiplayer_peer.h` at 4.7.2-stable).
- `ENetMultiplayerPeer`: default transport, modified ENet over UDP with full IPv6; per-channel reliability/ordering, optional compression and DTLS encryption ([class_enetmultiplayerpeer.html](https://docs.godotengine.org/en/stable/classes/class_enetmultiplayerpeer.html), source `modules/enet/enet_multiplayer_peer.{h,cpp}` at 4.7.2-stable).
- `WebSocketMultiplayerPeer`: WebSocket (TCP) transport, TLS support, required for web exports ([class_websocketmultiplayerpeer.html](https://docs.godotengine.org/en/stable/classes/class_websocketmultiplayerpeer.html), source `modules/websocket/websocket_multiplayer_peer.{h,cpp}` at 4.7.2-stable).
- `WebRTCMultiplayerPeer` also ships (`modules/webrtc/`), and `OfflineMultiplayerPeer` is the no-network default ([class_webrtcmultiplayerpeer.html](https://docs.godotengine.org/en/stable/classes/class_webrtcmultiplayerpeer.html), [class_offlinemultiplayerpeer.html](https://docs.godotengine.org/en/stable/classes/class_offlinemultiplayerpeer.html)).
- Low-level stack underneath: `PacketPeerUDP`, `TCPServer`, `StreamPeerTCP`, plus HTTP/TLS ([class_packetpeerudp.html](https://docs.godotengine.org/en/stable/classes/class_packetpeerudp.html), [class_tcpserver.html](https://docs.godotengine.org/en/stable/classes/class_tcpserver.html), [class_streampeertcp.html](https://docs.godotengine.org/en/stable/classes/class_streampeertcp.html)).

#### Scene replication
- `MultiplayerSpawner`: replicates spawn/despawn of nodes under `spawn_path` from authority to peers; configurable spawnable scene list (`add_spawnable_scene`), custom `spawn(data)` with `spawn_function` run on all peers, `spawn_limit`, `spawned`/`despawned` signals ([class_multiplayerspawner.html](https://docs.godotengine.org/en/stable/classes/class_multiplayerspawner.html)).
- `MultiplayerSynchronizer`: pushes configured properties from authority to peers; `replication_interval` / `delta_interval` throttling, delta synchronization on change, per-peer visibility (`set_visibility_for`, `add_visibility_filter`, `public_visibility`), `synchronized`/`delta_synchronized` signals. Object-type properties and instance IDs/RIDs cannot sync ([class_multiplayersynchronizer.html](https://docs.godotengine.org/en/stable/classes/class_multiplayersynchronizer.html)).
- `SceneReplicationConfig`: resource listing which properties sync, with per-property replication mode (`ALWAYS`/`ON_CHANGE`/`NEVER`) and spawn/watch flags ([class_scenereplicationconfig.html](https://docs.godotengine.org/en/stable/classes/class_scenereplicationconfig.html)).
- Source layout at 4.7.2-stable: the entire stack is a module — `modules/multiplayer/` (`scene_multiplayer`, `multiplayer_spawner`, `multiplayer_synchronizer`, `scene_replication_config`, plus `scene_cache_interface`/`scene_replication_interface`/`scene_rpc_interface` internals); base classes in `scene/main/multiplayer_api.h`, `scene/main/multiplayer_peer.h`.
- Editor tooling: `modules/multiplayer/multiplayer_debugger.{h,cpp}` + `editor/` profiler for replication/RPC traffic (engine module, not a separate plugin).

### Craft inventory (this worktree)

- **No networking code exists.** Repo-wide grep of `crates/` for `network|multiplayer|websocket|ENet|replicat|transport|socket|tcp|udp|peer`: zero matches in production source (matches only in docs). No crate for networking: `crates/` contains animation, assets, audio, bridge, editor, eval, gpu, kernel, lua, macros, particles, physics, replay, schema, terminal.
- **Explicitly deferred**: `ROADMAP.md:97` — "Multiplayer/networking | v3+ | Replication protocol, server-client split".
- **Out of scope for v1 by design**: `docs/specs/2026-07-09-craft-prd.md:72` (non-goals list: "Multiplayer, networking, replication") and `:829` ("Multiplayer / network replay — Out of scope v1. Determinism layer is the precondition").
- **Bridge is not a network transport**: sync NAPI + in-process JSON-RPC, "no IPC, no network round-trips" (`docs/adr/0007-bridge-layer.md:14`); WebSocket JSON-RPC listed only as a future "Remote agent" option (`0007-bridge-layer.md:91`). `crates/craft-bridge/src/` is a single `lib.rs`; no transport abstraction trait exists in code (see note below).
- **Adjacent assets that bound any future design** (not multiplayer features): deterministic replay with per-tick state hash (`docs/adr/0006-replay-system.md`; `crates/craft-replay/`), single-threaded tick (`docs/adr/0015-*.md`, AGENTS.md constraints table), next-tick signal delivery (`docs/adr/0003-behavior-runtime.md`), closed set of 9 action verbs (ADR 0003).

### Graded comparison

| # | Feature | Grade | Notes |
|---|---------|-------|-------|
| 1 | High-level multiplayer API (MultiplayerAPI/SceneMultiplayer, peer IDs, connection signals) | **missing** | Deferred v3+ (`ROADMAP.md:97`) |
| 2 | Per-branch MultiplayerAPI override (server+client in one process) | **missing** | Craft is single-engine, single-process (ADR 0015) |
| 3 | RPC system (@rpc, modes, transfer modes, channels, checksum) | **missing** | — |
| 4 | ENet transport (UDP, channels, compression, DTLS) | **missing** | — |
| 5 | WebSocket transport (TCP/TLS) | **missing** | — |
| 6 | WebRTC transport | **missing** | — |
| 7 | OfflineMultiplayerPeer no-op peer | **missing** | — |
| 8 | Low-level networking (UDP/TCP packet peers, HTTP, TLS) | **missing** | No socket usage anywhere in `crates/` |
| 9 | MultiplayerSpawner (scene spawn/despawn replication) | **missing** | Craft `spawn` verb is local-only (`crates/craft-kernel/src/behavior.rs`) |
| 10 | MultiplayerSynchronizer (property push, delta sync, intervals) | **missing** | — |
| 11 | SceneReplicationConfig (per-property modes, spawn/watch) | **missing** | — |
| 12 | Per-peer visibility / interest management | **missing** | — |
| 13 | Multiplayer authority model (`set_multiplayer_authority`) | **missing** | — |
| 14 | Authentication handshake (auth_callback/complete_auth) | **missing** | — |
| 15 | MultiplayerAPIExtension (custom implementations) | **missing** | — |
| 16 | Multiplayer profiler/debugger | **missing** | — |
| 17 | Agent/engine bridge transport over the network | **divergence** | Deliberate: sync NAPI in-process, WebSocket explicitly rejected for v1 (ADR 0007, `docs/adr/0007-bridge-layer.md:86-94`) |

### Gap bound

- **Scope**: 16 of 17 graded items are entirely absent — no partial implementations. The only divergence is the deliberate sync-NAPI bridge (ADR 0007).
- **Deferral is documented, not accidental**: PRD non-goals (`docs/specs/2026-07-09-craft-prd.md:72,829`), ROADMAP v3+ row (`ROADMAP.md:97`).
- **Precondition already in place**: PRD §829 names the determinism layer as the precondition for network replay; Craft ships deterministic replay with per-tick hash equality (ADR 0006, `crates/craft-replay/`).
- **Doc inconsistency worth flagging**: `AGENTS.md:50` claims a "transport trait exists for v2" in the bridge; no such trait exists in `crates/craft-bridge/src/` (single `lib.rs`, zero `transport`/`socket`/`peer` symbols in `crates/`).

## 13. Editor & debugging tools

> Graded evidence preserved from ticket [#23 — Gap analysis: Godot editor surface vs craft-editor (E1–E4)](https://github.com/0xnicholas/Craft/issues/23) (findings: `docs/research/gap-editor-tools.md`, branch `research/editor-tools`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Inventory and grades

#### 1. Scene dock → Scene Tree panel — **partial**

Godot's Scene dock (`editor/docks/scene_tree_dock.cpp`, ref `4.7`) lists the active scene's nodes with per-scene tabs, instancing, and companion **Signals**/**Groups** docks (`editor/docks/signals_dock.cpp`, `groups_dock.cpp`) and a **History** dock ([First look at Godot's interface](https://docs.godotengine.org/en/stable/getting_started/introduction/first_look_at_the_editor.html)).

Craft's Scene Tree (`crates/craft-editor/src/panels/scene_tree.rs`) covers the core: filter box, select→inspector, context menu (Add Child / Duplicate / Rename / Attach Lua / Delete), drag-to-reparent, drag-Lua-attach (E1/E4). Missing relative to Godot: multiple open scene tabs, scene instancing UI, signals/groups companion docks. Signal wiring in Craft lives in behavior JSON (closed-set verbs, ADR 0003) rather than a node dock — **divergence** on that axis, not a gap.

#### 2. Inspector — **partial**

Godot's Inspector (`editor/inspector/`, `editor/docks/inspector_dock.cpp`) edits all exported properties with type-aware sub-editors, nested Resource editing, and per-property keyframe buttons feeding the Animation editor ([Animation introduction](https://docs.godotengine.org/en/stable/tutorials/animation/introduction.html)).

Craft's Inspector (`crates/craft-editor/src/panels/inspector.rs`, E1) is a component form editor over the property-bag model (ADR 0002): DragValues, text fields, checkboxes, transient badges, Add Component. It shows **live** engine values while running and file values when stopped (ADR 0017 §Read/Write Split). Missing: resource editors, keyframe integration, property search parity. The property-bag form is itself a **divergence** from Godot's class-property inspector (ADR 0002).

#### 3. Script editor & LSP — **partial** (with one Craft-only advantage)

Godot's Script screen is a full code editor with debugger integration and built-in class reference; Godot also *serves* LSP (`modules/gdscript/language_server`) and DAP (`editor/debugger/debug_adapter/`) so external editors can attach ([Using an external text editor](https://docs.godotengine.org/en/stable/tutorials/editor/external_editor.html)).

Craft (E2) ships a Lua editor panel that acts as an LSP **client** to a LuaLS subprocess with didOpen/didChange/didSave/diagnostics/completion (`crates/craft-editor/src/lua_lsp.rs`, `src/lsp/mod.rs`), plus auto-generated engine type stubs injected into the LuaLS workspace (`src/lua_stub_gen.rs`, ADR 0018 Appx A). Craft additionally has a **Behavior Editor** with schema-driven JSON auto-complete and inline validation (`crates/craft-editor/src/panels/behavior_editor.rs`, `src/json_path.rs`) — Godot has no equivalent (its behavior lives in GDScript), making this a Craft-only capability rooted in ADR 0003/0005. Missing: no DAP/debug-adapter integration, no go-to-definition parity beyond what LuaLS provides, no built-in class-reference browser.

#### 4. Debugger (breakpoints, stack trace, step into/over, REPL) — **missing**

Godot's Debugger panel provides breakpoints (gutter or `breakpoint` keyword, persisted across restarts), stack trace with variable inspection, Break/Continue/Step Into/Step Over, an Errors tab, and an expression evaluator (REPL) usable while paused ([Debugger panel](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/debugger_panel.html); source `editor/debugger/script_editor_debugger.cpp`).

Craft has **no** breakpoint/step-through-debugger surface. The editor offers only run/stop (F5/F8), pause/resume, and single-tick **Step (F10)** (`crates/craft-editor/src/menu.rs`, `src/panels/terminal_preview.rs:41`, `src/engine.rs:81`), plus a structured Error panel (ADR 0008). Adjacent divergence: Craft's debugging substrate is deterministic replay with per-tick state hashes (ADR 0006), but the editor has **no replay scrubber UI** — ADR 0018 Appx B lists "replay debugging" as a workflow, yet no replay code exists in `crates/craft-editor/src/` (grep-verified 2026-08-24).

#### 5. Profiler — **missing**

Godot's Profiler records function-level CPU timing per frame ([Debugger panel §Profiler](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/debugger_panel.html); source `editor/debugger/editor_profiler.cpp`). Craft has no profiling UI. The engine enforces its ≤8 ms tick budget only via test harnesses (ADR 0015, `games/tower_defense/tests/`), not via an editor surface.

#### 6. Visual Profiler — **missing**

Godot's Visual Profiler shows per-category CPU/GPU framegraphs for rendering tasks ([Debugger panel §Visual Profiler](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/debugger_panel.html); source `editor/debugger/editor_visual_profiler.cpp`). Craft has nothing comparable; relevant now that `craft-gpu` (wgpu 2D renderer) shipped in v0.2.0 (`docs/CHANGELOG.md`).

#### 7. Monitors — **missing**

Godot's Monitors tab graphs FPS, memory, object/node counts, and supports custom performance monitors that keep history even when closed ([Debugger panel §Monitors](https://docs.godotengine.org/en/stable/tutorials/scripting/debug/debugger_panel.html); source `editor/debugger/editor_performance_profiler.cpp`). Craft has no runtime metrics surface in the editor. Also missing: Godot's Video RAM and Network Profiler tabs (`editor/debugger/` listing, ref `4.7`).

#### 8. Remote scene tree / live-game inspection — **partial** (deliberate write-path divergence)

While a project runs, Godot's Scene dock gains **Remote/Local** toggles: Remote exposes the running instance's tree, and node parameters can be inspected *and changed* live; remote edits use a dedicated undo history (`REMOTE_HISTORY`, [EditorUndoRedoManager](https://docs.godotengine.org/en/stable/classes/class_editorundoredomanager.html); sources `editor/debugger/editor_debugger_tree.cpp`, `editor_debugger_inspector.cpp`). Godot's Game screen similarly allows real-time adjustments that are discarded on stop ([First look at Godot's interface](https://docs.godotengine.org/en/stable/getting_started/introduction/first_look_at_the_editor.html)).

Craft's Inspector and Scene Tree already show **live** values from the embedded engine while running (ADR 0017 §Read/Write Split), so live *inspection* is present. Live *mutation* of engine memory is deliberately absent: all writes go file → hot reload (`crates/craft-editor/src/engine.rs:86`, `src/watcher.rs`), so what you see always matches disk. This is a **divergence** (ADR 0017: "No direct mutation of engine memory… file-first principle applies to writes"), with the caveat that Godot's remote *write* capability (quick throwaway tweaks) has no Craft equivalent even as an option.

#### 9. Animation editor — **missing**

Godot's Animation bottom panel edits keyframe tracks for any property of any node, with per-track interpolation/loop modes, bezier curves, blend spaces, and an AnimationTree state-machine editor ([Animation introduction](https://docs.godotengine.org/en/stable/tutorials/animation/introduction.html); sources `editor/animation/animation_track_editor.cpp`, `animation_bezier_editor.cpp`, `animation_state_machine_editor.cpp`, ref `4.7`).

Craft has no keyframe/timeline editor and no engine-level keyframe animation. `crates/craft-animation/src/lib.rs` is a `PreTick` frame-cycling system (sprite `frames` array from a resource) — closer to AnimatedSprite playback than an animatable-property system. The editor surfaces nothing (grep: no animation code in `crates/craft-editor/src/` beyond egui theme timing).

#### 10. Tilemap editor — **missing**

Godot's TileMap panel paints onto TileMapLayer nodes with selection/paint/line/rect/bucket/picker/eraser tools, scattering, patterns, and terrain (autotiling) modes ([Using TileMaps](https://docs.godotengine.org/en/stable/tutorials/2d/using_tilemaps.html); sources `editor/scene/2d/tiles/tile_map_layer_editor.cpp`, `tile_set_editor.cpp`, `tiles_editor_plugin.cpp`, ref `4.7`).

Craft has no tilemap node type and no tilemap editor. Partial divergence context: v1 was ANSI-terminal-only (4-method Render trait, ADR 0004); GPU 2D sprites shipped in v0.2.0 but tile grids were not part of that scope (`docs/CHANGELOG.md`, `ROADMAP.md` §v3).

#### 11. Undo/redo — **parity**

Godot's `EditorUndoRedoManager` manages per-scene undo histories plus global and remote histories, with merge modes and unsaved-marking ([class reference](https://docs.godotengine.org/en/stable/classes/class_editorundoredomanager.html); source `editor/editor_undo_redo_manager.cpp`). Craft's `UndoRedo` (`crates/craft-editor/src/undo.rs`, E4) is the same command pattern — `begin_action`/`add_do`/`add_undo`/`commit_action`, redo-stack invalidation, 100-step cap — scoped per SceneDef. Core capability is equivalent; Godot's multi-history scoping (per-scene + global + remote) is richer, a minor **partial** note on an otherwise parity item.

#### 12. Run/preview loop — **partial**

Godot: F5 run project / F6 run current scene, embedded **Game** screen with pause and live adjust, Movie Maker mode ([First look at Godot's interface](https://docs.godotengine.org/en/stable/getting_started/introduction/first_look_at_the_editor.html)). Craft: F5 runs the scene — GPU window as subprocess on macOS (`docs/CHANGELOG.md` v0.2.0) — F8 stop, F10 step, pause/resume, and an embedded ANSI Terminal Preview panel rendering the engine's output (`crates/craft-editor/src/panels/terminal_preview.rs`). Missing: embedded interactive game view with input passthrough, per-scene run targets, movie capture. Divergence: Craft's preview edits persist via hot reload (ADR 0009) where Godot's Game-screen tweaks are discarded.

#### 13. Agent Copilot — **divergence** (Craft-only)

Godot has no in-editor agent surface (ADR 0019: "Godot has no equivalent to this panel"). Craft's Copilot (E3) is a docked chat panel with editor-context injection (active file, selected node, recent edits, engine schema), LLM tool use over engine AI primitives (`lint`, `dry_run`, `explain`, `read_scene`, `read_node`), and a diff preview/accept/reject flow where the agent cannot write to disk unapproved (`crates/craft-editor/src/panels/agent_panel.rs`, `src/agent/tools.rs`, `src/agent/context.rs`; ADR 0019).

#### 14. Adjacent Godot surface with no Craft counterpart (not graded — out of ticket scope)

For completeness of the inventory: editor plugin system (`editor/plugins/`, dock-slot API; Craft defers a plugin marketplace to v4 per `ROADMAP.md`), Asset Library, Import dock (`editor/docks/import_dock.cpp`), 2D/3D canvas viewport with gizmos (`editor/scene/canvas_item_editor_plugin.cpp`), shader editor, audio mixer bottom panels ([First look at Godot's interface](https://docs.godotengine.org/en/stable/getting_started/introduction/first_look_at_the_editor.html)).

### Summary table

| Godot surface | Craft counterpart (E1–E4) | Grade |
|---|---|---|
| Scene dock | Scene Tree panel | partial |
| Inspector | Inspector (component forms) | partial |
| Script editor + LSP/DAP | Lua editor (LuaLS client) + Behavior JSON editor | partial |
| Debugger (breakpoints/stack/REPL) | Step-tick only | missing |
| Profiler | none | missing |
| Visual Profiler | none | missing |
| Monitors (+VRAM/Network) | none | missing |
| Remote scene tree (live inspect+edit) | live inspect yes, live write no | partial / divergence (ADR 0017) |
| Animation editor | none (craft-animation = frame cycling only) | missing |
| Tilemap editor | none | missing |
| Undo/redo | UndoRedo command stack, 100 steps | parity |
| Run/preview loop | F5/F8/F10 + Terminal Preview | partial |
| Agent Copilot | Agent Copilot panel (E3) | divergence (Craft-only, ADR 0019) |

### Unverified / caveats

- All Godot doc claims are from `/en/stable/` pages fetched 2026-08-24; per-feature page content is not version-pinned to 4.7.2 specifically (docs track the stable branch).
- GitHub source paths were verified to *exist* at ref `4.7` via the Contents API; file *contents* were not audited.
- "No agent integration in Godot" is asserted per ADR 0019 and absence from the official editor docs; third-party editor plugins are out of scope (engine surface only).

## 14. Engine architecture (concurrency, node model, persistence)

> Graded evidence preserved from ticket [#24 — Gap analysis: architectural shape — object/ClassDB/Variant, threading, file formats, save/load](https://github.com/0xnicholas/Craft/issues/24) (findings: `docs/research/gap-architecture.md`, branch `research/architecture`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### 1. Godot inventory (4.7.2-stable)

#### Object / ClassDB / Variant
- `Object` is the base of almost everything; it provides reflection and editable properties via the `GDCLASS` macro and works with or without RTTI (`Object::cast_to<T>`) ([object_class.html](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/object_class.html)).
- `ClassDB` is a static registry of all `Object`-derived classes with dynamic bindings to methods, properties, and constants; `ClassDB::register_class<T>()` makes a class instantiable from scripts and deserializable ([object_class.html](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/object_class.html); runtime API: [class_classdb.html](https://docs.godotengine.org/en/stable/classes/class_classdb.html)).
- `Variant` is the universal tagged union (~20 bytes) used for communication, editing, serialization, thread hand-off, and script interop; it can be hashed, converted, and serialized to binary or text ([class_variant.html](https://docs.godotengine.org/en/stable/classes/class_variant.html)).
- Properties are declared with `PropertyInfo(type, name, hint, usage_flags)`; only `PROPERTY_USAGE_STORAGE` properties serialize ([object_class.html](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/object_class.html)).
- `RefCounted` provides intrusive reference counting; `Resource` inherits it, so all resources are refcounted and path-unique ([object_class.html](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/object_class.html)).

#### Threading
- `WorkerThreadPool` singleton: worker threads allocated at startup; `add_task` / `add_group_task` offload work; tasks must be awaited ([class_workerthreadpool.html](https://docs.godotengine.org/en/stable/classes/class_workerthreadpool.html)).
- The active scene tree is **not** thread-safe; tree mutation from threads requires `call_deferred`/`set_deferred` ([thread_safe_apis.html](https://docs.godotengine.org/en/stable/tutorials/performance/thread_safe_apis.html)).
- Rendering can run on a separate thread via the `Rendering > Driver > Thread Model = Separate` project setting (documented as having known bugs); physics servers can each run on separate threads; navigation servers are thread-safe and run queries in true parallel ([thread_safe_apis.html](https://docs.godotengine.org/en/stable/tutorials/performance/thread_safe_apis.html)).
- Signals: default connections invoke callables instantly at the emission point; `CONNECT_DEFERRED` connections trigger callables "on idle time, rather than instantly" ([class_object.html — ConnectFlags](https://docs.godotengine.org/en/stable/classes/class_object.html)).

#### File formats & save/load
- Everything saved/loaded is a `Resource`; text formats `.tscn`/`.tres` are VCS-friendly, binary `.scn`/`.res` are used on export for speed/compression ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html); loaders: `scene/resources/resource_format_text.cpp`, `core/io/resource_format_binary.cpp` at tag `4.7.2-stable`).
- `PackedScene` is a serialized scene-tree resource; `pack()` serializes, `instantiate()` rebuilds ([class_packedscene.html](https://docs.godotengine.org/en/stable/classes/class_packedscene.html)).
- `ResourceLoader` caches by path — a path loads once and is shared across instances; `ResourceSaver` bundles un-pathed sub-resources with sub-IDs ([resources.html](https://docs.godotengine.org/en/stable/tutorials/scripting/resources.html), [object_class.html](https://docs.godotengine.org/en/stable/contributing/development/core_and_modules/object_class.html)).
- Godot ships **no built-in save-game system**: the documented idiom is per-project — mark nodes in a "Persist" group, each returns a dict, serialize line-by-line as JSON via `FileAccess`, re-instantiate from `scene_file_path` on load; binary `store_var`/`get_var` is the suggested alternative for complex state ([saving_games.html](https://docs.godotengine.org/en/stable/tutorials/io/saving_games.html)).

### 2. Craft inventory (this worktree)

- **Node model**: one `Node` struct — `id: String`, `type_name`, `parent`, `components: BTreeMap<String, Component>`, `behaviors`, `lua_class` (`crates/craft-kernel/src/scene.rs:45-72`). `ComponentValue` is a closed 8-variant enum (`Nil, Bool, Int, Float, String, Vec2, Vec3, Rect`) (`scene.rs:12-23`). Types differ by component key sets registered via `NodeRegistry`/`inventory` and validated at load (`scene.rs:664-692`; ADR `docs/adr/0002-node-model.md`).
- **Reflection replacement**: compile-time JSON Schema via schemars + `craft-schema`, generating the TS SDK (ADR `docs/adr/0005-schema-pipeline.md`). No runtime class registry, no downcasting, no method binding — dynamic dispatch is JSON behaviors (9 verbs) + `craft_system!` registry (ADR `docs/adr/0003-behavior-runtime.md`).
- **Threading**: engine core has zero `std::thread` usage (verified by grep over `crates/`); threads appear only in `craft-editor` for the LLM agent client and the Lua LSP sidecar (`crates/craft-editor/src/agent/client.rs:188`, `lua_lsp.rs:53`) — outside the engine. GPU rendering runs as a separate **process** with a file-based handoff (ADR `docs/adr/0020-gpu-subprocess-boundary.md`). Single-threaded tick is non-negotiable (ADR `docs/adr/0015-performance-budgets.md`; ARCHITECTURE.md:75).
- **Signals**: next-tick delivery queue (`VecDeque`), not same-tick (ADR 0003; ARCHITECTURE.md §Command Buffer Pipeline).
- **Persistence**: scene = single JSON document, flat node list with string `parent` ids; serde `Serialize`/`Deserialize` with `#[serde(skip)]` on runtime-only fields (`destroyed`) (`scene.rs:70-71, 201-214`); `Scene::load`/`parse`/`from_value`/`to_value` (`scene.rs:214-288`). Resources: JSON-only `ResourceRegistry` with monotonically increasing versions; `ResourceRef` pins `snapshot_version` at load — loaded instances are deliberately NOT retroactively updated (`crates/craft-kernel/src/resource.rs`; ADR `docs/adr/0009-hot-reload-and-resources.md`).
- **Replay**: `craft-replay` `Recording` = scene snapshot + resource snapshots + seed + per-tick `state_hash` + input frames; replay verifies hash equality per tick (`crates/craft-replay/src/lib.rs:16-186`; ADR `docs/adr/0006-replay-system.md`).
- **Save games**: no engine save/load-state API exists (`Engine` public API, `crates/craft-kernel/src/engine.rs:66-450` — no `save`/`restore`). The nearest mechanisms are `Scene::to_value()` (in-memory JSON) and replay `Recording`s.

### 3. Comparison (graded)

#### Object model / reflection

| # | Godot feature | Grade | Craft position / ADR |
|---|---------------|-------|----------------------|
| 1 | `GDCLASS` + `ClassDB` runtime reflection registry | **divergence** | Compile-time JSON Schema instead (ADR 0005, 0002) |
| 2 | `Variant` universal type (hashable, binary/text serializable) | **divergence** | `ComponentValue` — 8 closed variants, serde-tagged (`scene.rs:12-23`; ADR 0002) |
| 3 | Inheritance + `Object::cast_to<T>()` | **divergence** | No inheritance; types = component key sets (ADR 0002) |
| 4 | `MethodBind` string-keyed dynamic dispatch (`call()`) | **divergence** | 9 closed action verbs + `craft_system!` functions (ADR 0003) |
| 5 | `RefCounted` intrusive refcounting / `Resource` path-unique sharing | **divergence** | Rust ownership; registry versioning instead of shared mutable resources (ADR 0009) |
| 6 | `PROPERTY_USAGE_STORAGE` selective property serialization | **parity** | serde `skip`/`skip_serializing_if` achieves the same selectivity (`scene.rs:37, 70`) |
| 7 | `_notification` engine callbacks | **partial** | Lifecycle covered by signals + Lua hooks (`on_tick`/`on_signal`/`on_spawn`, ADR 0016); no general notification enum |

#### Threading

| # | Godot feature | Grade | Craft position / ADR |
|---|---------------|-------|----------------------|
| 8 | `WorkerThreadPool` task/group parallelism | **divergence** | Single-threaded tick, non-negotiable (ADR 0015) |
| 9 | Rendering on separate thread (Thread Model = Separate) | **divergence** | Render runs in-tick; GPU renderer is a separate **process** (ADR 0015, 0020) |
| 10 | Physics servers on separate threads | **divergence** | `craft-physics` runs inside the tick (ADR 0015); determinism requires it |
| 11 | Scene tree single-main-thread discipline | **parity** | Both engines serialize tree mutation onto one logical thread (Godot via docs/locking advice, Craft by construction) |
| 12 | Thread-safe server singletons (Rendering/Physics/Navigation) | **divergence** | 4-method `Render` trait, no server layer (ADR 0004) |
| 13 | Signal delivery: instant-at-emission / `CONNECT_DEFERRED` on idle | **divergence** | Next-tick queue only; no same-tick delivery at all (ADR 0003) — stronger ordering guarantee than Godot's deferred flag |
| 14 | `call_deferred` / `set_deferred` | **divergence** | Not needed; Lua direct mutation + JSON command buffer replace deferred calls (ADR 0003) |

#### File formats & save/load

| # | Godot feature | Grade | Craft position / ADR |
|---|---------------|-------|----------------------|
| 15 | Dual text/binary formats (`.tscn`/`.tres` + `.scn`/`.res`) | **divergence** | JSON only — "no import pipeline, no binary formats" (ADR 0009 §Rationale) |
| 16 | `PackedScene` (scene as resource, `pack`/`instantiate`) | **missing** | Flat node list + inline `spawn` verb; no scene templates (owned by scene-assets domain, ticket #18) |
| 17 | `ResourceLoader` load-once path cache, shared instances | **divergence** | Deliberately opposite: snapshot-at-load versioning (ADR 0009 §"loaded instances not retroactively updated") |
| 18 | `ResourceSaver` + sub-resource bundling (`path::id`) | **missing** | No engine-side write API for resources; resources are agent-authored JSON files |
| 19 | Import pipeline (`.png`→`.ctex`, `.import` metadata) | **divergence** | `craft-assets` is a deterministic generator (JSON pixel-map → atlas), not an importer (ADR 0009; `crates/craft-assets/`) |
| 20 | Save games | **parity** | Neither engine ships a save system. Godot documents a per-project JSON idiom; Craft has `Scene::to_value()` + replay `Recording` as the closest built-ins, but no save/restore endpoint (`engine.rs`) |
| 21 | Replays / deterministic recording | **divergence** | Godot has none; Craft has per-tick hash-verified replay as a core system (ADR 0006) — Craft exceeds Godot here |

### 4. Where architecture bounds feature gaps in other domains

Sibling findings live in their own worktrees under `docs/research/`; cited here by branch.

- **Networking** (`research/networking`, gap-networking.md): every item graded **missing**. Architecture is the bound: single-process single-threaded engine (ADR 0015), sync NAPI with no IPC/WebSocket in v1 (ADR 0007), and the determinism invariant (ADR 0006) leave no seam for asynchronous, non-deterministic network I/O. Even the `spawn` verb is local-only.
- **2D/3D rendering** (`research/2d-rendering`, `research/3d-rendering`): the 4-method `Render` trait (ADR 0004) and the GPU **subprocess** boundary (ADR 0020) bound the renderable feature surface — no rendering thread, no shared-memory editor viewport, no in-process GPU access from the engine.
- **Scene assets** (`research/scene-assets`, gap-scene-assets.md): the flat single-document JSON scene (ADR 0002/0009) is what makes `PackedScene` instancing, scene inheritance, and binary formats **missing/divergence** — there is no resource-typed scene to instance, by design.
- **Physics** (`research/physics`, gap-physics.md): in-tick-only simulation and f64-only arithmetic (ADR 0006 §Determinism Requirements, ADR 0015) exclude threaded physics servers and any engine whose solver isn't deterministic.
- **Scripting** (`research/scripting`, gap-scripting.md): the absence of `Variant`/`MethodBind` (ADR 0002) is exactly why scripting is a two-tier Lua + closed-verb-JSON model (ADR 0016) instead of GDScript-style arbitrary method calls — the scripting gap list is a downstream consequence of the object-model divergence.
- **Audio/animation/UI/editor**: all signal-driven, in-tick subsystems; their ceilings (e.g. no streaming thread for audio, no animation blending thread) trace back to ADR 0015.

Cross-cutting observation: the single most load-bearing divergence is ADR 0006 (deterministic replay). Threading (ADR 0015), next-tick signals (ADR 0003), resource snapshot versioning (ADR 0009), and `BTreeMap` component ordering (`scene.rs:53`) are all downstream of "per-tick hash must match." Feature gaps in networking, threaded physics, and shared-resource semantics are bounded by that one invariant, not by missing effort.

### 5. ADR-vs-implementation drift (worth flagging to maintainers)

- ADR 0002 and ADR 0015 specify `SlotMap<NodeId, Node>` with generational indices. The implementation uses a flat `Vec<Node>` with `String` ids and a `spawn_counter` (`scene.rs:201-214, 313-318`); `SlotMap` does not appear in `crates/`. The implementation is arguably *more* JSON-faithful than the ADR, but the ADRs (and ARCHITECTURE.md §1, README table) still describe the SlotMap design.
- ADR 0002 §"NodeId Stability" describes in-place `Node::replace_type` for hot reload; hot reload in practice diffs the flat list and preserves string ids (`crates/craft-kernel/src/hot_reload.rs`). Same invariant, different mechanism than documented.
- `Scene::to_value()` uses `.expect("scene serialization is infallible")` (`scene.rs:288`) — pre-existing, against the repo's no-`expect` convention.

### Unverified / caveats

- Godot "signals invoke callables instantly at emission" is stated in `class_object.html`'s `CONNECT_DEFERRED` enum description ("on idle time, rather than instantly"); I did not additionally verify against `object.cpp` source.
- Line-count claims for Godot subsystems (e.g. `core/` ≈170K) are from Craft's own ARCHITECTURE.md/README, not re-measured against the 4.7.2-stable tree.
- Sibling gap docs are uncommitted work in their own worktrees (not on `main`); section 4 summarizes their headlines, which may shift before the driving session merges them.

## 15. Reverse gaps: what Craft has that Godot lacks

> Graded evidence preserved from ticket [#25 — Reverse gap analysis: Craft capabilities with no Godot equivalent](https://github.com/0xnicholas/Craft/issues/25) (findings: `docs/research/gap-reverse-gaps.md`, branch `research/reverse-gaps`). Per-ticket date/version pins are consolidated in [Baselines and method](#baselines-and-method); body text is otherwise verbatim.

### Summary

| # | Craft capability | Grade |
|---|------------------|-------|
| 1 | Deterministic replay with per-tick hash verification | **Full gap** |
| 2 | Closed-set 9-verb action vocabulary | **Full gap** |
| 3 | Compile-time schema as source of truth, generated TS types | **Partial equivalent** (different scope) |
| 4 | Structured JSON errors | **Full gap** |
| 5 | Hot-reload-as-default authoring loop | **Partial equivalent** (script-only in Godot) |
| 6 | NAPI bridge + agent SDK | **Designed-but-unimplemented** in Craft; no Godot equivalent |
| 7 | LLM benchmark harness (craft-eval) | **Full gap** |
| 8 | Deterministic asset generation | **Full gap** |

### 1. Deterministic replay with per-tick hash verification

**Craft (implemented).** `crates/craft-replay/src/lib.rs`: `Recorder` captures scene/resource snapshots, seed, and per-tick `TickReport { tick, input, state_hash, signals_emitted }` (lines 33–75); `ReplayRunner::step` recomputes the engine hash each tick and compares it against the recorded hash (`hash_ok`, lines 406–421); `diff()` compares any two recorded ticks (lines 431–450). The hashed state comes from `craft_kernel::hash_scene_state` / `Engine::state_hash`, and determinism is enforced at the scripting layer by `crates/craft-lua/src/determinism.rs` (`DeterminismMode::Replay` locks `math.random` → `engine.rng()` and disables `os.clock`). Recordings pin Lua module versions and source SHA-256s (`ModuleRecord`, ADR 0016 §"L3"). Design: `docs/adr/0006-replay-system.md`; regression layer per `docs/adr/0010-testing-strategy.md`.

**Godot.** No equivalent. Godot ships no record/replay facility, no scene-state hash API, and no determinism guarantee — GodotPhysics is not deterministic across runs/platforms, and determinism questions are perennially redirected to userland (e.g. [godot-proposals #11034](https://github.com/godotengine/godot-proposals/issues/11034), [forum: making physics fully deterministic](https://godotengine.org/t/making-physics-fully-deterministic/22575)). Engine-integrated per-tick state hashing does not exist in the class reference.

**Grade: full gap.**

### 2. Closed-set 9-verb action vocabulary

**Craft (implemented).** `crates/craft-kernel/src/behavior.rs:153` defines `enum Action` with exactly nine variants — `set_state`, `emit`, `destroy`, `spawn`, `if`, `move`, `animate`, `log`, `call_system` — mirrored in `crates/craft-schema/src/lib.rs:10` (`ACTION_VERBS`) and linted (`engine.lint` rejects unknown verbs). ADR 0003 makes the closed set non-negotiable: agents cannot invent new verbs.

**Godot.** No equivalent concept. Behavior in Godot is open-ended imperative code (GDScript/C#/GDExtension); there is no declarative, schema-bounded action DSL anywhere in the engine. The closest first-party constructs are `AnimationPlayer`/tween method calls and `InputMap` action names — both unrelated in kind.

**Grade: full gap.**

### 3. Compile-time schema as source of truth with generated TS types

**Craft (implemented).** `crates/craft-schema/src/lib.rs` derives JSON Schema at compile time via schemars 0.8 (`SchemaSettings::openapi3`, `get_action_schema`, verb descriptions); `crates/craft-schema/src/bin/gen-sdk.rs` emits `sdk/src/generated.ts` (header: "AUTO-GENERATED by craft-schema — do not edit"), so the Rust types are the single source of truth for the TypeScript SDK surface. Design: `docs/adr/0005-schema-pipeline.md`.

**Godot (partial).** Godot has an analogous *mechanism* at a different *scope*: running the editor with `--dump-extension-api` produces `extension_api.json`, a machine-readable API dump from which godot-cpp and other binding generators generate language bindings at build time ([GDExtension docs, stable](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/what_is_gdextension.html)). However, it describes the engine's native C++ API for GDExtension authors — not a schema governing game content (scenes, behaviors, actions) — and Godot itself generates no TypeScript types from it.

**Grade: partial equivalent** (mechanism exists; purpose and scope differ).

### 4. Structured JSON errors

**Craft (implemented).** `crates/craft-kernel/src/error.rs`: `ValidationError` carries `file`, `json_path`, `expected_type`, `actual_value`, `suggestion`, and an `AutoFix` classification; parse and IO errors carry file/line/column/snippet. `crates/craft-bridge/src/lib.rs` maps every `EngineError` into a JSON-RPC error with a structured `data` payload (`engine_error_to_rpc`, `validation_errors_to_data`). ADR 0008: "never panic, never print to stderr."

**Godot.** No equivalent. Godot errors are human-readable strings emitted through `ERR_PRINT`/`ERR_FAIL_*` macros and `push_error()`; GDScript has no exception/try-catch mechanism and the error-handling model is acknowledged as under-documented and stringly-typed ([godot-docs issue #10542](https://github.com/godotengine/godot-docs/issues/10542), [godot-proposals discussion #4111](https://github.com/godotengine/godot-proposals/discussions/4111)). No JSON-path-addressable, machine-consumable error payloads exist.

**Grade: full gap.**

### 5. Hot-reload-as-default authoring loop

**Craft (implemented).** `crates/craft-kernel/src/hot_reload.rs`: `compute_scene_diff` / `apply_scene_diff` handle structural changes (added/removed nodes, changed behaviors, component value changes) against a *running* engine; `HotReloader::watch` uses `notify` to watch scene files; the bridge exposes `engine.applyHotReload` (`crates/craft-bridge/src/lib.rs:601`). ADR 0009 states the requirement plainly: "Hot reload is the default authoring loop… Every file save is a potential hot reload" — with a defined interaction boundary with replay (recordings cannot cross a reload boundary in v1).

**Godot (partial).** Godot hot-reloads *scripts*: saving a `.gd` file re-parses and rebinds it in both the editor and a running game, and since 4.2 GDExtension libraries can be hot-reloaded in the editor ([godot-proposals #9620](https://github.com/godotengine/godot-proposals/issues/9620), [godot-rust book](https://godot-rust.github.io/book/intro/hello-world.html)). But it is script-file-scoped: scene structure (`.tscn`/`.tres`) is not diffed and re-applied to a running scene, there is no reload-notification API (that is exactly what proposal #9620 requests), and reload is an editor convenience, not the engine's default authoring contract.

**Grade: partial equivalent** (Godot: script-level hot reload; Craft: structural scene-diff hot reload as the default loop).

### 6. NAPI bridge + agent SDK

**Craft (designed-but-unimplemented binding; JSON-RPC core implemented).** ADR 0007 (`docs/adr/0007-bridge-layer.md`) specifies sync NAPI bindings via napi-rs with a thin JSON-RPC dispatcher. Verified directly in-tree: `crates/craft-bridge/src/lib.rs` implements a complete, tested JSON-RPC 2.0 dispatcher (`Bridge::dispatch`, `dispatch_batch_str`, 16 `engine.*` methods including `engine.getSchema`, `engine.dryRun`, `engine.applyHotReload`, `engine.stateHash`) — **but** `crates/craft-bridge/Cargo.toml` has no napi-rs dependency, a workspace-wide grep finds no `#[napi]` functions, and no `package.json` exists anywhere in the repo. The SDK is `sdk/src/generated.ts` — generated type definitions only, with no client/transport code. This confirms the sibling-ticket (#21) finding: the Node-facing NAPI layer and npm-packaged SDK do not exist; only the transport-agnostic dispatcher does (ADR 0007 leaves a transport trait for v2).

**Godot.** No equivalent. GDExtension runs native code *inside* the engine process; there is no first-party agent-facing RPC surface or JavaScript/TypeScript SDK.

**Grade: designed-but-unimplemented on the Craft side; full gap on the Godot side.**

### 7. LLM benchmark harness (craft-eval)

**Craft (implemented).** `crates/craft-eval/src/lib.rs`: `BenchmarkSpec` (seed, ticks, scene, scripted actions, `expected_final_hash`, expected components/signals, plus a `full_game` mode where the agent authors an entire game whose scripted playthrough must satisfy a win condition), a `Backend` trait with `StubBackend` (deterministic, for CI) and `LiveBackend` (HTTP LLM API), and hash-equality-based pass/fail reporting. Task specs live in `benchmarks/01_on_tick_increment.json` through `05_platformer.json`; the runner is `crates/craft-eval/src/bin/eval.rs`. Design: `docs/adr/0010-testing-strategy.md` layer 3.

**Godot.** No equivalent. Godot's testing story is unit/integration test frameworks and headless CI (`--headless`, doctest-based engine tests); there is nothing that drives an LLM agent against the engine with reproducible pass/fail criteria.

**Grade: full gap.**

### 8. Deterministic asset generation

**Craft (implemented).** `crates/craft-assets/` generates binary assets from declarative specs as pure functions: `pixel_map.rs` (JSON pixel-map → PNG + atlas), `font.rs` (built-in bitmap font → PNG + metadata), `wav.rs` (synthesized tone WAVs for jump/coin/win), driven by the CLI in `main.rs`. `Cargo.toml` has no `rand`/time dependency — output is a deterministic function of input by construction, which is what lets `craft-eval` full-game tasks treat assets as agent-authored data (`GameDeliverable.assets`, `crates/craft-eval/src/lib.rs:68`).

**Godot.** No equivalent. Godot's asset pipeline is an *import* pipeline (source art/audio converted to engine formats by importers); procedural generation is possible only by writing imperative game code against runtime APIs (`Image`, `AudioStreamGenerator`, `SurfaceTool`), not as a first-party declarative, reproducible asset-generation feature.

**Grade: full gap.**

### Caveats

- Godot-side claims are verified against official docs/source and first-party issue trackers for 4.7.2-stable; community addons (e.g. deterministic physics plugins, rollback frameworks) were excluded by the ticket's "engine feature surface only" scope.
- The NAPI/SDK item is graded on implementation status as of this commit; the JSON-RPC dispatcher it would wrap is fully implemented and tested in `craft-bridge`.
## Provenance and assembly notes

Assembled by ticket [Assemble gap-analysis report from per-domain findings](https://github.com/0xnicholas/Craft/issues/26) for the wayfinding map [Craft vs Godot engine-feature gap analysis](https://github.com/0xnicholas/Craft/issues/10). Chapter bodies are preserved verbatim from the research findings (metadata blocks and per-ticket grading legends consolidated into this report's front matter).

| Chapter | Ticket | Findings document (research branch) |
|---|---|---|
| 1 | [2D rendering (sprites, tilemaps, lighting, post-processing)](https://github.com/0xnicholas/Craft/issues/11) | `docs/research/gap-2d-rendering.md` (`research/2d-rendering`) |
| 2 | [3D rendering + XR](https://github.com/0xnicholas/Craft/issues/12) | `docs/research/gap-3d-rendering.md` (`research/3d-rendering`) |
| 3 | [Shaders, materials, particles/VFX](https://github.com/0xnicholas/Craft/issues/13) | `docs/research/gap-shaders-vfx.md` (`research/shaders-vfx`) |
| 4 | [Physics 2D/3D](https://github.com/0xnicholas/Craft/issues/14) | `docs/research/gap-physics.md` (`research/physics`) |
| 5 | [Animation](https://github.com/0xnicholas/Craft/issues/15) | `docs/research/gap-animation.md` (`research/animation`) |
| 6 | [Audio](https://github.com/0xnicholas/Craft/issues/16) | `docs/research/gap-audio.md` (`research/audio`) |
| 7 | [UI/GUI](https://github.com/0xnicholas/Craft/issues/17) | `docs/research/gap-ui-gui.md` (`research/ui-gui`) |
| 8 | [Scene system + asset pipeline](https://github.com/0xnicholas/Craft/issues/18) | `docs/research/gap-scene-assets.md` (`research/scene-assets`) |
| 9 | [Scripting & languages](https://github.com/0xnicholas/Craft/issues/19) | `docs/research/gap-scripting.md` (`research/scripting`) |
| 10 | [Navigation & pathfinding](https://github.com/0xnicholas/Craft/issues/20) | `docs/research/gap-navigation.md` (`research/navigation`) |
| 11 | [Input + platform export](https://github.com/0xnicholas/Craft/issues/21) | `docs/research/gap-input-export.md` (`research/input-export`) |
| 12 | [Networking & multiplayer](https://github.com/0xnicholas/Craft/issues/22) | `docs/research/gap-networking.md` (`research/networking`) |
| 13 | [Editor & debugging tools](https://github.com/0xnicholas/Craft/issues/23) | `docs/research/gap-editor-tools.md` (`research/editor-tools`) |
| 14 | [Engine architecture](https://github.com/0xnicholas/Craft/issues/24) | `docs/research/gap-architecture.md` (`research/architecture`) |
| 15 | [Reverse gaps](https://github.com/0xnicholas/Craft/issues/25) | `docs/research/gap-reverse-gaps.md` (`research/reverse-gaps`) |

Assembly decisions:

- **Chapter order** follows the research ticket order (#11–#25), which mirrors Godot's own feature-domain grouping; the reverse-gap chapter is last, per the report's charter.
- **Filename** `docs/gap-analysis-godot.md` adopts the map's working title as final.
- **Version-pin drift** (ticket #11 pinned 4.7.0, ticket #16 pinned 4.7.1 vs the canonical 4.7.2-stable) is normalized in [Baselines and method](#baselines-and-method) and flagged as comments on those tickets — no grades were affected, since all tickets cite the same `en/stable` (4.7) documentation branch.
- **One headline/table mismatch** in the source findings: chapter 2's summary prose says "13/13 functional areas missing; 2 deliberate divergences" while its graded table holds 12 rows (9 missing + 3 divergence). The table is authoritative; the mismatch is flagged on ticket #12.
- **Summary-matrix row counts** grade each table row by its **first** grade token; rows carrying an additional divergence annotation (e.g. physics' "missing (deferred)", shaders' "missing — structural divergence: ADR 0002") count once, by primary grade.
- **No cross-chapter contradictions were found.** Adjacent chapters' claims about hot reload, the NAPI bridge, determinism, and the `animate` verb stub agree with each other; each is graded once in its owning chapter.
- Physics rows graded **missing (deferred)** keep that parenthetical: the primary grade is `missing`, the annotation cites the documented ROADMAP deferral.
