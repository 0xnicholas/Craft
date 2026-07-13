# ADR 0011: Expression Vocabulary

**Date**: 2026-07-13
**Status**: Accepted
**Reference**: PRD §6.5 — closed-set structured expressions for `cond`, `guard`, `value`, `by`, `args`

## Context

The PRD defines a closed expression system used in `if` conditions, state machine guards, `set_state` values, `move` deltas, `animate` targets, and `call_system` arguments. Expressions must be **structured JSON objects**, not free-form text. This is the AI-native choice — schema-validated data, not parseable strings.

The expression vocabulary is separate from the action vocabulary. Expressions appear *inside* actions (as values, conditions, arguments).

## Decision

**A closed set of 7 expression operators, represented as Rust enum with `#[serde(untagged)]` for the expression/string dual representation.**

```rust
/// A structured expression. The canonical JSON representation is an object like
/// `{ "ref": "self.position" }`, but bare strings are accepted as shorthand
/// for `{ "ref": <string> }`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Expression {
    /// Shorthand: bare string → Ref
    ShortRef(String),

    /// Full form (used when the expression is an object)
    Full(ExpressionNode),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpressionNode {
    /// Reference a component value: `{ "ref": "nodeId.componentKey" }`
    Ref {
        #[serde(rename = "ref")]
        path: String,
    },

    /// Equality comparison: `{ "eq": [a, b] }`
    Eq {
        eq: Vec<Expression>,
    },

    /// Inequality comparison: `{ "neq": [a, b] }`
    Neq {
        neq: Vec<Expression>,
    },

    /// Less-than comparison: `{ "lt": [a, b] }`
    Lt {
        lt: Vec<Expression>,
    },

    /// Greater-than comparison: `{ "gt": [a, b] }`
    Gt {
        gt: Vec<Expression>,
    },

    /// Addition: `{ "add": [a, b] }`
    Add {
        add: Vec<Expression>,
    },

    /// Subtraction: `{ "sub": [a, b] }`
    Sub {
        sub: Vec<Expression>,
    },
}
```

### Expression Evaluation

```rust
impl Expression {
    /// Evaluate an expression against the current scene state.
    /// Returns `ComponentValue` or an evaluation error.
    pub fn evaluate(&self, ctx: &EvalContext) -> Result<ComponentValue, ExprError>;
}

pub struct EvalContext<'a> {
    pub current_node: NodeId,          // "self" resolves to this
    pub tree: &'a SceneTree,           // for cross-node refs
    pub resources: &'a ResourceRegistry,
}

pub enum ExprError {
    UndefinedRef { path: String },
    TypeMismatch { path: String, expected: &'static str, actual: &'static str },
    DivisionByZero,
}
```

### Bare String Shorthand

In JSON, a bare string in expression position is sugar for `{ "ref": <string> }`:

```json
// These are equivalent:
"self.position"
{ "ref": "self.position" }

// These are also equivalent:
{ "add": ["self.score", 1] }
{ "add": [{ "ref": "self.score" }, 1] }
```

The engine normalizes shorthand to `Expression::ShortRef` during parse, and the evaluator treats both identically.

### Reserved Tokens

| Token | Meaning |
|-------|---------|
| `self` | The current node (the node whose behavior is being evaluated) |
| `none` | Null / absent value. Dangling refs resolve to `none` |
| `true` / `false` | Boolean literals |
| Integer literals | e.g., `1`, `-5`, `0` → `ComponentValue::Int` |
| Decimal literals | e.g., `1.5`, `-0.3` → `ComponentValue::Float` |
| `"string literals"` | JSON strings that don't match a ref pattern → `ComponentValue::String` |

### SetState with Expression Value

When `set_state` has an expression as its value, the engine evaluates the expression against the current state, then writes the result:

```json
// Increment score: read self.score, add 1, write back
{ "kind": "set_state", "target": "self", "key": "score", "value": { "add": ["self.score", 1] } }

// Conditional death check
{ "kind": "if", "cond": { "lt": ["self.health", 1] },
  "then": [
    { "kind": "set_state", "target": "self", "key": "dead", "value": true }
  ]
}
```

Read-then-write of the same component in `set_state` is valid and deterministic — the expression's `ref` reads the value *before* the write.

### Type Validation

Expressions are type-checked at evaluation time:

- Comparison operators (`eq`, `neq`, `lt`, `gt`) require compatible types on both sides
- Arithmetic operators (`add`, `sub`) require numeric operands
- `ref` paths are validated: component key must exist, type must match usage context
- Boolean operators are excluded (use `if`/`then`/`else` for control flow, not boolean expressions)

### Schema Exposure

`engine.getSchema()` returns the expression vocabulary under `expressions`:

```json
{
  "expressions": {
    "operators": ["ref", "eq", "neq", "lt", "gt", "add", "sub"],
    "reserved_tokens": ["self", "none", "true", "false"],
    "type_rules": { "eq": "compatible_types", "add": "both_numeric", ... }
  }
}
```

## Rationale

1. **No free-form expressions**: A grammar-based DSL (`self.score + 1`) requires a parser, has ambiguous syntax, and defeats JSON Schema validation. Structured expressions are schema-validatable.

2. **Shorthand sugar for the common case**: `"self.position"` as a bare string covers 80% of expression usage (reading a component). The verbose `{ "ref": "..." }` form is only needed for operator composition.

3. **3 comparison + 2 arithmetic = closed and sufficient**: The vocabulary covers all game-logic needs (health ≤ 0, distance > threshold, score += 1, position + input). Boolean logic (`and`/`or`/`not`) is handled by `if` nesting and guard composition in state machines, not expression operators.

4. **Type validation catches agent errors early**: An expression `{ "add": ["self.name", 1] }` (string + int) produces a `TypeMismatch` error at evaluation time with a clear message, not a runtime panic.

## Godot Mapping

| Godot | Craft Expression |
|-------|-----------------|
| `node->get("position")` (Variant access) | `{ "ref": "node.position" }` |
| `if (health <= 0)` (C++/GDScript) | `{ "cond": { "lt": ["self.health", 1] } }` |
| `position += velocity * delta` | `{ "kind": "move", "key": "position", "by": "self.velocity" }` |
| `emit_signal("hit", damage * multiplier)` | `{ "emit", "signal": "hit", "args": { "damage": { "add": ["self.damage", "self.multiplier"] } } }` |
| No equivalent — direct code | `engine.getSchema().expressions.operators` → AI discovers the vocabulary |
