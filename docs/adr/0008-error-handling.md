# ADR 0008: Error Handling — Structured, Actionable, Multi-Error

**Date**: 2026-07-13
**Status**: Accepted
**Supersedes**: Godot's `ERR_FAIL_COND_V` macro + `print_error()` string-based error system

## Context

PRD requirement: "Errors are first-class structured data, with location, context, and actionable suggestions."

Godot's error system uses C macros (`ERR_FAIL_COND_V`, `ERR_FAIL_COND`, `ERR_PRINT`) that print to stderr and return/abort. Errors are strings, not structured data. An agent parsing Godot errors must regex through log output to find file paths, line numbers, and error messages.

Craft's agent is an LLM. Errors must be machine-readable JSON with enough context for the agent to auto-correct the scene file.

## Decision

**Structured error enum hierarchy with `file`/`json_path`/`suggestion` fields. Validation collects all errors before returning (bulk error, not first-error-abort).**

```rust
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "category")]
pub enum EngineError {
    #[serde(rename = "parse")]
    Parse(ParseError),
    #[serde(rename = "validation")]
    Validation(Vec<ValidationError>),
    #[serde(rename = "runtime")]
    Runtime(RuntimeError),
    #[serde(rename = "replay")]
    Replay(ReplayError),
}

pub struct ParseError {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub snippet: Option<String>,          // 3 lines around the error
}

pub struct ValidationError {
    pub file: String,
    pub json_path: String,               // "$.nodes.enemy.components.health"
    pub message: String,
    pub expected_type: String,
    pub actual_value: Option<Value>,
    pub suggestion: Option<String>,       // accessible fix, e.g. "Replace with a number like 100"
    pub auto_fixable: AutoFix,            // whether the agent can safely auto-apply the fix
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum AutoFix {
    /// Safe to auto-fix: the fix is unambiguous and reversible.
    /// Example: missing required field with a default value — fill in the default.
    Safe,

    /// Fix has low ambiguity but human review recommended.
    /// Example: type mismatch with a clear coercion (string "5" → integer 5).
    Suggested,

    /// Fix requires human judgment — multiple valid interpretations.
    /// Example: unknown component key "posiiton" — did they mean "position"?
    NeedsReview,
}

pub struct RuntimeError {
    pub tick: u32,
    pub node: Option<String>,
    pub behavior: Option<String>,
    pub action: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

pub struct ReplayError {
    pub tick: u32,
    pub expected_hash: u64,
    pub actual_hash: u64,
    pub first_divergent_component: Option<ComponentDiff>,
}
```

### Error Collector (for validation)

```rust
pub struct ErrorCollector {
    errors: Vec<ValidationError>,
    file: String,
}

impl ErrorCollector {
    pub fn new(file: &str) -> Self;
    pub fn add(&mut self, path: &str, expected: &str, actual: impl Debug, suggestion: Option<&str>);
    pub fn into_result(self) -> EngineResult<()>;  // Ok if no errors, Err(Validation) otherwise
}
```

## Rationale

1. **Bulk error collection saves agent iterations**: If a scene has 5 validation errors, the agent sees all 5 in one response and can fix them all. Godot's first-error-abort pattern would require 5 edit-reload cycles.

2. **`json_path` pinpoints the exact location**: The agent can find `$.nodes.enemy.components.health` directly in the JSON and replace it. No code navigation needed.

3. **`suggestion` is a template for the fix**: "Replace string with a numeric value, e.g. 5.0" is enough context for an LLM to generate the correct edit.

4. **Structured category discriminator**: `EngineError` uses `#[serde(tag = "category")]` so the JSON has `"category": "validation"` — the agent's SDK can pattern-match on the category for control flow.

5. **Lint is separate from errors**: `engine.lint()` produces `LintWarning` values (warnings, not errors). It checks for signal-with-no-subscribers, unreachable states, unused components. These don't block loading but inform the agent.

## Godot Mapping

| Godot | Craft |
|-------|-------|
| `ERR_FAIL_COND_V(msg, ret)` → stderr string | `EngineError::Runtime { tick, node, suggestion }` → JSON |
| `ERR_PRINT("bad thing")` | `ValidationError { json_path, expected_type, actual_value }` |
| First error aborts | All errors collected, then returned |
| No locational metadata | `file` + `json_path` + `line`/`column` |
| Human parses log output | Agent parses typed JSON response |
| No lint system | `engine.lint()` checks signal wiring, state reachability, etc. |

## Error Flow Example

```
Agent writes scene.json with 3 mistakes
     ↓
engine.start("res://scene.json")
     ↓
returns EngineError::Validation([
  { json_path: "$.root.components.bad_key", expected_type: "─", actual: {...}, suggestion: "Remove unknown key 'bad_key'" },
  { json_path: "$.nodes[2].type", expected_type: "known node type", actual: "\"Monstr\"", suggestion: "Did you mean 'Monster'?" },
  { json_path: "$.nodes[3].behaviors[0].states.attacking.on.tick", expected_type: "string (state name)", actual: "null", suggestion: "Specify a target state, e.g. \"idle\"" },
])
     ↓
Agent reads errors, makes 3 edits, calls engine.start() again → success
```
