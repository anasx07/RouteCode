use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::Value;

use super::prompt::ENTER_PLAN_MODE_PROMPT;
use super::schema::{
    ENTER_PLAN_MODE_DESCRIPTION, ENTER_PLAN_MODE_TOOL_NAME, enter_parameters,
};

/// Switches the orchestrator into plan mode.
///
/// In plan mode, write tools are filtered out of the schema and bash
/// is constrained to read-only commands. The user reviews the plan
/// before any writes happen.
pub struct EnterPlanModeTool;

#[async_trait]
impl Tool for EnterPlanModeTool {
    fn name(&self) -> &str {
        ENTER_PLAN_MODE_TOOL_NAME
    }

    fn description(&self) -> &str {
        ENTER_PLAN_MODE_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        enter_parameters()
    }

    async fn execute(&self, _args: Value) -> Result<ToolResult, anyhow::Error> {
        // The actual state mutation happens in the orchestrator when it
        // processes the tool call (it needs access to the orchestrator
        // and config). This tool returns a synthetic result so the model
        // sees a clean response.
        Ok(ToolResult::success(
            "Entered plan mode. You can now use read-only tools to explore \
             the codebase. Call `exit_plan_mode` when ready to present your \
             plan for approval.",
        ))
    }
}

impl EnterPlanModeTool {
    /// Returns the prompt text for the system message describing when
    /// and how to use this tool.
    pub fn prompt() -> &'static str {
        ENTER_PLAN_MODE_PROMPT
    }

    /// Static description for use in schema construction outside of
    /// the trait (e.g. by the plan-mode filter).
    pub fn description_static() -> &'static str {
        ENTER_PLAN_MODE_DESCRIPTION
    }
}
