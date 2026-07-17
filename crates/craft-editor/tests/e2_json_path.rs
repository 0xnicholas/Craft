use craft_editor::json_path::{CompletionKind, CursorCtx, JsonPathLsp, PathSeg, Severity};
use serde_json::json;

#[test]
fn end_to_end_path_and_complete() {
    let schema = json!({
        "type": "object",
        "required": ["kind"],
        "properties": {
            "kind": { "enum": ["set_state", "emit", "destroy"] },
            "params": {
                "type": "object",
                "properties": {
                    "state": { "type": "string" }
                }
            }
        }
    });
    let lsp = JsonPathLsp::new(schema);

    let buf = r#"{ "kind": "set_state", "params": { "#;
    let path = lsp.path_at(buf, buf.len());
    assert_eq!(path, vec![PathSeg::Key("params".to_string())]);

    let ctx = CursorCtx {
        in_object_key: true,
        in_object_value: false,
        partial_token: String::new(),
    };
    let completions = lsp.complete(&path, &ctx);
    assert!(completions.iter().any(|c| c.label == "state"));
}

#[test]
fn validate_missing_required_and_unknown_field() {
    let schema = json!({
        "type": "object",
        "required": ["kind"],
        "properties": {
            "kind": { "enum": ["set_state", "emit", "destroy"] },
            "params": {
                "type": "object",
                "properties": {
                    "state": { "type": "string" }
                }
            }
        }
    });
    let lsp = JsonPathLsp::new(schema);

    let errors = lsp.validate(r#"{"params": {}, "extra": 1}"#);
    assert!(
        errors
            .iter()
            .any(|e| matches!(e.severity, Severity::Error) && e.message.contains("kind")),
        "should flag missing required 'kind': {errors:?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| matches!(e.severity, Severity::Warning) && e.message.contains("extra")),
        "should warn about unknown 'extra': {errors:?}"
    );
}

#[test]
fn completion_kinds_are_consistent() {
    let lsp = JsonPathLsp::new(json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    }));
    let ctx_key = CursorCtx {
        in_object_key: true,
        ..Default::default()
    };
    let ctx_value = CursorCtx {
        in_object_value: true,
        ..Default::default()
    };
    let path = vec![PathSeg::Key("name".to_string())];

    let key_completions = lsp.complete(&path, &ctx_key);
    assert!(matches!(key_completions[0].kind, CompletionKind::Property));

    let value_completions = lsp.complete(&path, &ctx_value);
    assert!(matches!(value_completions[0].kind, CompletionKind::Snippet));
}
