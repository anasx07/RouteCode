use super::path;
use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "Read the content of a file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The path to the file" }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let raw_path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let resolved = path::normalize(raw_path);
        match path::is_within_workspace(&resolved) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Access denied: Path '{}' is outside the workspace boundary",
                    resolved.display()
                )))
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to verify path '{}': {}",
                    resolved.display(),
                    e
                )))
            }
        }
        match fs::read_to_string(&resolved) {
            Ok(content) => Ok(ToolResult::success(content)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to read file '{}': {}",
                resolved.display(),
                e
            ))),
        }
    }
}
