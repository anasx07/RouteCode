use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::Value;

pub mod allowlist;
pub mod decision;
pub mod destructive;
pub mod exec;
pub mod mode;
pub mod permissions;
pub mod readonly;
pub mod schema;
pub mod validation;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        schema::TOOL_NAME
    }

    fn description(&self) -> &str {
        schema::TOOL_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        schema::parameters()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let command_str = validation::parse_command(&args)?;

        if !validation::is_valid(command_str) {
            return Ok(ToolResult::error("Command cannot be empty"));
        }

        let output = exec::run(command_str).await?;
        Ok(exec::to_result(output))
    }
}

impl BashTool {
    /// Legacy permission check. Returns the old `Permission` enum.
    /// Kept for backward compatibility with existing callers.
    pub fn check_permission(
        &self,
        command: &str,
        config: &crate::core::Config,
    ) -> permissions::Permission {
        permissions::check(command, config)
    }

    /// Evaluate a command against the user's config and return a `Decision`
    /// describing whether the command is allowed, should prompt, or is
    /// denied. This is the new permission flow used by the orchestrator.
    pub fn evaluate(
        command: &str,
        config: &crate::core::Config,
    ) -> decision::Decision {
        mode::evaluate(command, config)
    }
}
