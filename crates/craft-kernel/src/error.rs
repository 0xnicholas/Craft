use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
#[serde(tag = "category", content = "data")]
pub enum EngineError {
    #[error("{}", .0.message)]
    #[serde(rename = "parse")]
    Parse(ParseError),

    #[error("validation failed with {} error(s)", .errors.len())]
    #[serde(rename = "validation")]
    Validation {
        file: String,
        errors: Vec<ValidationError>,
    },

    #[error("io error in {}: {}", .0.file, .0.message)]
    #[serde(rename = "io")]
    Io(IoError),

    #[error("internal error: {0}")]
    #[serde(rename = "internal")]
    Internal(String),
}

impl EngineError {
    pub fn file(&self) -> &str {
        match self {
            Self::Parse(e) => &e.file,
            Self::Validation { file, .. } => file,
            Self::Io(e) => &e.file,
            Self::Internal(_) => "<internal>",
        }
    }
}

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Error, Serialize, Deserialize, Clone)]
#[error("io {kind:?} on {file}: {message}")]
pub struct IoError {
    pub file: String,
    pub kind: IoErrorKind,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IoErrorKind {
    Read,
    Write,
    NotFound,
    PermissionDenied,
    Other,
}

impl IoError {
    pub fn read(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            kind: IoErrorKind::Read,
            message: message.into(),
        }
    }

    pub fn not_found(file: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            kind: IoErrorKind::NotFound,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParseError {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub message: String,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidationError {
    pub file: String,
    pub json_path: String,
    pub message: String,
    pub expected_type: String,
    pub actual_value: Option<serde_json::Value>,
    pub suggestion: Option<String>,
    pub auto_fixable: AutoFix,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutoFix {
    Safe,
    Suggested,
    NeedsReview,
}

#[derive(Debug, Default)]
pub struct ErrorCollector {
    file: String,
    errors: Vec<ValidationError>,
}

impl ErrorCollector {
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            errors: Vec::new(),
        }
    }

    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn into_result(self) -> EngineResult<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(EngineError::Validation {
                file: self.file,
                errors: self.errors,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_collector_collects_multiple() {
        let mut c = ErrorCollector::new("scene.json");
        c.push(ValidationError {
            file: "scene.json".to_string(),
            json_path: "$.nodes[0].components.health".to_string(),
            message: "expected integer".to_string(),
            expected_type: "integer".to_string(),
            actual_value: Some(serde_json::json!("fast")),
            suggestion: Some("Replace with a number like 100".to_string()),
            auto_fixable: AutoFix::Suggested,
        });
        c.push(ValidationError {
            file: "scene.json".to_string(),
            json_path: "$.nodes[1].type".to_string(),
            message: "unknown node type".to_string(),
            expected_type: "known node type".to_string(),
            actual_value: Some(serde_json::json!("Monstr")),
            suggestion: Some("Did you mean 'Monster'?".to_string()),
            auto_fixable: AutoFix::Suggested,
        });
        assert_eq!(c.len(), 2);
        assert!(!c.is_empty());

        let result = c.into_result();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, EngineError::Validation { .. }));
        if let EngineError::Validation { file, errors } = err {
            assert_eq!(file, "scene.json");
            assert_eq!(errors.len(), 2);
            assert_eq!(errors[0].json_path, "$.nodes[0].components.health");
            assert_eq!(
                errors[0].suggestion.as_deref(),
                Some("Replace with a number like 100")
            );
        }
    }

    #[test]
    fn empty_collector_returns_ok() {
        let c = ErrorCollector::new("scene.json");
        assert!(c.into_result().is_ok());
    }

    #[test]
    fn parse_error_serializes_with_category() {
        let err = EngineError::Parse(ParseError {
            file: "scene.json".to_string(),
            line: Some(42),
            column: Some(5),
            message: "unexpected token".to_string(),
            snippet: None,
        });
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["category"], "parse");
        assert_eq!(v["data"]["file"], "scene.json");
        assert_eq!(v["data"]["line"], 42);
    }

    #[test]
    fn validation_error_serializes_with_category() {
        let err = EngineError::Validation {
            file: "scene.json".to_string(),
            errors: vec![ValidationError {
                file: "scene.json".to_string(),
                json_path: "$.kind".to_string(),
                message: "expected \"scene\"".to_string(),
                expected_type: "string literal \"scene\"".to_string(),
                actual_value: Some(serde_json::json!("level")),
                suggestion: Some("Set \"kind\": \"scene\" at the top level".to_string()),
                auto_fixable: AutoFix::Safe,
            }],
        };
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["category"], "validation");
        assert_eq!(v["data"]["errors"][0]["json_path"], "$.kind");
        assert_eq!(v["data"]["errors"][0]["auto_fixable"], "safe");
    }

    #[test]
    fn io_error_serializes_with_file_and_kind() {
        let err = EngineError::Io(IoError::read("craft.toml", "permission denied"));
        assert_eq!(err.file(), "craft.toml");
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["category"], "io");
        assert_eq!(v["data"]["file"], "craft.toml");
        assert_eq!(v["data"]["kind"], "read");
    }
}
