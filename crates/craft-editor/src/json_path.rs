use serde_json::Value;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct JsonPathLsp {
    schema_root: Value,
    #[allow(dead_code)]
    schema_root_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSeg {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CursorCtx {
    pub in_object_key: bool,
    pub in_object_value: bool,
    pub partial_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Property,
    Value,
    Snippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub insert_text: String,
    pub insert_range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub line: u32,
    pub col: u32,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionPopup {
    pub items: Vec<Completion>,
    pub selected: usize,
}

impl JsonPathLsp {
    pub fn new(schema_root: Value) -> Self {
        Self {
            schema_root,
            schema_root_path: None,
        }
    }

    pub fn path_at(&self, buffer: &str, cursor_byte: usize) -> Vec<PathSeg> {
        let bytes = buffer.as_bytes();
        let cursor = cursor_byte.min(bytes.len());
        let mut path: Vec<PathSeg> = Vec::new();
        let mut stack: Vec<ContainerState> = Vec::new();
        let mut i = 0;
        while i < cursor {
            let b = bytes[i];
            match b {
                b' ' | b'\t' | b'\n' | b'\r' => {
                    i += 1;
                }
                b'{' => {
                    stack.push(ContainerState::Object {
                        expect_key: true,
                        path_len_on_entry: path.len(),
                    });
                    i += 1;
                }
                b'[' => {
                    stack.push(ContainerState::Array {
                        next_index: 0,
                        path_len_on_entry: path.len(),
                    });
                    i += 1;
                }
                b'}' | b']' => {
                    if !stack.is_empty() {
                        stack.pop();
                    }
                    i += 1;
                }
                b',' => {
                    if let Some(ContainerState::Object { expect_key, .. }) = stack.last_mut() {
                        *expect_key = true;
                    } else if let Some(ContainerState::Array {
                        next_index,
                        path_len_on_entry,
                    }) = stack.last_mut()
                    {
                        *next_index += 1;
                        path.truncate(*path_len_on_entry);
                        path.push(PathSeg::Index(*next_index));
                    }
                    i += 1;
                }
                b'"' => {
                    let (consumed, value) = parse_string(bytes, i);
                    if let Some(top) = stack.last_mut() {
                        match top {
                            ContainerState::Object {
                                expect_key,
                                path_len_on_entry,
                            } => {
                                if *expect_key {
                                    path.truncate(*path_len_on_entry);
                                    path.push(PathSeg::Key(value.clone()));
                                    *expect_key = false;
                                    if let Some(peek) = bytes.get(i + consumed) {
                                        if *peek == b':' {
                                            i += consumed + 1;
                                            continue;
                                        }
                                    }
                                }
                            }
                            ContainerState::Array { .. } => {}
                        }
                    }
                    i += consumed;
                }
                b':' => {
                    i += 1;
                }
                _ => {
                    i = skip_literal(bytes, i);
                }
            }
        }

        if let Some(ContainerState::Array {
            next_index,
            path_len_on_entry,
        }) = stack.last()
        {
            path.truncate(*path_len_on_entry);
            path.push(PathSeg::Index(*next_index));
        }

        path
    }

    pub fn complete(&self, path: &[PathSeg], ctx: &CursorCtx) -> Vec<Completion> {
        if ctx.in_object_key {
            let schema = self
                .lookup(path)
                .filter(|s| s.get("properties").is_some())
                .or_else(|| self.lookup_parent(path));
            let Some(schema) = schema else {
                return Vec::new();
            };
            let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
                return Vec::new();
            };
            props
                .keys()
                .map(|k| Completion {
                    label: k.clone(),
                    kind: CompletionKind::Property,
                    detail: schema_detail(props.get(k).unwrap_or(&Value::Null)),
                    insert_text: format!("\"{k}\": "),
                    insert_range: 0..ctx.partial_token.len(),
                })
                .collect()
        } else if ctx.in_object_value {
            let Some(schema) = self.lookup(path) else {
                return Vec::new();
            };
            if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
                return enum_values
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .map(|s| Completion {
                        label: s.clone(),
                        kind: CompletionKind::Value,
                        detail: None,
                        insert_text: s,
                        insert_range: 0..ctx.partial_token.len(),
                    })
                    .collect();
            }
            let snippet = match schema.get("type").and_then(|t| t.as_str()) {
                Some("string") => "\"\"",
                Some("integer") | Some("number") => "0",
                Some("boolean") => "false",
                Some("object") => "{}",
                Some("array") => "[]",
                _ => return Vec::new(),
            };
            vec![Completion {
                label: snippet.to_string(),
                kind: CompletionKind::Snippet,
                detail: schema_detail(schema),
                insert_text: snippet.to_string(),
                insert_range: 0..ctx.partial_token.len(),
            }]
        } else {
            Vec::new()
        }
    }

    pub fn validate(&self, buffer: &str) -> Vec<SchemaError> {
        let parsed: Value = match serde_json::from_str(buffer) {
            Ok(v) => v,
            Err(e) => {
                let (line, col) = byte_offset_to_line_col(buffer, e.column());
                return vec![SchemaError {
                    line,
                    col,
                    message: format!("invalid JSON: {e}"),
                    severity: Severity::Error,
                }];
            }
        };

        let mut errors = Vec::new();
        validate_value(&parsed, &self.schema_root, &[], buffer, &mut errors);
        errors
    }

    fn lookup(&self, path: &[PathSeg]) -> Option<&Value> {
        let mut current = &self.schema_root;
        for seg in path {
            current = self.step(current, seg)?;
        }
        Some(current)
    }

    fn lookup_parent(&self, path: &[PathSeg]) -> Option<&Value> {
        if path.is_empty() {
            return Some(&self.schema_root);
        }
        let parent_path = &path[..path.len() - 1];
        self.lookup(parent_path)
    }

    fn step<'a>(&self, current: &'a Value, seg: &PathSeg) -> Option<&'a Value> {
        match seg {
            PathSeg::Key(k) => current
                .get("properties")
                .and_then(|p| p.get(k))
                .or_else(|| current.get(k)),
            PathSeg::Index(i) => current.get("items").or_else(|| current.as_array()?.get(*i)),
        }
    }
}

#[derive(Debug, Clone)]
enum ContainerState {
    Object {
        expect_key: bool,
        path_len_on_entry: usize,
    },
    Array {
        next_index: usize,
        path_len_on_entry: usize,
    },
}

fn parse_string(bytes: &[u8], start: usize) -> (usize, String) {
    let mut i = start + 1;
    let mut value = String::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if i + 1 < bytes.len() => {
                let next = bytes[i + 1];
                match next {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'n' => value.push('\n'),
                    b't' => value.push('\t'),
                    _ => value.push(next as char),
                }
                i += 2;
            }
            b'"' => {
                let consumed = i - start + 1;
                return (consumed, value);
            }
            other => {
                value.push(other as char);
                i += 1;
            }
        }
    }
    let consumed = bytes.len() - start;
    (consumed, value)
}

fn skip_literal(bytes: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r' => break,
            _ => i += 1,
        }
    }
    i
}

fn schema_detail(schema: &Value) -> Option<String> {
    schema
        .get("type")
        .and_then(|t| t.as_str())
        .map(|t| t.to_string())
}

fn validate_value(
    value: &Value,
    schema: &Value,
    path: &[PathSeg],
    buffer: &str,
    errors: &mut Vec<SchemaError>,
) {
    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
        if let Some(obj) = value.as_object() {
            for req in required {
                if let Some(key) = req.as_str() {
                    if !obj.contains_key(key) {
                        let offset = buffer_offset_for_path(buffer, path);
                        let (line, col) = byte_offset_to_line_col(buffer, offset);
                        errors.push(SchemaError {
                            line,
                            col,
                            message: format!("missing required property: {key}"),
                            severity: Severity::Error,
                        });
                    }
                }
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                let prop_schema = properties.get(key);
                if prop_schema.is_none() {
                    let offset = buffer_offset_for_path(buffer, path) + key.len() + 4;
                    let (line, col) = byte_offset_to_line_col(buffer, offset);
                    errors.push(SchemaError {
                        line,
                        col,
                        message: format!("unknown property: {key}"),
                        severity: Severity::Warning,
                    });
                } else if let Some(ps) = prop_schema {
                    if let Some(expected) = ps.get("type").and_then(|t| t.as_str()) {
                        let actual_ok = match expected {
                            "string" => val.is_string(),
                            "integer" => val.is_i64() || val.is_u64(),
                            "number" => val.is_number(),
                            "boolean" => val.is_boolean(),
                            "object" => val.is_object(),
                            "array" => val.is_array(),
                            _ => true,
                        };
                        if !actual_ok {
                            let offset = buffer_offset_for_path(buffer, path) + key.len() + 4;
                            let (line, col) = byte_offset_to_line_col(buffer, offset);
                            errors.push(SchemaError {
                                line,
                                col,
                                message: format!("type mismatch: {key} expected {expected}"),
                                severity: Severity::Error,
                            });
                        }
                    }
                    if let Some(enum_values) = ps.get("enum").and_then(|e| e.as_array()) {
                        if let Some(s) = val.as_str() {
                            if !enum_values.iter().any(|v| v.as_str() == Some(s)) {
                                let offset = buffer_offset_for_path(buffer, path) + key.len() + 4;
                                let (line, col) = byte_offset_to_line_col(buffer, offset);
                                errors.push(SchemaError {
                                    line,
                                    col,
                                    message: format!("{key}: invalid enum value: {s}"),
                                    severity: Severity::Error,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn byte_offset_to_line_col(buffer: &str, offset: usize) -> (u32, u32) {
    let prefix = &buffer[..offset.min(buffer.len())];
    let line = prefix.bytes().filter(|b| *b == b'\n').count() as u32;
    let last_nl = prefix.rfind('\n').map(|p| p + 1).unwrap_or(0);
    let col = (offset.saturating_sub(last_nl)) as u32;
    (line, col)
}

fn buffer_offset_for_path(_buffer: &str, _path: &[PathSeg]) -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lsp_with_empty_object() -> JsonPathLsp {
        JsonPathLsp::new(json!({}))
    }

    #[test]
    fn path_at_root_returns_empty() {
        let lsp = lsp_with_empty_object();
        let path = lsp.path_at("{}", 1);
        assert!(path.is_empty());
    }

    #[test]
    fn path_at_object_key_tracks_key() {
        let lsp = lsp_with_empty_object();
        let buf = r#"{"foo"#;
        let path = lsp.path_at(buf, buf.len());
        assert_eq!(path, vec![PathSeg::Key("foo".to_string())]);
    }

    #[test]
    fn path_at_array_index() {
        let lsp = lsp_with_empty_object();
        let buf = r#"[1, 2, "#;
        let path = lsp.path_at(buf, buf.len());
        assert_eq!(path, vec![PathSeg::Index(2)]);
    }

    #[test]
    fn path_at_nested_key_value() {
        let lsp = lsp_with_empty_object();
        let buf = r#"{"a": {"b": "v"#;
        let path = lsp.path_at(buf, buf.len());
        assert_eq!(
            path,
            vec![PathSeg::Key("a".to_string()), PathSeg::Key("b".to_string()),]
        );
    }

    #[test]
    fn path_at_handles_incomplete_buffer() {
        let lsp = lsp_with_empty_object();
        let buf = r#"{"foo": "bar", "#;
        let path = lsp.path_at(buf, buf.len());
        assert_eq!(path, vec![PathSeg::Key("foo".to_string())]);
    }

    #[test]
    fn complete_at_object_key_returns_schema_properties() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "properties": {
                "kind": { "type": "string" },
                "params": { "type": "object" }
            }
        }));
        let path = vec![PathSeg::Key("kind".to_string())];
        let ctx = CursorCtx {
            in_object_key: true,
            in_object_value: false,
            partial_token: String::new(),
        };
        let completions = lsp.complete(&path, &ctx);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"kind"));
        assert!(labels.contains(&"params"));
        assert!(
            completions
                .iter()
                .all(|c| matches!(c.kind, CompletionKind::Property))
        );
    }

    #[test]
    fn complete_at_enum_returns_enum_values() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "properties": {
                "kind": { "enum": ["set_state", "emit", "destroy"] }
            }
        }));
        let path = vec![PathSeg::Key("kind".to_string())];
        let ctx = CursorCtx {
            in_object_key: false,
            in_object_value: true,
            partial_token: String::new(),
        };
        let completions = lsp.complete(&path, &ctx);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, vec!["set_state", "emit", "destroy"]);
    }

    #[test]
    fn complete_at_typed_value_returns_snippet() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "properties": {
                "state": { "type": "string" },
                "hp": { "type": "integer" }
            }
        }));
        let path = vec![PathSeg::Key("state".to_string())];
        let ctx = CursorCtx {
            in_object_key: false,
            in_object_value: true,
            partial_token: String::new(),
        };
        let completions = lsp.complete(&path, &ctx);
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].insert_text, "\"\"");
        assert!(matches!(completions[0].kind, CompletionKind::Snippet));
    }

    #[test]
    fn validate_flags_missing_required_property() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "required": ["kind"],
            "properties": { "kind": { "type": "string" } }
        }));
        let errors = lsp.validate("{}");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("kind") && matches!(e.severity, Severity::Error))
        );
    }

    #[test]
    fn validate_flags_type_mismatch() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "properties": { "hp": { "type": "integer" } }
        }));
        let errors = lsp.validate(r#"{"hp": "not a number"}"#);
        assert!(errors.iter().any(|e| e.message.contains("hp")));
    }

    #[test]
    fn validate_ignores_unknown_property_as_warning() {
        let lsp = JsonPathLsp::new(json!({
            "type": "object",
            "properties": { "kind": { "type": "string" } }
        }));
        let errors = lsp.validate(r#"{"kind": "set_state", "unknown_field": true}"#);
        assert!(errors.iter().any(
            |e| e.message.contains("unknown_field") && matches!(e.severity, Severity::Warning)
        ));
        assert!(
            errors
                .iter()
                .all(|e| !matches!(e.severity, Severity::Error) || e.message.contains("parse"))
        );
    }

    #[test]
    fn validate_returns_parse_error_on_invalid_json() {
        let lsp = JsonPathLsp::new(json!({"type": "object"}));
        let errors = lsp.validate("{not valid");
        assert!(!errors.is_empty());
        assert!(errors[0].message.to_lowercase().contains("json"));
    }
}
