use crate::core::ToolCall;
use serde::{Deserialize, Serialize};

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
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
