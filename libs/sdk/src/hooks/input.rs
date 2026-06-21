use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::events::HookEvent;

/// Common fields included in every hook input. Mirrors
/// `BaseHookInputSchema` in Claude Code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    pub session_id: String,
    pub transcript_path: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub permission_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub agent_type: Option<String>,
}

/// Input to a `PreToolUse` hook. The hook can inspect `tool_input`,
/// optionally return `permissionDecision: "block"` to deny the call,
/// or `updatedInput` to modify the arguments before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: HookEvent,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_use_id: String,
}

/// Input to a `PostToolUse` hook. Fires after a tool returns Ok.
/// The hook can return `additionalContext` to inject text into the
/// model context, or `updatedMCPToolOutput` to override MCP tool
/// results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: HookEvent,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_response: Value,
    pub tool_use_id: String,
}

/// Input to a `PostToolUseFailure` hook. Fires when a tool returns
/// Err or is denied by the permission engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailureInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: HookEvent,
    pub tool_name: String,
    pub tool_input: Value,
    pub tool_use_id: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub is_interrupt: Option<bool>,
}

/// Input to a `Stop` hook. Fires when the agent finishes a turn
/// successfully. `stop_hook_active` is true when this hook is itself
/// the reason the agent stopped (so a hook can avoid infinite loops
/// by checking the flag).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: HookEvent,
    pub stop_hook_active: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_assistant_message: Option<String>,
}

/// Input to a `StopFailure` hook. Fires when the agent stops due
/// to an error (provider error, tool loop, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopFailureInput {
    #[serde(flatten)]
    pub base: BaseHookInput,
    pub hook_event_name: HookEvent,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error_details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_assistant_message: Option<String>,
}

/// Tagged union of all hook inputs. The orchestrator dispatches
/// based on `hook_event_name`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name", rename_all = "snake_case")]
pub enum HookInput {
    PreToolUse(PreToolUseInput),
    PostToolUse(PostToolUseInput),
    PostToolUseFailure(PostToolUseFailureInput),
    Stop(StopInput),
    StopFailure(StopFailureInput),
}

impl HookInput {
    pub fn event(&self) -> HookEvent {
        match self {
            HookInput::PreToolUse(i) => i.hook_event_name,
            HookInput::PostToolUse(i) => i.hook_event_name,
            HookInput::PostToolUseFailure(i) => i.hook_event_name,
            HookInput::Stop(i) => i.hook_event_name,
            HookInput::StopFailure(i) => i.hook_event_name,
        }
    }

    pub fn base(&self) -> &BaseHookInput {
        match self {
            HookInput::PreToolUse(i) => &i.base,
            HookInput::PostToolUse(i) => &i.base,
            HookInput::PostToolUseFailure(i) => &i.base,
            HookInput::Stop(i) => &i.base,
            HookInput::StopFailure(i) => &i.base,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_tool_use_input_serializes_with_event_tag() {
        let input = HookInput::PreToolUse(PreToolUseInput {
            base: BaseHookInput {
                session_id: "s1".into(),
                transcript_path: "/tmp/t.jsonl".into(),
                cwd: "/cwd".into(),
                permission_mode: None,
                agent_id: None,
                agent_type: None,
            },
            hook_event_name: HookEvent::PreToolUse,
            tool_name: "bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
            tool_use_id: "call_1".into(),
        });
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["hook_event_name"], "pre_tool_use");
        assert_eq!(json["tool_name"], "bash");
        assert_eq!(json["tool_input"]["command"], "ls");
    }
}
