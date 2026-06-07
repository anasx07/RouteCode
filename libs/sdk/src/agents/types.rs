use crate::core::{Message, ToolCall};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ConfirmationResponse {
    AllowOnce,
    AllowSession,
    AllowWorkspace,
    Deny,
    Feedback(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    Text {
        content: String,
    },
    Thought {
        content: String,
    },
    ToolCall {
        tool_call: ToolCall,
    },
    ToolResult {
        tool_call_id: String,
        name: String,
        content: String,
    },
    Usage {
        usage: Usage,
    },
    Error {
        content: String,
    },
    FinalHistory {
        history: Vec<Message>,
    },
    Models {
        models: Vec<crate::core::DynamicModelInfo>,
    },
    ModelsDone,
    RequestConfirmation {
        message: String,
        target: String,
        #[serde(skip)]
        tx: Option<
            Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>>,
        >,
    },
    UpdateAvailable {
        version: String,
        changelog: String,
        published_at: String,
    },
    Status {
        content: String,
    },
    SessionStats {
        total_tokens: u32,
        total_cost: f64,
        qir_attempts: u32,
    },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
