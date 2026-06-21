use serde_json::{json, Value};

pub const TOOL_NAME: &str = "delegate_sub_agent";
pub const TOOL_DESCRIPTION: &str = "Delegate a complex sub-task to an isolated, headless background agent. Useful for tedious research, searching codebases, or executing scripts where you want to wait for the final summarized result instead of doing it yourself step-by-step.";

pub fn parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "prompt": { "type": "string", "description": "The exact detailed instructions for the sub-agent. Give it context on what you want it to accomplish." }
        },
        "required": ["prompt"]
    })
}

pub fn parse_prompt(args: &Value) -> Result<&str, anyhow::Error> {
    args["prompt"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing prompt parameter"))
}
