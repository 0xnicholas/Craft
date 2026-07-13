# ADR 0002: Node Model — Property Bag + Tree

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: Godot's `GDCLASS` macro + `Object::cast_to<T>()` inheritance model

## Context

Godot defines ~1,811 classes through `GDCLASS` macros that generate ClassDB entries, virtual method bindings, and inheritance chains (`Node → CanvasItem → Node2D → Sprite2D`). This deep OOP hierarchy relies on:

- Virtual inheritance + `static_cast` for type traversal
- `Object::cast_to<T>()` for runtime downcasting
- `MethodBind` for string-keyed dynamic dispatch
- Intrusive reference counting (`RefCounted`)
- `Variant` as a universal tagged union bridging C++ and scripting

None of these patterns have clean Rust equivalents. The `object.h` source itself comments: *"The following is an incomprehensible blob of hacks and workarounds to compensate for many of the fallacies in C++."*

## Decision

**Unified `Node` struct with a property-bag component system** — no inheritance, no trait objects, no downcasting.

```rust
pub struct Node {
    pub id: NodeId,                   // generational index
    pub type_name: String,            // "Player", "Enemy", "Tower"
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub components: HashMap<String, Component>,
    pub behaviors: Vec<Behavior>,
}

pub struct Component {
    pub value: ComponentValue,
    pub default: ComponentValue,
    pub kind: ComponentKind,          // Regular | Transient { lifetime, remaining }
}

pub enum ComponentValue { Nil, Bool(bool), Int(i64), Float(f64), String(String), Vec2([f64; 2]), ... }
```

Node types differ only in their component key sets, defined declaratively via the `craft_node!` macro:

```rust
craft_node!(Player, {
    components: {
        position: Vec2 = [0.0, 0.0],
        health: Int = 100,
        damage_flash: transient String = "normal" @ 5 ticks,
    },
});
```

The engine stores all nodes homogenously. Type safety is enforced at *load time* by schema validation, not at *compile time* by the type system.

## Rationale

1. **Maps 1:1 to the agent's JSON**: An agent writes `{ "type": "Player", "components": { "position": [10,10] } }` and the engine's internal representation is structurally identical. No translation layer.

2. **No class hierarchy needed**: Godot's deep inheritance exists to share implementation (e.g., `Node2D` adds transform, `Sprite2D` adds texture). In Craft, all shared behavior is expressed through components and declarative rules, not C++ method dispatch.

3. **Runtime type checking is a feature, not a bug**: Schema validation produces structured errors with JSON paths and suggestions. An agent can read `expected: "integer", actual: "\"fast\""` and auto-correct. This is more actionable to an LLM than a Rust compile error.

4. **Eliminates the entire Variant/ClassDB/MethodBind subsystem**: Godot's `core/` is 170K lines; a significant fraction is the Object system. Craft replaces it with a single `HashMap<String, ComponentValue>` and compile-time JSON Schema generation.

5. **SlotMap for generational indexing**: `NodeId` uses a generational index to prevent ABA problems when nodes are despawned and re-spawned with the same index. Hot reload guarantees ID stability.

## Godot Mapping

| Godot Pattern | Craft Replacement |
|---------------|-------------------|
| `GDCLASS(MyNode, Node2D)` | `craft_node!(MyNode, { components: {...} })` |
| `node->get("position")` → Variant | `node.components.get("position")` → Option<&ComponentValue> |
| `node->call("method", args...)` | JSON-declared actions + `craft_system!` functions |
| `Object::cast_to<Sprite2D>()` | Not needed — all nodes are `Node`, types differ by component keys |
| `memnew()` / `memdelete()` | `SceneTree::spawn()` / `SceneTree::despawn()` |
| `_get_property_list()` | `craft_node!` macro generates schema |
| `Variant` (989-line header) | `ComponentValue` enum (compact, ~15 variants for v1) |

## Rejected Alternatives

### Trait-object nodes (`dyn Node`)
Requires `as_any()` downcasting boilerplate per type. Offers compile-time safety but adds ceremony for every new node type. Worse: the agent doesn't benefit from Rust compile errors — it sees runtime validation errors regardless.

### Pure ECS (archetype-based, Bevy-style)
Too much indirection for a scene tree that needs explicit parent/child hierarchy, depth-first traversal ordering, and per-node behavioral state machines. ECS is great for query-driven systems; Craft is tree-driven.

### Runtime-typed enum
Every node type would require modifying the engine's source enum. Not viable for user-defined types.

## Component Dependency Declaration

Property-bag components are independent by default, but some components have implicit dependencies (e.g., a `Sprite` component is meaningless without a `Transform` component, `Collision` needs `Hitbox`). These are not enforced at the type-system level but declared in the schema — enabling lint-time detection, not runtime crashes.

```rust
craft_node!(Player, {
    components: {
        position: Vec2 = [0.0, 0.0],         // independent
        sprite: String = "player.asc",        // #[requires(position)] — lint warns if missing
        hitbox: Vec2 = [1.0, 1.0],          // #[requires(position)]
    },
});
```

The `requires` declaration:
- **Lint-time only** — does not affect component storage or tick behavior
- **Schema-visible** — surfaced in JSON Schema so agents see dependency constraints
- **Warning, not error** — missing a dependency emits a lint warning, not a validation error (allows iterative authoring)
- **Transitive** — if sprite requires position and hitbox requires sprite, hitbox transitively requires position

This gives the structural safety of class hierarchies (can't have a Sprite without a Transform) without the inheritance.
