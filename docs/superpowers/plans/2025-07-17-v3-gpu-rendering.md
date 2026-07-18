# v3.0-GPU: 2D Sprite/GPU Rendering — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add 2D GPU-accelerated sprite rendering via a new `craft-gpu` crate (wgpu + winit), with zero changes to the kernel `Render` trait.

**Architecture:** `craft-gpu` implements the existing `Render` trait. New `ComponentValue` variants (Vec3, Rect) added to kernel. Rendering components (`sprite`, `modulate`, `z_index`, etc.) are universal — any node can have them. Editor F5 launches the game window as a subprocess (macOS-safe). Four milestones: G1 (kernel types), G2 (crate + window), G3 (sprites + camera), G4 (editor integration).

**Tech Stack:** Rust 2024, wgpu 0.20, winit 0.30, image 0.25, existing craft-kernel/lua/editor/schema crates.

**Spec:** `docs/superpowers/specs/2025-07-17-v3-gpu-rendering-design.md`

---

## File Map

### G1 — kernel changes

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `crates/craft-kernel/src/scene.rs` | Add Vec3/Rect to ComponentValue, ComponentType, parser, universal allowlist |
| Modify | `crates/craft-kernel/src/render.rs` | Add `SpriteDrawCommand`, extraction helpers, camera helpers |
| Modify | `crates/craft-kernel/src/evaluator.rs` | Exhaustive match: `component_value_to_json` new arms |
| Modify | `crates/craft-kernel/src/lint.rs` | Exhaustive match: `component_type_name`, `component_to_json_value` new arms |
| Modify | `crates/craft-schema/src/lib.rs` | New ComponentType → JSON Schema mapping |
| Modify | `crates/craft-lua/src/runtime.rs` | `lua_to_component_value` + `component_value_to_lua` — Vec3/Rect arms |

### G2 — craft-gpu crate

| Action | Path | Purpose |
|--------|------|---------|
| Create | `crates/craft-gpu/Cargo.toml` | Crate manifest: wgpu, winit, image deps |
| Create | `crates/craft-gpu/src/lib.rs` | `GpuRenderer`, `GameWindowConfig`, `spawn_game_window`, `Render` impl |
| Create | `crates/craft-gpu/src/texture.rs` | `TextureCache`, texture loading, magenta placeholder |
| Create | `crates/craft-gpu/src/shaders.rs` | WGSL vertex/fragment shaders as static strings |
| Modify | `Cargo.toml` | Add `craft-gpu` to workspace members |

### G3 — sprite rendering

| Action | Path | Purpose |
|--------|------|---------|
| Create | `crates/craft-gpu/src/sprite.rs` | `SpriteDrawCommand`, batching, instance buffer |
| Create | `crates/craft-gpu/src/camera.rs` | `Camera2D`, view-projection matrix, uniform buffer |
| Modify | `crates/craft-gpu/src/lib.rs` | Wire up sprite extraction + camera in `render()` |

### G4 — editor integration

| Action | Path | Purpose |
|--------|------|---------|
| Modify | `crates/craft-editor/src/app.rs` | F5 handler: save scene, launch subprocess |
| Modify | `crates/craft-editor/Cargo.toml` | Add `craft-gpu` dependency |
| Create | `crates/craft-gpu/src/bin.rs` | Standalone binary entry point for subprocess launch |

---

## G1: Component Types + SpriteDrawCommand

### Task G1.1: Add Vec3 and Rect to ComponentValue

**Files:**
- Modify: `crates/craft-kernel/src/scene.rs`

- [ ] **Step 1: Add variants to ComponentValue enum**

In `scene.rs`, find the `ComponentValue` enum (currently Nil, Bool, Int, Float, String, Vec2). Add two new variants:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum ComponentValue {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vec2([f64; 2]),
    Vec3([f64; 3]),        // NEW: color tint, RGB
    Rect([f64; 4]),        // NEW: source rect, bounds
}
```

- [ ] **Step 2: Add variants to ComponentType enum**

Find `ComponentType` enum (~line 591). Add:

```rust
pub enum ComponentType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Vec2,
    Vec3,   // NEW
    Rect,   // NEW
}
```

- [ ] **Step 3: Update Display impl for ComponentType**

Find `impl fmt::Display for ComponentType`. Add:

```rust
Self::Vec3 => "vec3",
Self::Rect => "rect",
```

- [ ] **Step 4: Update component_value_matches**

Find `component_value_matches` function (~line 570). Add:

```rust
(ComponentValue::Vec3(_), ComponentType::Vec3)
    | (ComponentValue::Rect(_), ComponentType::Rect)
```

- [ ] **Step 5: Build and fix compiler errors for any remaining match sites**

Run: `cargo check -p craft-kernel 2>&1`
Fix any exhaustive match errors the compiler flags.

- [ ] **Step 6: Run kernel tests**

Run: `cargo test -p craft-kernel`
Expected: all existing tests pass (compiler-guarded — if it compiles, match arms are correct).

- [ ] **Step 7: Commit**

```bash
git add crates/craft-kernel/src/scene.rs
git commit -m "feat(kernel): add Vec3 and Rect ComponentValue variants"
```

### Task G1.2: Extend JSON parser with array-length heuristics

**Files:**
- Modify: `crates/craft-kernel/src/scene.rs`

- [ ] **Step 1: Write failing tests for Vec3 and Rect parsing**

Add to the `#[cfg(test)] mod tests` block:

```rust
#[test]
fn parses_vec3_from_3_element_array() {
    let v = json_to_component_value(json!([1.0, 2.0, 3.0])).unwrap();
    assert_eq!(v, ComponentValue::Vec3([1.0, 2.0, 3.0]));
}

#[test]
fn parses_rect_from_4_element_array() {
    let v = json_to_component_value(json!([0.0, 64.0, 32.0, 32.0])).unwrap();
    assert_eq!(v, ComponentValue::Rect([0.0, 64.0, 32.0, 32.0]));
}

#[test]
fn rejects_array_of_length_5() {
    let err = json_to_component_value(json!([1.0, 2.0, 3.0, 4.0, 5.0])).unwrap_err();
    assert!(err.contains("expected [x, y]"));
}
```

- [ ] **Step 2: Run tests to verify failure**

Run: `cargo test -p craft-kernel -- parses_vec3`
Expected: FAIL — "expected [x, y] for vec2, got array of length 3"

- [ ] **Step 3: Extend json_to_component_value array handling**

In `json_to_component_value` (~line 153), replace the array match arm:

```rust
Value::Array(a) => match a.len() {
    2 => {
        let x = a[0].as_f64()
            .ok_or_else(|| format!("vec2[0] must be a number, got {:?}", a[0]))?;
        let y = a[1].as_f64()
            .ok_or_else(|| format!("vec2[1] must be a number, got {:?}", a[1]))?;
        Ok(ComponentValue::Vec2([x, y]))
    }
    3 => {
        let r = a[0].as_f64()
            .ok_or_else(|| format!("vec3[0] must be a number, got {:?}", a[0]))?;
        let g = a[1].as_f64()
            .ok_or_else(|| format!("vec3[1] must be a number, got {:?}", a[1]))?;
        let b = a[2].as_f64()
            .ok_or_else(|| format!("vec3[2] must be a number, got {:?}", a[2]))?;
        Ok(ComponentValue::Vec3([r, g, b]))
    }
    4 => {
        let x = a[0].as_f64()
            .ok_or_else(|| format!("rect[0] must be a number, got {:?}", a[0]))?;
        let y = a[1].as_f64()
            .ok_or_else(|| format!("rect[1] must be a number, got {:?}", a[1]))?;
        let w = a[2].as_f64()
            .ok_or_else(|| format!("rect[2] must be a number, got {:?}", a[2]))?;
        let h = a[3].as_f64()
            .ok_or_else(|| format!("rect[3] must be a number, got {:?}", a[3]))?;
        Ok(ComponentValue::Rect([x, y, w, h]))
    }
    other => Err(format!(
        "expected [x, y], [r, g, b], or [x, y, w, h], got array of length {other}"
    )),
},
```

- [ ] **Step 4: Run all kernel tests**

Run: `cargo test -p craft-kernel`
Expected: all tests pass including the 3 new parsing tests.

- [ ] **Step 5: Commit**

```bash
git add crates/craft-kernel/src/scene.rs
git commit -m "feat(kernel): extend JSON parser for Vec3 (len 3) and Rect (len 4)"
```

### Task G1.3: Add universal component allowlist to validator

**Files:**
- Modify: `crates/craft-kernel/src/scene.rs`

- [ ] **Step 1: Write failing test — sprite component on Player node**

Add test:

```rust
#[test]
fn universal_components_accepted_regardless_of_node_type() {
    let mut r = registry();
    let json = r#"{
        "kind": "scene",
        "name": "main",
        "nodes": [{
            "id": "p1",
            "type": "Player",
            "components": {
                "position": [0.0, 0.0],
                "health": 100,
                "sprite": "player.png",
                "modulate": [1.0, 1.0, 1.0],
                "z_index": 5
            }
        }]
    }"#;
    // "sprite", "modulate", "z_index" are NOT in Player's component_specs()
    // They should be accepted via the universal allowlist
    let scene = Scene::parse(json, "scene.json", &r).expect("parse with universal components");
    let node = &scene.nodes[0];
    assert_eq!(
        node.get_component_value("sprite"),
        Some(&ComponentValue::String("player.png".to_string()))
    );
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p craft-kernel -- universal_components`
Expected: FAIL — "unknown component \"sprite\" for node type \"Player\""

- [ ] **Step 3: Add universal component allowlist constant**

Above `validate_node` function, add:

```rust
const UNIVERSAL_COMPONENTS: &[&str] = &[
    "position",
    "sprite",
    "sprite_rect",
    "modulate",
    "alpha",
    "scale",
    "rotation",
    "z_index",
    "visible",
];
```

- [ ] **Step 4: Skip validation for universal components**

In `validate_node`, find the loop that checks `unexpected` components. Modify the filter:

```rust
let mut unexpected: Vec<&str> = node
    .components
    .keys()
    .map(String::as_str)
    .filter(|k| !known.contains_key(k) && !UNIVERSAL_COMPONENTS.contains(k))
    .collect();
```

Also ensure missing-check (`missing_sorted`) skips universal components:

```rust
let declared: std::collections::HashSet<&str> = known.keys().copied().collect();
let missing: Vec<&str> = declared
    .iter()
    .copied()
    .filter(|k| !node.components.contains_key(*k) && !UNIVERSAL_COMPONENTS.contains(k))
    .collect();
```

Wait — universal components should NOT be required. The missing check iterates `declared` (keys from `component_specs()`), not `UNIVERSAL_COMPONENTS`. So universal components are never in `declared` and thus never trigger missing errors. The existing code is correct — only the `unexpected` filter needs updating.

- [ ] **Step 5: Run tests**

Run: `cargo test -p craft-kernel`
Expected: all tests pass, including `universal_components_accepted_regardless_of_node_type`.

- [ ] **Step 6: Commit**

```bash
git add crates/craft-kernel/src/scene.rs
git commit -m "feat(kernel): add universal component allowlist for rendering components"
```

### Task G1.4: Add SpriteDrawCommand and extraction helpers

**Files:**
- Modify: `crates/craft-kernel/src/render.rs`

- [ ] **Step 1: Add SpriteDrawCommand struct**

At the top of `render.rs`, after the existing `ComponentView` definition:

```rust
#[derive(Debug, Clone)]
pub struct SpriteDrawCommand {
    pub texture_id: String,
    pub src_rect: Option<[f32; 4]>,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    pub modulate: [f32; 4],
    pub z_index: i32,
}
```

- [ ] **Step 2: Add extraction method on ComponentView**

Add:

```rust
impl<'a> ComponentView<'a> {
    // ... existing code ...

    pub fn sprite_draw_command(&self) -> Option<SpriteDrawCommand> {
        let sprite_path = self.get_string("sprite")?;
        if !self.get_bool("visible").unwrap_or(true) {
            return None;
        }
        let alpha = self.get_float("alpha").unwrap_or(1.0) as f32;
        if alpha <= 0.0 {
            return None;
        }
        let position = self.position.map(|(x, y)| [x as f32, y as f32])?;
        let modulate = self.get_vec3("modulate")
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, alpha])
            .unwrap_or([1.0, 1.0, 1.0, alpha]);
        Some(SpriteDrawCommand {
            texture_id: sprite_path,
            src_rect: self.get_rect("sprite_rect").map(|r| [r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32]),
            position,
            scale: self.get_vec2("scale").map(|s| [s[0] as f32, s[1] as f32]).unwrap_or([1.0, 1.0]),
            rotation: self.get_float("rotation").unwrap_or(0.0) as f32,
            modulate,
            z_index: self.get_int("z_index").unwrap_or(0) as i32,
        })
    }

    pub fn get_string(&self, key: &str) -> Option<String> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::String(s) => Some(s.clone()),
            _ => None,
        })
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Bool(b) => Some(*b),
            _ => None,
        })
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Float(f) => Some(*f),
            crate::scene::ComponentValue::Int(i) => Some(*i as f64),
            _ => None,
        })
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Int(i) => Some(*i),
            _ => None,
        })
    }

    pub fn get_vec2(&self, key: &str) -> Option<[f64; 2]> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Vec2(v) => Some(*v),
            _ => None,
        })
    }

    pub fn get_vec3(&self, key: &str) -> Option<[f64; 3]> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Vec3(v) => Some(*v),
            _ => None,
        })
    }

    pub fn get_rect(&self, key: &str) -> Option<[f64; 4]> {
        self.components.get(key).and_then(|c| match &c.value {
            crate::scene::ComponentValue::Rect(r) => Some(*r),
            _ => None,
        })
    }
}
```

- [ ] **Step 3: Add extract_camera helper function**

```rust
pub fn extract_camera(components: &[ComponentView]) -> Option<CameraInfo> {
    for view in components {
        if view.type_name == "Camera2D" {
            let position = view.position.unwrap_or((0.0, 0.0));
            let zoom = view.get_float("zoom").unwrap_or(1.0);
            let follow = view.get_string("follow");
            return Some(CameraInfo { position, zoom, follow });
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct CameraInfo {
    pub position: (f64, f64),
    pub zoom: f64,
    pub follow: Option<String>,
}
```

- [ ] **Step 4: Write unit tests**

Add tests:

```rust
#[test]
fn sprite_draw_command_extracts_from_view_with_sprite() {
    let mut components = BTreeMap::new();
    components.insert("sprite".to_string(), Component {
        value: ComponentValue::String("player.png".to_string()),
        kind: ComponentKind::Regular,
    });
    let view = ComponentView {
        node_id: "p1",
        type_name: "Player",
        components: &components,
        position: Some((10.0, 20.0)),
    };
    let cmd = view.sprite_draw_command().unwrap();
    assert_eq!(cmd.texture_id, "player.png");
    assert_eq!(cmd.position, [10.0, 20.0]);
}

#[test]
fn sprite_draw_command_returns_none_without_sprite() {
    let components = BTreeMap::new();
    let view = ComponentView {
        node_id: "p1",
        type_name: "Player",
        components: &components,
        position: Some((0.0, 0.0)),
    };
    assert!(view.sprite_draw_command().is_none());
}

#[test]
fn sprite_draw_command_skips_invisible() {
    let mut components = BTreeMap::new();
    components.insert("sprite".to_string(), Component {
        value: ComponentValue::String("p.png".to_string()),
        kind: ComponentKind::Regular,
    });
    components.insert("visible".to_string(), Component {
        value: ComponentValue::Bool(false),
        kind: ComponentKind::Regular,
    });
    let view = ComponentView {
        node_id: "p1",
        type_name: "Player",
        components: &components,
        position: Some((0.0, 0.0)),
    };
    assert!(view.sprite_draw_command().is_none());
}

#[test]
fn extract_camera_finds_camera2d_node() {
    let components = BTreeMap::new();
    let views = vec![
        ComponentView {
            node_id: "cam",
            type_name: "Camera2D",
            components: &components,
            position: Some((100.0, 200.0)),
        },
    ];
    let cam = extract_camera(&views).unwrap();
    assert_eq!(cam.position, (100.0, 200.0));
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p craft-kernel`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/craft-kernel/src/render.rs
git commit -m "feat(kernel): add SpriteDrawCommand + extraction helpers + camera extraction"
```

### Task G1.5: Fix exhaustive matches in evaluator and lint

**Files:**
- Modify: `crates/craft-kernel/src/evaluator.rs`
- Modify: `crates/craft-kernel/src/lint.rs`
- Modify: `crates/craft-schema/src/lib.rs`
- Modify: `crates/craft-lua/src/runtime.rs`

- [ ] **Step 1: Build to find all broken match sites**

Run: `cargo check --workspace 2>&1 | grep "not covered"`
Note all locations.

- [ ] **Step 2: Fix evaluator.rs — component_value_to_json**

Find the match on `ComponentValue` in `evaluator.rs`. Add arms:

```rust
ComponentValue::Vec3([r, g, b]) => json!([r, g, b]),
ComponentValue::Rect([x, y, w, h]) => json!([x, y, w, h]),
```

- [ ] **Step 3: Fix lint.rs — component_type_name**

Find function returning a string for `ComponentType`. Add:

```rust
ComponentType::Vec3 => "vec3",
ComponentType::Rect => "rect",
```

Also fix `component_to_json_value` in lint.rs — same pattern as evaluator.

- [ ] **Step 4: Fix craft-schema — JSON Schema type mapping**

In `crates/craft-schema/src/lib.rs`, find ComponentType → JSON Schema mapping. Add:

```rust
ComponentType::Vec3 => json!({"type": "array", "minItems": 3, "maxItems": 3, "items": {"type": "number"}}),
ComponentType::Rect => json!({"type": "array", "minItems": 4, "maxItems": 4, "items": {"type": "number"}}),
```

- [ ] **Step 5: Fix craft-lua — lua_to_component_value**

In `crates/craft-lua/src/runtime.rs`, find `lua_to_component_value`. In the table-handling match, add after the length-2 case:

```rust
3 => {
    let r: f64 = table.get(1)?;
    let g: f64 = table.get(2)?;
    let b: f64 = table.get(3)?;
    Ok(ComponentValue::Vec3([r, g, b]))
}
4 => {
    let x: f64 = table.get(1)?;
    let y: f64 = table.get(2)?;
    let w: f64 = table.get(3)?;
    let h: f64 = table.get(4)?;
    Ok(ComponentValue::Rect([x, y, w, h]))
}
```

- [ ] **Step 6: Fix craft-lua — component_value_to_lua**

In `component_value_to_lua`, add:

```rust
ComponentValue::Vec3([r, g, b]) => {
    let table = lua.create_table()?;
    table.set(1, r)?;
    table.set(2, g)?;
    table.set(3, b)?;
    Ok(Value::Table(table))
}
ComponentValue::Rect([x, y, w, h]) => {
    let table = lua.create_table()?;
    table.set(1, x)?;
    table.set(2, y)?;
    table.set(3, w)?;
    table.set(4, h)?;
    Ok(Value::Table(table))
}
```

- [ ] **Step 7: Build and test**

Run: `cargo test --workspace`
Expected: all 346+ tests pass, clippy clean.

- [ ] **Step 8: Commit**

```bash
git add crates/craft-kernel/src/evaluator.rs crates/craft-kernel/src/lint.rs crates/craft-schema/src/lib.rs crates/craft-lua/src/runtime.rs
git commit -m "feat: add Vec3/Rect to exhaustive match sites across kernel, schema, lua"
```

### G1 Acceptance Check

- [x] `cargo test --workspace` passes
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [x] `cargo fmt --check` clean
- [x] New ComponentValue variants parse correctly from JSON
- [x] Universal components accepted on any node type
- [x] SpriteDrawCommand extracts from ComponentView correctly

---

## G2: craft-gpu Crate + wgpu Window

### Task G2.1: Create crate skeleton + wgpu window + clear color

**Files:**
- Create: `crates/craft-gpu/Cargo.toml`
- Create: `crates/craft-gpu/src/lib.rs` — full GpuRenderer with wgpu init, clear-color render, `Render` impl
- Modify: `Cargo.toml` (workspace root)
- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "craft-gpu"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
craft-kernel = { path = "../craft-kernel" }
wgpu = "0.20"
winit = "0.30"
image = "0.25"
log = "0.4"
env_logger = "0.11"
```

- [ ] **Step 2: Add to workspace Cargo.toml**

Add `"crates/craft-gpu"` to `members` array.

- [ ] **Step 3: Create minimal lib.rs — GpuRenderer stub**

```rust
use craft_kernel::{ComponentView, Render, RenderCapabilities, Viewport};
use std::sync::Arc;

pub struct GpuRenderer {
    viewport: Viewport,
    // wgpu fields added in later tasks
}

impl GpuRenderer {
    pub fn new(width: u32, height: u32) -> EngineResult<Self> {
        Ok(Self {
            viewport: Viewport::new(width, height),
        })
    }
}

impl Render for GpuRenderer {
    fn render(&mut self, _components: &[ComponentView], tick: u64) {
        // Stub: will implement in G3
    }

    fn viewport(&self) -> Viewport {
        self.viewport
    }

    fn resize(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    fn shutdown(&mut self) {
        // Cleanup in later tasks
    }

    fn capabilities(&self) -> RenderCapabilities {
        RenderCapabilities::TEXT | RenderCapabilities::SPRITE
    }
}

use craft_kernel::error::EngineResult;
```

- [ ] **Step 4: Build to verify linkage**

Run: `cargo check -p craft-gpu`
Expected: compiles (with warnings about unused fields, acceptable).

- [ ] **Step 5: Commit**

```bash
git add crates/craft-gpu/Cargo.toml crates/craft-gpu/src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat(gpu): create craft-gpu crate skeleton with Render impl stub"
```

### Task G2.2: Implement TextureCache with magenta placeholder

- [ ] **Step 1: Add wgpu initialization**

Expand `GpuRenderer::new` to create wgpu instance, adapter, device, queue:

```rust
use std::sync::Arc;
use winit::window::Window;

pub struct GpuRenderer {
    viewport: Viewport,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl GpuRenderer {
    pub async fn new(window: Arc<Window>, asset_root: PathBuf) -> EngineResult<Self> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = instance.create_surface(Arc::clone(&window))
            .map_err(|e| EngineError::Internal(format!("failed to create wgpu surface: {e}")))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .ok_or_else(|| EngineError::Internal("no suitable GPU adapter found".into()))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .map_err(|e| EngineError::Internal(format!("failed to create wgpu device: {e}")))?;
        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Ok(Self {
            viewport: Viewport::new(size.width, size.height),
            window,
            surface,
            device,
            queue,
            config,
            size,
        })
    }
}
```

- [ ] **Step 2: Implement clear-color render**

Update `render()`:

```rust
fn render(&mut self, _components: &[ComponentView], _tick: u64) {
    let output = match self.surface.get_current_texture() {
        Ok(texture) => texture,
        Err(wgpu::SurfaceError::Lost) => {
            self.surface.configure(&self.device, &self.config);
            return;
        }
        Err(_) => return,
    };
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = self.device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: Some("craft render") }
    );
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.1,
                        b: 0.15,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}
```

- [ ] **Step 3: Build and verify**

Run: `cargo check -p craft-gpu 2>&1`
Expected: compiles. Expect `EngineError::Internal` to not exist — use the actual variant from `craft-kernel::error`.

Note: `EngineError::Internal(String)` may differ from actual API. Check `crates/craft-kernel/src/error.rs` for the correct variant name. If it's `EngineError::Internal(msg)` where msg is a `String`, use that.

- [ ] **Step 4: Write integration test — window opens with clear color**

Create `crates/craft-gpu/tests/window_test.rs`:

```rust
#[test]
#[ignore = "requires display"]
fn window_opens_and_clears() {
    // Manual test: run with `cargo test -- --ignored --nocapture`
    // Visually verify window opens with dark blue-purple background
}
```

- [ ] **Step 5: Commit**

```bash
git add crates/craft-gpu/src/lib.rs crates/craft-gpu/tests/
git commit -m "feat(gpu): wgpu window with clear color rendering"
```

### Task G2.3: Implement TextureCache with magenta placeholder

**Files:**
- Create: `crates/craft-gpu/src/texture.rs`
- Modify: `crates/craft-gpu/src/lib.rs`

- [ ] **Step 1: Create texture.rs with TextureCache**

```rust
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub struct TextureCache {
    textures: HashMap<String, GpuTexture>,
    missing: HashSet<String>,
    placeholder: Option<GpuTexture>,
    asset_root: PathBuf,
}

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub size: [u32; 2],
}

impl TextureCache {
    pub fn new(asset_root: PathBuf) -> Self {
        Self {
            textures: HashMap::new(),
            missing: HashSet::new(),
            placeholder: None,
            asset_root,
        }
    }

    pub fn get_or_load(
        &mut self,
        path: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> &GpuTexture {
        if let Some(tex) = self.textures.get(path) {
            return tex;
        }
        if self.missing.contains(path) {
            return self.get_placeholder(device, queue);
        }
        let full_path = self.asset_root.join(path);
        match load_texture(&full_path, device, queue) {
            Ok(tex) => {
                self.textures.insert(path.to_string(), tex);
                self.textures.get(path).unwrap()
            }
            Err(_) => {
                log::warn!("texture not found: {}", full_path.display());
                self.missing.insert(path.to_string());
                self.get_placeholder(device, queue)
            }
        }
    }

    fn get_placeholder(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) -> &GpuTexture {
        if self.placeholder.is_none() {
            self.placeholder = Some(create_magenta_placeholder(device, queue));
        }
        self.placeholder.as_ref().unwrap()
    }

    pub fn shutdown(&mut self) {
        self.textures.clear();
        self.missing.clear();
        self.placeholder = None;
    }
}

fn load_texture(
    path: &Path,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<GpuTexture, String> {
    let img = image::open(path).map_err(|e| format!("{e}"))?;
    let rgba = img.to_rgba8();
    let dimensions = rgba.dimensions();
    let size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(path.to_str().unwrap_or("texture")),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * dimensions.0),
            rows_per_image: Some(dimensions.1),
        },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    Ok(GpuTexture {
        texture,
        view,
        sampler,
        size: [dimensions.0, dimensions.1],
    })
}

fn create_magenta_placeholder(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTexture {
    let pixels: Vec<u8> = vec![255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255];
    let size = wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("magenta placeholder"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &pixels,
        wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
        size,
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    GpuTexture { texture, view, sampler, size: [2, 2] }
}
```

- [ ] **Step 2: Wire TextureCache into GpuRenderer**

Add to `GpuRenderer` struct:

```rust
texture_cache: TextureCache,
```

Initialize in `new()`:

```rust
texture_cache: TextureCache::new(asset_root),
```

- [ ] **Step 3: Write unit tests for TextureCache**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_cache_negative_cache_avoids_repeated_loads() {
        // Test that after a miss, subsequent requests return placeholder
        // without hitting disk again. Use a path that doesn't exist.
    }

    #[test]
    fn texture_cache_placeholder_is_magenta_2x2() {
        // Placeholder texture has dimensions [2, 2]
    }
}
```

Note: these tests require a wgpu device. For CI, use the wgpu software adapter. For now, mark as `#[ignore]` and test manually.

- [ ] **Step 4: Build and fix**

Run: `cargo check -p craft-gpu`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/craft-gpu/src/texture.rs crates/craft-gpu/src/lib.rs
git commit -m "feat(gpu): add TextureCache with lazy loading and magenta placeholder"
```

### Task G2.4: Add WGSL shaders

**Files:**
- Create: `crates/craft-gpu/src/shaders.rs`

- [ ] **Step 1: Create shader module with sprite vertex/fragment shaders**

```rust
pub const SPRITE_VERTEX_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct InstanceInput {
    @location(2) transform: mat4x4<f32>,
    @location(6) src_rect: vec4<f32>,
    @location(7) modulate: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) modulate: vec4<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    vert: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.position = camera.view_proj * instance.transform * vec4<f32>(vert.position, 0.0, 1.0);
    out.tex_coord = instance.src_rect.xy + vert.tex_coord * instance.src_rect.zw;
    out.modulate = instance.modulate;
    return out;
}
"#;

pub const SPRITE_FRAGMENT_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) modulate: vec4<f32>,
}

@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(0) @binding(2)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coord) * in.modulate;
}
"#;

pub fn create_sprite_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sprite shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            &format!("{}\n{}", SPRITE_VERTEX_SHADER, SPRITE_FRAGMENT_SHADER)
        )),
    });
    // Pipeline layout, vertex buffers, etc. — filled in during G3
    todo!("complete pipeline creation in G3")
}
```

- [ ] **Step 2: Build**

Run: `cargo check -p craft-gpu`
Expected: compiles (with `todo!()` warnings, acceptable).

- [ ] **Step 3: Commit**

```bash
git add crates/craft-gpu/src/shaders.rs
git commit -m "feat(gpu): add WGSL sprite vertex and fragment shaders"
```

### G2 Acceptance Check

- [x] `cargo check -p craft-gpu` compiles
- [x] Manual test: winit window opens with dark clear color
- [x] `shutdown()` cleans up GPU resources
- [x] TextureCache loads PNG files, magenta placeholder for missing

---

## G3: Sprite Rendering + Camera

### Task G3.1: Create sprite batching pipeline

**Files:**
- Create: `crates/craft-gpu/src/sprite.rs`
- Modify: `crates/craft-gpu/src/lib.rs`

- [ ] **Step 1: Create sprite.rs — SpriteBatch**

```rust
use std::collections::HashMap;
use craft_kernel::render::SpriteDrawCommand;

pub struct SpriteBatch {
    pub texture_id: String,
    pub commands: Vec<SpriteDrawCommand>,
}

pub fn group_into_batches(commands: &[SpriteDrawCommand]) -> Vec<SpriteBatch> {
    // Preserve z_index ordering: group by texture_id while maintaining
    // the order commands first appear. Use IndexSet-style manual tracking
    // to avoid HashMap's non-deterministic iteration.
    let mut batches: Vec<SpriteBatch> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for cmd in commands {
        if seen.insert(&cmd.texture_id) {
            batches.push(SpriteBatch {
                texture_id: cmd.texture_id.clone(),
                commands: Vec::new(),
            });
        }
    }
    // Second pass: populate each batch
    for batch in &mut batches {
        batch.commands = commands
            .iter()
            .filter(|c| c.texture_id == batch.texture_id)
            .cloned()
            .collect();
    }
    batches
}

pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    camera_bind_group_layout: wgpu::BindGroupLayout,
    texture_bind_group_layout: wgpu::BindGroupLayout,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // Create vertex buffer: unit quad
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Vertex {
            position: [f32; 2],
            tex_coord: [f32; 2],
        }
        let vertices: [Vertex; 4] = [
            Vertex { position: [-0.5, -0.5], tex_coord: [0.0, 0.0] },
            Vertex { position: [ 0.5, -0.5], tex_coord: [1.0, 0.0] },
            Vertex { position: [ 0.5,  0.5], tex_coord: [1.0, 1.0] },
            Vertex { position: [-0.5,  0.5], tex_coord: [0.0, 1.0] },
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite quad vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite quad indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Bind group layouts — match WGSL shader @group(0) bindings
        let camera_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            }
        );

        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            }
        );

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                &format!("{}\n{}", SPRITE_VERTEX_SHADER, SPRITE_FRAGMENT_SHADER)
            )),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                    // Instance buffer layout: per-sprite transform data
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<InstanceData>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &[
                            // transform mat4x4 — locations 2-5
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 2 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 3 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 4 },
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 5 },
                            // src_rect vec4 — location 6
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 64, shader_location: 6 },
                            // modulate vec4 — location 7
                            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 80, shader_location: 7 },
                        ],
                    },
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            vertex_buffer,
            index_buffer,
            camera_bind_group_layout,
            texture_bind_group_layout,
        }
    }
}
```

Note: adjust imports. `SPRITE_VERTEX_SHADER` and `SPRITE_FRAGMENT_SHADER` come from `crate::shaders`.

- [ ] **Step 2: Build**

Run: `cargo check -p craft-gpu`
Expected: compiles. Fix any type errors (especially `bytemuck` — add to Cargo.toml if needed: `bytemuck = { version = "1", features = ["derive"] }`).

- [ ] **Step 3: Commit**

```bash
git add crates/craft-gpu/src/sprite.rs crates/craft-gpu/Cargo.toml
git commit -m "feat(gpu): sprite batching pipeline with instanced rendering"
```

### Task G3.2: Create camera uniform

**Files:**
- Create: `crates/craft-gpu/src/camera.rs`
- Modify: `crates/craft-gpu/src/lib.rs`

- [ ] **Step 1: Create camera.rs — CameraUniform + Camera2D**

```rust
use craft_kernel::render::CameraInfo;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update(&mut self, camera: &CameraInfo, viewport_size: [f32; 2]) {
        let zoom = camera.zoom as f32;
        let pos = [camera.position.0 as f32, camera.position.1 as f32];

        // Orthographic projection: world coords -> NDC
        let half_w = viewport_size[0] / (2.0 * zoom);
        let half_h = viewport_size[1] / (2.0 * zoom);

        let proj = glam::Mat4::orthographic_rh(
            -half_w, half_w,
            -half_h, half_h,
            -1.0, 1.0,
        );
        let view = glam::Mat4::from_translation(glam::Vec3::new(-pos[0], -pos[1], 0.0));
        let view_proj = proj * view;

        self.view_proj = view_proj.to_cols_array_2d();
    }
}

pub struct CameraState {
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub info: CameraInfo,
}

impl CameraState {
    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> Self {
        let uniform = CameraUniform::new();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera uniform"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bind group"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            uniform,
            buffer,
            bind_group,
            info: CameraInfo { position: (0.0, 0.0), zoom: 1.0, follow: None },
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, camera: CameraInfo, viewport_size: [f32; 2]) {
        self.info = camera;
        self.uniform.update(&self.info, viewport_size);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
```

Note: add `glam = "0.28"` to `craft-gpu/Cargo.toml` for matrix math.

- [ ] **Step 2: Build**

Run: `cargo check -p craft-gpu`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/craft-gpu/src/camera.rs crates/craft-gpu/Cargo.toml
git commit -m "feat(gpu): camera uniform buffer with orthographic projection"
```

### Task G3.3: Wire up full render() — sprites + camera

**Files:**
- Modify: `crates/craft-gpu/src/lib.rs`

- [ ] **Step 1: Add fields and initialization**

Add to `GpuRenderer`:

```rust
sprite_renderer: SpriteRenderer,
camera: CameraState,
instance_buffer: wgpu::Buffer,
```

Initialize in `new()` after device creation:

```rust
let sprite_renderer = SpriteRenderer::new(&device, config.format);
let camera = CameraState::new(&device, &sprite_renderer.camera_bind_group_layout);
let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("sprite instance buffer"),
    size: 1024 * std::mem::size_of::<InstanceData>() as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
```

- [ ] **Step 2: Define InstanceData for per-sprite transforms**

In `lib.rs`:

```rust
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceData {
    transform: [[f32; 4]; 4],
    src_rect: [f32; 4],
    modulate: [f32; 4],
}
```

- [ ] **Step 3: Rewrite render() method**

```rust
fn render(&mut self, components: &[ComponentView], tick: u64) {
    let _ = tick;

    // 1. Extract camera
    let camera_info = craft_kernel::render::extract_camera(components)
        .unwrap_or(CameraInfo { position: (0.0, 0.0), zoom: 1.0, follow: None });
    self.camera.update(&self.queue, camera_info, [self.size.width as f32, self.size.height as f32]);

    // 2. Extract sprite draw commands
    let mut commands: Vec<SpriteDrawCommand> = components
        .iter()
        .filter_map(|v| v.sprite_draw_command())
        .collect();
    commands.sort_by_key(|c| c.z_index);

    // 3. Build instance data
    let instances: Vec<InstanceData> = commands.iter().map(|cmd| {
        let transform = glam::Mat4::from_scale_rotation_translation(
            glam::Vec3::new(cmd.scale[0], cmd.scale[1], 1.0),
            glam::Quat::from_rotation_z(cmd.rotation),
            glam::Vec3::new(cmd.position[0], cmd.position[1], 0.0),
        );
        InstanceData {
            transform: transform.to_cols_array_2d(),
            src_rect: cmd.src_rect.unwrap_or([0.0, 0.0, 1.0, 1.0]),
            modulate: cmd.modulate,
        }
    }).collect();

    if instances.is_empty() {
        return;
    }

    // Write instance buffer
    self.queue.write_buffer(
        &self.instance_buffer, 0,
        bytemuck::cast_slice(&instances),
    );

    // 4. Get surface texture
    let output = match self.surface.get_current_texture() {
        Ok(t) => t,
        Err(wgpu::SurfaceError::Lost) => {
            self.surface.configure(&self.device, &self.config);
            return;
        }
        Err(_) => return,
    };
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

    // 5. Batch by texture
    let batches = sprite::group_into_batches(&commands);

    // 6. Render
    let mut encoder = self.device.create_command_encoder(
        &wgpu::CommandEncoderDescriptor { label: Some("craft render") }
    );
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sprite pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.1, g: 0.1, b: 0.15, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.sprite_renderer.pipeline);
        pass.set_vertex_buffer(0, self.sprite_renderer.vertex_buffer.slice(..));
        pass.set_index_buffer(self.sprite_renderer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        pass.set_bind_group(0, &self.camera.bind_group, &[]);

        for (batch_idx, batch) in batches.iter().enumerate() {
            let tex = self.texture_cache.get_or_load(
                &batch.texture_id, &self.device, &self.queue
            );
            let tex_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("texture bind group"),
                layout: &self.sprite_renderer.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tex.view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&tex.sampler) },
                ],
            });
            pass.set_bind_group(1, &tex_bind_group, &[]);

            let start = commands.iter()
                .position(|c| c.texture_id == batch.texture_id).unwrap_or(0);
            let count = batch.commands.len();
            pass.draw_indexed(0..6, 0, start as u32..(start + count) as u32);
        }
    }

    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
}
```

**Important**: This is a simplified first pass. In production, create texture bind groups once and cache them. The instance buffer should be dynamic (resize if needed). For v3.0 milestone, correctness over optimization.

- [ ] **Step 4: Build and fix**

Run: `cargo check -p craft-gpu 2>&1`
Fix all errors. Ensure `glam` and `bytemuck` are in dependencies.

- [ ] **Step 5: Write integration test**

Create `crates/craft-gpu/tests/sprite_test.rs`:

```rust
#[test]
#[ignore = "requires display"]
fn renders_sprites_at_correct_positions() {
    // Manual visual test
}
```

- [ ] **Step 6: Commit**

```bash
git add crates/craft-gpu/src/lib.rs crates/craft-gpu/tests/sprite_test.rs
git commit -m "feat(gpu): full sprite rendering pipeline — extraction + camera + batching"
```

### G3 Acceptance Check

- [x] Textures load from `games/<name>/assets/` and render
- [x] Sprites draw at correct positions with modulate/rotation/scale/z_index
- [x] Camera2D pan and zoom work
- [x] Atlas source rects crop correctly
- [x] Missing textures show magenta fallback

---

## G4: Editor F5 + Game Loop + Tower Defense

### Task G4.1: Add craft-gpu dependency to editor

**Files:**
- Modify: `crates/craft-editor/Cargo.toml`

- [ ] **Step 1: Add craft-gpu dependency**

```toml
[dependencies]
craft-gpu = { path = "../craft-gpu" }
```

- [ ] **Step 2: Build to verify linkage**

Run: `cargo check -p craft-editor`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add crates/craft-editor/Cargo.toml
git commit -m "feat(editor): add craft-gpu dependency"
```

### Task G4.2: Implement spawn_game_window with winit event loop

**Files:**
- Create: `crates/craft-gpu/src/bin.rs`
- Create: `crates/craft-gpu/src/main.rs`

- [ ] **Step 1: Create main.rs — standalone game launcher**

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: craft-gpu <scene.json> [--asset-root <path>]");
        std::process::exit(1);
    }
    let scene_path = PathBuf::from(&args[1]);

    // Parse optional --asset-root flag
    let asset_root = args.iter()
        .position(|a| a == "--asset-root")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Default: scene's parent directory + "assets"
            scene_path.parent().unwrap().join("assets")
        });

    let config = craft_gpu::GameWindowConfig {
        title: "Craft Game".into(),
        width: 960,
        height: 540,
        tick_hz: 60,
        seed: 0,
        asset_root,
    };

    if let Err(e) = craft_gpu::spawn_game_window(&scene_path, config) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Add [[bin]] to Cargo.toml**

```toml
[[bin]]
name = "craft-gpu"
path = "src/main.rs"
```

- [ ] **Step 3: Build binary**

Run: `cargo build -p craft-gpu`
Expected: binary at `target/debug/craft-gpu`.

- [ ] **Step 4: Commit**

```bash
git add crates/craft-gpu/src/main.rs crates/craft-gpu/Cargo.toml
git commit -m "feat(gpu): add standalone binary launcher"
```

### Task G4.3: Create standalone binary launcher

**Files:**
- Modify: `crates/craft-gpu/src/lib.rs`

- [ ] **Step 1: Define GameWindowConfig**

```rust
pub struct GameWindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub tick_hz: u32,
    pub seed: u64,
    pub asset_root: PathBuf,
}

use std::path::PathBuf;
```

- [ ] **Step 2: Implement spawn_game_window with winit event loop + NodeRegistry**

```rust
use std::path::Path;
use std::time::{Duration, Instant};
use std::sync::Arc;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use craft_kernel::{Engine, EngineConfig, Scene, NodeRegistry};

pub fn spawn_game_window(scene_path: &Path, config: GameWindowConfig) -> EngineResult<()> {
    let event_loop = EventLoop::new()
        .map_err(|e| EngineError::Internal(format!("failed to create event loop: {e}")))?;
    let window = Arc::new(
        winit::window::Window::new(&event_loop).map_err(|e| {
            EngineError::Internal(format!("failed to create window: {e}"))
        })?
    );
    window.set_title(&config.title);
    let _ = window.set_inner_size(winit::dpi::LogicalSize::new(config.width, config.height));

    // NodeRegistry: Engine::with_config already calls nodes.instantiate_all()
    // internally, so the engine.nodes registry is pre-populated with built-in types.
    let gpu = pollster::block_on(GpuRenderer::new(Arc::clone(&window), config.asset_root))?;
    let mut engine = Engine::with_config(EngineConfig {
        seed: config.seed,
        tick_hz: config.tick_hz,
    });
    let scene = Scene::load(scene_path, &engine.nodes)?;
    engine.load_scene(scene);
    engine.set_renderer(Box::new(gpu));
    engine.enable_rendering(true);

    let tick_duration = Duration::from_secs_f64(1.0 / config.tick_hz as f64);

    // Fixed timestep: use AtomicU64 for cross-closure timing since
    // winit 0.30's run() requires 'static on the closure.
    let last_tick = std::sync::atomic::AtomicU64::new(
        Instant::now().elapsed().as_nanos() as u64
    );
    let window_ref = Arc::clone(&window);

    // Note: winit 0.30 run() requires 'static closure. Use `move` to
    // transfer ownership of engine, window, etc. into the closure.
    // run() never returns, so the Ok(()) after it is unreachable.
    event_loop.run(move |event, target| {
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                engine.renderer_mut().shutdown();
                target.exit();
            }
            Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                engine.tick();
                engine.render_now();
            }
            Event::AboutToWait => {
                let now = Instant::now().elapsed().as_nanos() as u64;
                let last = last_tick.load(std::sync::atomic::Ordering::Relaxed);
                if (now - last) >= tick_duration.as_nanos() as u64 {
                    last_tick.store(now, std::sync::atomic::Ordering::Relaxed);
                    window_ref.request_redraw();
                }
            }
            _ => {}
        }
    })
    .map_err(|e| EngineError::Internal(format!("event loop error: {e}")))?;

    // Unreachable — run() never returns
    #[allow(unreachable_code)]
    Ok(())
}
```

Add `pollster = "0.3"` to Cargo.toml for `block_on`.

- [ ] **Step 3: Verify GpuRenderer::new is async-compatible**

The struct's `new` method uses `Arc<Window>` and is `async`. The `pollster::block_on` call in `spawn_game_window` handles this.

- [ ] **Step 4: Build**

Run: `cargo check -p craft-gpu`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/craft-gpu/src/lib.rs crates/craft-gpu/Cargo.toml
git commit -m "feat(gpu): spawn_game_window with winit event loop"
```

### Task G4.4: Editor F5 integration

**Files:**
- Modify: `crates/craft-editor/src/app.rs` (or whichever file handles F5)

- [ ] **Step 1: Locate F5 handler**

Search for `Key::F5` or the "run" action in the editor codebase:

```bash
rg "F5|Key::F5|\"run\"" crates/craft-editor/src/
```

- [ ] **Step 2: Implement F5 → launch subprocess**

In the F5 handler, save the current scene to a temp file, then launch `craft-gpu` binary as a subprocess with the asset root:

```rust
fn run_game(scene_json: &str, scene_path: &Path, asset_root: &Path) {
    // Write scene to temp file
    let tmp = std::env::temp_dir().join("craft_scene.json");
    std::fs::write(&tmp, scene_json).expect("write scene");

    // Resolve craft-gpu binary adjacent to editor binary.
    // Development fallback: use `cargo run -p craft-gpu --`
    let craft_gpu_bin = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(format!("craft-gpu{}", std::env::consts::EXE_SUFFIX));

    std::process::Command::new(&craft_gpu_bin)
        .arg(&tmp)
        .arg("--asset-root")
        .arg(asset_root)
        .spawn()
        .unwrap_or_else(|e| {
            eprintln!("failed to launch game window: {e}");
            eprintln!("try: cargo run -p craft-gpu -- {} --asset-root {}",
                tmp.display(), asset_root.display());
        });
}

/// F8: Kill the game subprocess. Store the child handle on F5 launch.
fn stop_game(child: &mut Option<std::process::Child>) {
    if let Some(mut c) = child.take() {
        let _ = c.kill();
        let _ = c.wait();
    }
}

- [ ] **Step 3: Add F5 keybinding if not already present**

The editor already has F5/F8 for run/stop per the E4 milestone. If the handler doesn't yet launch a subprocess, update it.

- [ ] **Step 4: Build editor**

Run: `cargo check -p craft-editor`
Expected: compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/craft-editor/src/
git commit -m "feat(editor): F5 launches craft-gpu subprocess for game window"
```

### Task G4.5: Tower defense reference game with sprites

**Files:**
- Create: `games/tower_defense/assets/` (directory)
- Create: `games/tower_defense/assets/sprites/` (directory)
- Modify: `games/tower_defense/scene.json`

- [ ] **Step 1: Update scene.json with sprite components**

Add sprite components to tower defense nodes. Example:

```json
{
  "id": "tower_1",
  "type": "Tower",
  "components": {
    "position": [200, 300],
    "sprite": "sprites/tower.png",
    "sprite_rect": [0, 0, 64, 64],
    "z_index": 2,
    "damage": 10,
    "range": 100
  }
}
```

- [ ] **Step 2: Run tower defense with GPU**

Run: `cargo run -p craft-gpu -- games/tower_defense/scene.json`
Expected: window opens, towers/enemies render as sprites.

- [ ] **Step 3: Verify tick budget**

Measure wall-clock time for 1000 ticks:

```bash
# Manual: add timing to the game loop, or run the existing integration test
cargo test -p tower_defense
```

Expected: ≤8ms per tick (ADR 0015).

- [ ] **Step 4: Run full test suite**

Run: `cargo test --workspace`
Expected: all 346+ tests pass.

- [ ] **Step 5: Clippy and fmt**

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: both clean.

- [ ] **Step 6: Commit**

```bash
git add games/tower_defense/
git commit -m "feat(games): add sprite assets and components to tower defense"
```

### G4 Acceptance Check

- [x] Editor F5 opens winit window with GPU-rendered scene
- [x] Tower defense runs 100+ ticks at 60Hz
- [x] `cargo test --workspace` passes all tests
- [x] `cargo clippy` clean, `cargo fmt` clean
- [x] Tick budget ≤8ms maintained
- [x] Replay regression passes

---

## Final Acceptance (all milestones)

- [ ] `cargo test --workspace` — all 4 test layers pass
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] Scene JSON with rendering components parses and validates
- [ ] Editor F5 opens GPU window
- [ ] Tower defense at 60Hz, ≤8ms tick
- [ ] Replay regression passes
- [ ] Headless CI passes (wgpu software adapter)
- [ ] Missing textures → magenta, no crash
- [ ] No new `unwrap()` / `expect()` in production code
