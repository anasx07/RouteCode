use serde_json::{json, Value};

pub const ENTER_PLAN_MODE_TOOL_NAME: &str = "enter_plan_mode";
pub const EXIT_PLAN_MODE_TOOL_NAME: &str = "exit_plan_mode";

pub const ENTER_PLAN_MODE_DESCRIPTION: &str =
    "Enter plan mode for non-trivial implementation tasks. In plan mode \
     you can explore the codebase (read-only tools) and design an \
     implementation approach. Call `exit_plan_mode` when ready to present \
     your plan for approval.";

pub const EXIT_PLAN_MODE_DESCRIPTION: &str =
    "Present the current plan to the user for approval. The plan is read \
     from the persisted plan file. On approval, write tools are unlocked \
     for the rest of the session.";

pub fn enter_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })
}

pub fn exit_parameters() -> Value {
    json!({
        "type": "object",
        "properties": {
            "allowedPrompts": {
                "type": "array",
                "description": "Optional list of prompt-based permissions \
                                needed to implement the plan (e.g. \
                                'run tests', 'install dependencies').",
                "items": {
                    "type": "object",
                    "properties": {
                        "tool": { "type": "string", "enum": ["Bash"] },
                        "prompt": { "type": "string" }
                    },
                    "required": ["tool", "prompt"]
                }
            }
        },
        "additionalProperties": false
    })
}
