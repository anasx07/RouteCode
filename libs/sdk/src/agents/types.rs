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

/// User's response to a plan-approval prompt. Distinct from
/// `ConfirmationResponse` because the plan flow has its own semantics
/// (approving a plan unlocks write tools for the session, not just the
/// current tool call).
#[derive(Debug, Clone)]
pub enum PlanApprovalResponse {
    /// Approve this plan, do NOT unlock write tools. Equivalent to
    /// the user saying "go ahead" for a single step; if the AI then
    /// needs to do anything in plan mode, the next tool call will
    /// still hit the plan-mode filter. (Rarely used; the typical
    /// approve is `ApproveAndUnlock`.)
    ApproveOnce,
    /// Approve AND unlock write tools for the rest of the session.
    /// This is the common approve.
    ApproveAndUnlock,
    /// Reject; stay in plan mode. The AI should revise the plan.
    Deny,
    /// Reject with feedback the AI can use to revise the plan.
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
        /// Optional human-readable warning shown alongside the prompt
        /// (e.g. "may overwrite remote history"). Empty string means no
        /// warning.
        #[serde(default)]
        warning: String,
        #[serde(skip)]
        tx: Option<
            Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>>,
        >,
    },
    /// Sent by the orchestrator when the AI calls `exit_plan_mode`.
    /// The UI must present the plan to the user and respond with
    /// `PlanApprovalResponse`. Only emitted in plan mode.
    RequestPlanApproval {
        /// The plan markdown, loaded from disk.
        plan: String,
        /// Absolute path of the plan file (for the UI to show or
        /// open in an editor).
        plan_path: String,
        /// Optional semantic permissions the AI requested.
        allowed_prompts: Vec<AllowedPrompt>,
        #[serde(skip)]
        tx: Option<
            Arc<
                tokio::sync::Mutex<
                    Option<tokio::sync::oneshot::Sender<PlanApprovalResponse>>,
                >,
            >,
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
    /// Sent when the orchestrator starts running a hook for a given
    /// event. The UI uses this to show a brief "running PreToolUse
    /// hook" message in the spinner.
    HookProgress {
        event: String,
        name: String,
    },
    /// Sent when a hook (or aggregated set of hooks) finishes. The
    /// UI uses this to log blocked calls or context injections.
    HookResult {
        event: String,
        name: String,
        decision: Option<String>,
        reason: Option<String>,
        additional_context: Option<String>,
        system_message: Option<String>,
    },
    /// Sent by the orchestrator the first time it encounters a
    /// project whose `.routecode/settings.json` defines hooks. The
    /// UI must present the hooks the project wants to register
    /// and respond with the user's decision.
    RequestHookTrust {
        project_signature: String,
        project_path: String,
        /// List of (event, matcher, description) the project wants
        /// to register.
        hooks: Vec<crate::hooks::HookTrustEntry>,
        #[serde(skip)]
        tx: Option<
            Arc<
                tokio::sync::Mutex<
                    Option<tokio::sync::oneshot::Sender<HookTrustResponse>>,
                >,
            >,
        >,
    },
    CompactProgress {
        status: String,
    },
    CompactResult {
        pre_tokens: u32,
        post_tokens: u32,
    },
    ContextWarning {
        message: String,
    },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A semantic permission the AI requests in an `exit_plan_mode` call,
/// e.g. `{tool: "Bash", prompt: "run tests"}`. Currently informational
/// only; the permission engine doesn't yet match these to specific
/// commands. They are surfaced in the approval dialog for the user to
/// see and acknowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedPrompt {
    pub tool: String,
    pub prompt: String,
}

/// User's response to a `RequestHookTrust` prompt.
#[derive(Debug, Clone)]
pub enum HookTrustResponse {
    /// Trust this project. The project signature is added to
    /// `.routecode/trusted_hooks.json` and the hooks will run
    /// without re-prompting.
    Trust,
    /// Don't trust. The project hooks are silently dropped for this
    /// session. (Re-prompted next time the settings file changes.)
    Deny,
}

/// One entry in a `RequestHookTrust` chunk — a human-readable
/// summary of a single hook the project wants to register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookTrustEntry {
    pub event: String,
    pub matcher: String,
    pub description: String,
}
