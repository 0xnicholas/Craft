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
                    stack.push(ContainerState::Object { expect_key: true });
                    i += 1;
                }
                b'[' => {
                    stack.push(ContainerState::Array { next_index: 0 });
                    i += 1;
                }
                b'}' | b']' => {
                    if !stack.is_empty() {
                        stack.pop();
                    }
                    i += 1;
                }
                b',' => {
                    if let Some(ContainerState::Object { expect_key }) = stack.last_mut() {
                        *expect_key = true;
                    } else if let Some(ContainerState::Array { next_index }) = stack.last_mut() {
                        *next_index += 1;
                    }
                    i += 1;
                }
                b'"' => {
                    let (consumed, value) = parse_string(bytes, i);
                    if let Some(top) = stack.last_mut() {
                        match top {
                            ContainerState::Object { expect_key } => {
                                if *expect_key {
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

        if let Some(ContainerState::Array { next_index }) = stack.last() {
            path.push(PathSeg::Index(*next_index));
        }

        path
    }

    #[allow(dead_code)]
    pub fn complete(&self, _path: &[PathSeg], _ctx: &CursorCtx) -> Vec<Completion> {
        Vec::new()
    }

    #[allow(dead_code)]
    pub fn validate(&self, _buffer: &str) -> Vec<SchemaError> {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
enum ContainerState {
    Object { expect_key: bool },
    Array { next_index: usize },
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
}
