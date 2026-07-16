#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("scene parse error: {0}")]
    SceneParse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{message}")]
    Other { message: String },
}
