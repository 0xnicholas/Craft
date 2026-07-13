# ADR 0004: Render Trait — Minimal Abstraction

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: Godot's `RenderingServer` (100+ method abstraction over Vulkan/Metal/D3D12/GLES3)

## Context

Godot's rendering architecture is a deep abstraction: `RenderingServer` defines an abstract GPU API (draw lists, RID allocation, materials, meshes, lights, environments, viewports, compositor effects). Four backends implement it: Vulkan (`drivers/vulkan/`), Metal (`drivers/metal/`), D3D12 (`drivers/d3d12/`), and GLES3 (`drivers/gles3/`). The server itself is 66+ files with 180K+ lines of C++.

Craft v1 has a single rendering target: an ANSI terminal. v2 may add 2D and 3D rendering. The question is whether to replicate Godot's server abstraction (trait with many methods, general enough for any backend) or define a minimal trait and extend it when new backends arrive.

## Decision

**A 4-method `Render` trait that receives `&[ComponentView]` — a flat list of typed component snapshots.**

```rust
pub trait Render: Send {
    fn render(&mut self, components: &[ComponentView], tick: u64);
    fn viewport(&self) -> Viewport;
    fn resize(&mut self, viewport: Viewport);
    fn shutdown(&mut self);
}

pub struct ComponentView<'a> {
    pub node_id: NodeId,
    pub type_name: &'a str,
    pub components: &'a HashMap<String, Component>,
}

pub struct Viewport {
    pub width: u32,   // character columns
    pub height: u32,  // character rows
}
```

The trait is defined in `craft-kernel`. `craft-terminal` is the v1 implementation.

## Rationale

1. **v1 only has one backend**: Designing a general GPU abstraction before having a second backend to validate it is premature. Godot's RenderingServer was built incrementally over years of adding backends.

2. **Flat `ComponentView` slice, not `&SceneTree`**: The renderer doesn't need to know about node hierarchy, behaviors, signals, or transient component lifecycle. Passing only the data it needs enforces the separation and enables testing with synthetic ComponentView vectors.

3. **Extension, not explosion**: When v2 adds 2D rendering, the trait grows methods (e.g., `render_sprite`, `render_text`). It doesn't fragment into multiple traits. This is YAGNI applied at the trait boundary.

4. **`NullRenderer` for testing**: The trait object approach lets tests inject a no-op renderer, making replay regression tests run without a terminal.

## Godot Mapping

| Godot | Craft |
|-------|-------|
| `RenderingServer` (abstract, 100+ methods) | `Render` trait (4 methods) |
| `servers/rendering/` (180K lines) | `craft-terminal/` (~300 lines) |
| Vulkan/Metal/D3D12/GLES3 backends | Single terminal backend |
| RID-based resource allocation | Not needed — no GPU resources |
| Draw lists, passes, compositor effects | Raw character grid → ANSI escape codes |
| Multi-threaded render thread | Single-threaded (v1 invariant) |

## Extension Seam — `RenderCapabilities` for v2

The 4-method trait will break when v2 adds 2D/3D rendering. To avoid a breaking change, the trait includes a **capability query** from v1:

```rust
pub trait Render: Send {
    fn render(&mut self, components: &[ComponentView], tick: u64);
    fn viewport(&self) -> Viewport;
    fn resize(&mut self, viewport: Viewport);
    fn shutdown(&mut self);

    /// Returns the set of rendering features this backend supports.
    /// Terminal backend returns TEXT only. GPU backend returns TEXTURE, SHADER, etc.
    fn capabilities(&self) -> RenderCapabilities { RenderCapabilities::TEXT }
}

bitflags! {
    pub struct RenderCapabilities: u32 {
        const TEXT    = 1 << 0;  // ANSI/character rendering (all backends)
        const SPRITE  = 1 << 1;  // 2D sprite rendering (v2)
        const SHADER  = 1 << 2;  // Custom shader support (v2)
        const MESH    = 1 << 3;  // 3D mesh rendering (v3)
    }
}
```

The engine queries `capabilities()` to decide which scene nodes to feed to the renderer. If a backend declares `SPRITE`, the engine includes sprite draw data alongside `ComponentView`. If not, it skips them. This keeps the 4 core methods stable while letting backends declare what they support — no trait method explosion, no breaking change.

## Rejected Alternatives

### Abstract trait from v1 with canvas/scene/overlay layers
Premature for a single backend. The right abstractions emerge from multiple implementations, not from up-front design.

### Concrete renderer with no trait, refactor later
The PRD explicitly calls for a Render trait boundary. It costs almost nothing to define (4 methods, a few lines) and gives us a clean seam for testing and future backends.

### Godot-style RenderingServer (drawing API with RIDs)
Over-engineered for terminal rendering. If v2 needs this level of abstraction, we can introduce a `Renderer2D` or `Renderer3D` trait alongside (not inside) the existing `Render` trait.
