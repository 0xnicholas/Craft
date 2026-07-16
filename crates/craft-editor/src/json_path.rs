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

    #[allow(dead_code)]
    pub fn path_at(&self, _buffer: &str, _cursor_byte: usize) -> Vec<PathSeg> {
        Vec::new()
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
