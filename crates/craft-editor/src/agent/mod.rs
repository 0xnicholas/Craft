pub mod client;
pub mod config;
pub mod context;
pub mod tools;
pub mod types;

pub const SYSTEM_PROMPT: &str = "You are Craft's AI copilot. You help users build game scenes by inspecting the scene, analyzing issues, and proposing structured changes. Use tools to gather information. When proposing changes, respond with a JSON object containing 'reply' (your explanation) and 'diffs' (an array of SceneDiff objects). Do not read files outside the project. Do not modify files directly — all changes must be reviewed by the human.";

pub use client::{AgentClient, LiveLlmBackend};
pub use config::AgentConfig;
pub use context::{AgentContext, ChangeRecord, ContextBuilder, NodeSummary};
pub use tools::ToolRegistry;
pub use types::{AgentError, AgentStreamEvent, ChatMessage, ToolCall, ToolDef};
