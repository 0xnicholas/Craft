#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("scene parse error: {0}")]
    SceneParse(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("{message}")]
    Other { message: String },
}

impl Clone for EditorError {
    fn clone(&self) -> Self {
        match self {
            Self::SceneParse(s) => Self::SceneParse(s.clone()),
            Self::Io(e) => Self::Other {
                message: e.to_string(),
            },
            Self::Json(e) => Self::Other {
                message: e.to_string(),
            },
            Self::Toml(e) => Self::Other {
                message: e.to_string(),
            },
            Self::Other { message } => Self::Other {
                message: message.clone(),
            },
        }
    }
}
