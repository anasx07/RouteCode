use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::Value;

use super::prompt::EXIT_PLAN_MODE_PROMPT;
use super::schema::{
    EXIT_PLAN_MODE_DESCRIPTION, EXIT_PLAN_MODE_TOOL_NAME, exit_parameters,
};

/// Presents the current plan (read from disk) to the user for approval.
/// On approval, the orchestrator unlocks write tools for the rest of
/// the session.
pub struct ExitPlanModeTool;

#[async_trait]
impl Tool for ExitPlanModeTool {
    fn name(&self) -> &str {
        EXIT_PLAN_MODE_TOOL_NAME
    }

    fn description(&self) -> &str {
        EXIT_PLAN_MODE_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        exit_parameters()
    }

    fn is_concurrency_safe(&self) -> bool {
        true
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult, anyhow::Error> {
        // The orchestrator intercepts this call before it would execute
        // (it needs the plan file content + UI channel). If execution
        // reaches here, it means the orchestrator didn't intercept (test
        // paths, etc.) — return a helpful synthetic result.
        Ok(ToolResult::success(
            "Plan presented to user for approval.",
        ))
    }
}

impl ExitPlanModeTool {
    /// Returns the prompt text for the system message describing when
    /// and how to use this tool.
    pub fn prompt() -> &'static str {
        EXIT_PLAN_MODE_PROMPT
    }

    /// Static description for use in schema construction outside of
    /// the trait (e.g. by the plan-mode filter).
    pub fn description_static() -> &'static str {
        EXIT_PLAN_MODE_DESCRIPTION
    }
}
