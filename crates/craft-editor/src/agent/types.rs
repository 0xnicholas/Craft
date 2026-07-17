use serde::{Deserialize, Serialize};
use std::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub def_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum AgentStreamEvent {
    Token(String),
    Done {
        full_text: String,
        tool_calls: Vec<ToolCall>,
    },
    Error(String),
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    #[error("API key not configured")]
    NoApiKey,
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Request already in flight")]
    Busy,
}

pub trait LlmBackend: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    fn chat(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        stream: bool,
        api_base: &str,
        api_key: &str,
        model: &str,
        event_tx: mpsc::Sender<AgentStreamEvent>,
    );
}
