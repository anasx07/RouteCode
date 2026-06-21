use serde_json::{json, Value};

pub fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "command": { "type": "string", "description": "The command to execute" }
        },
        "required": ["command"]
    })
}

pub const TOOL_NAME: &str = "bash";
pub const TOOL_DESCRIPTION: &str = "Execute a terminal command";
