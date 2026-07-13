# ADR 0005: Schema Pipeline — Compile-Time JSON Schema Generation

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: Godot's runtime ClassDB reflection + `MethodBind` system

## Context

PRD requirement: "Schema is a first-class product. Every Rust type — including every action verb — emits JSON Schema. TypeScript SDK types are auto-generated. The agent's API view cannot drift from engine reality."

Godot achieves cross-language introspection through `ClassDB`: a runtime registry of classes, methods, properties, and signals, populated by `GDCLASS` macros at startup and queried by scripting languages (GDScript, C#, GDExtension) via `method_bind.h`. This is a runtime reflection system built on C++ macros and template metaprogramming.

Craft needs the same goal (agent can discover the full API surface) but through a different mechanism: compile-time JSON Schema generation, since there's no runtime scripting VM that needs dynamic dispatch.

## Decision

**`schemars` crate as the foundation, with a thin `craft-schema` extension layer for Craft-specific metadata (`transient`, `node_type` descriptions, action enum descriptions).**

Pipeline:
```
Rust type (craft-kernel)
    │  #[derive(JsonSchema)] + #[node_type(...)] + #[component(...)]
    ▼
JSON Schema (schemars output)
    │  craft-schema::extend() — inject transient metadata, descriptions, action vocabulary
    ▼
Enriched JSON Schema (draft-07)
    │  craft-bridge::codegen() — schema → TypeScript type definitions
    ▼
TypeScript types (sdk/src/generated.ts)
    │  Agent imports and uses
    ▼
Agent-side type checking (tsc) + engine-side validation (jsonschema)
```

## Rationale

1. **`schemars` is the ecosystem standard**: Handles 90% of the work: Rust primitives, structs, enums, `Vec`, `HashMap`, `Option`, `serde` attributes. Well-maintained, widely used.

2. **Thin extension layer, not a custom proc-macro**: Writing a full JSON Schema derive macro from scratch is a sub-project in itself (handling generics, lifetimes, attribute parsing, schema draft compliance). The `craft-schema` crate only adds what schemars can't express: component kind metadata, node type descriptions, and action vocabulary documentation.

3. **Schema is the single source of truth**: TypeScript types are generated from the same schema that validates agent input. Drift is impossible — the schema *is* the API.

4. **Standard format (JSON Schema draft-07)**: Agents and tools can use off-the-shelf validators (ajv, jsonschema-rs) without understanding Craft internals.

## Godot Mapping

| Godot | Craft |
|-------|-------|
| `GDCLASS` macro (compile-time registration) | `#[derive(JsonSchema)]` + `craft_node!` (compile-time) |
| `ClassDB::bind_method()` (runtime binding) | No runtime binding — schema describes action vocabulary |
| `MethodBind` + `call()` (dynamic dispatch) | JSON actions — deserialized by serde, validated by jsonschema |
| `core/extension/gdextension_interface.gen.h` (C ABI) | `sdk/src/generated.ts` (TypeScript types from schema) |
| GDExtension compatibility hash | No hash needed — schema IS the compatibility contract |
| Variant as interchange type | JSON (schema-validated) + ComponentValue enum |

## Error Handling in the Pipeline

Schema validation failures produce structured errors:
```json
{
  "category": "validation",
  "errors": [
    {
      "file": "scene.json",
      "json_path": "$.nodes.enemy.components.speed",
      "expected_type": "number",
      "actual_value": "\"fast\"",
      "suggestion": "Replace string with a numeric value, e.g. 5.0",
      "auto_fixable": "suggested"
    }
  ]
}
```

This is directly consumable by an agent — no stack trace parsing, no grep through stderr.
