use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List files and directories in a given path"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The directory path to list (default: .)", "default": "." }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let path_str = args["path"].as_str().unwrap_or(".");
        let mut entries = Vec::new();

        match fs::read_dir(path_str) {
            Ok(dir) => {
                for entry in dir.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_type = if entry.path().is_dir() { "DIR" } else { "FILE" };
                    entries.push(format!("[{}] {}", file_type, file_name));
                }
                Ok(ToolResult::success(entries.join("\n")))
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to list directory: {}",
                e
            ))),
        }
    }
}
