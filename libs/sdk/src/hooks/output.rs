use serde::{Deserialize, Serialize};
use serde_json::Value;

// We intentionally keep the field names camelCase to match the
// JSON wire format used by Claude Code's hook protocol, so users
// can copy hook settings between tools.

/// What a hook decided for a PreToolUse event. `Block` denies the
/// call; `Approve` is informational (the call would have been
/// allowed anyway) and may suppress the user's confirmation prompt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreToolUseDecision {
    #[default]
    Approve,
    Block,
}

/// Per-event structured output. Currently only `PreToolUse` and
/// `PostToolUse` have meaningful fields, but the variant set matches
/// Claude Code's `hookSpecificOutput` union for forward-compat.
#[allow(non_snake_case)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hookEventName", rename_all = "PascalCase")]
pub enum HookSpecificOutput {
    PreToolUse {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        permissionDecision: Option<PreToolUseDecision>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        permissionDecisionReason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        updatedInput: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        additionalContext: Option<String>,
    },
    PostToolUse {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        additionalContext: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        updatedMCPToolOutput: Option<Value>,
    },
    PostToolUseFailure {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        additionalContext: Option<String>,
    },
    Stop {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        additionalContext: Option<String>,
    },
    StopFailure {
        #[serde(skip_serializing_if = "Option::is_none", default)]
        additionalContext: Option<String>,
    },
}

/// The result of running a single hook. Aggregated across all hooks
/// for an event by the `aggregate` module.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOutput {
    /// Whether the agent should continue after this hook. Default
    /// true. Set to false to stop the agent loop.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub continue_: Option<bool>,
    /// Hide the hook's stdout from the user transcript.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub suppress_output: Option<bool>,
    /// Shown when `continue_` is false.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub stop_reason: Option<String>,
    /// Per-event decision (approve/block).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub decision: Option<PreToolUseDecision>,
    /// Human-readable explanation of the decision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    /// Warning message shown to the user in the spinner.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub system_message: Option<String>,
    /// Per-event structured output.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hook_specific_output: Option<HookSpecificOutput>,
    /// Free-form additional context to inject into the model context.
    /// For PostToolUse this is the canonical way to add info; the
    /// runner also accepts it from the `additionalContext` field of
    /// any `hookSpecificOutput`. Both `additionalContext` (camelCase,
    /// matching the JSON wire format used by Claude Code) and
    /// `additional_context` are accepted on deserialize.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        alias = "additionalContext"
    )]
    pub additional_context: Option<String>,
}

impl HookOutput {
    /// Empty/approve result: hook ran and has no opinion.
    pub fn ok() -> Self {
        Self {
            continue_: Some(true),
            ..Default::default()
        }
    }

    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            continue_: Some(false),
            decision: Some(PreToolUseDecision::Block),
            reason: Some(reason.into()),
            ..Default::default()
        }
    }

    pub fn additional_context(ctx: impl Into<String>) -> Self {
        Self {
            additional_context: Some(ctx.into()),
            ..Self::ok()
        }
    }
}

/// Internal outcome of running a single hook. Used by the
/// aggregator; not part of the public hook protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookOutcome {
    /// Hook succeeded; output is present.
    Success,
    /// Hook exited with code 2 (blocking error). For PreToolUse this
    /// means deny; for other events it short-circuits the run.
    Blocking,
    /// Hook errored (non-zero exit other than 2, timeout, JSON parse
    /// failure). The agent continues.
    NonBlockingError,
    /// Hook was cancelled (abort signal).
    Cancelled,
    /// Hook was skipped (e.g. no matching matchers, disabled by env).
    Skipped,
}
