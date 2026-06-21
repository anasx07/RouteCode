use super::{diff, path};
use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The path to the file" },
                "content": { "type": "string", "description": "The content to write" }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let raw_path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let content = args["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing content"))?;

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
        let old_content = fs::read_to_string(&resolved).unwrap_or_default();
        let file_diff = diff::generate(&old_content, content);

        if let Err(e) = path::ensure_parent_dir(&resolved) {
            return Ok(ToolResult::error(format!(
                "Failed to create directories for '{}': {}",
                resolved.display(),
                e
            )));
        }

        match fs::write(&resolved, content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "File '{}' written successfully",
                resolved.display()
            ))
            .with_diff(file_diff)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write file '{}': {}",
                resolved.display(),
                e
            ))),
        }
    }
}
