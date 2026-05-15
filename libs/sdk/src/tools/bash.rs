use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command as TokioCommand;

pub struct BashTool;

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }
    fn description(&self) -> &str {
        "Execute a terminal command"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The command to execute" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let command_str = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing command"))?;

        let output = if cfg!(target_os = "windows") {
            TokioCommand::new("cmd")
                .args(["/C", command_str])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
                .wait_with_output()
                .await?
        } else {
            TokioCommand::new("sh")
                .args(["-c", command_str])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?
                .wait_with_output()
                .await?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            let mut result = stdout;
            if !stderr.is_empty() {
                result = format!("Stdout:\n{}\nStderr:\n{}", result, stderr);
            }
            Ok(ToolResult::success(result))
        } else {
            Ok(ToolResult::error(format!(
                "Command failed with exit code: {}\nStdout: {}\nStderr: {}",
                output.status.code().unwrap_or(-1),
                stdout,
                stderr
            )))
        }
    }
}
