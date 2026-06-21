use super::{diff, path};
use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct ApplyPatchTool;

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch to a file. Useful for making complex modifications without replacing the whole file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The path to the file being patched" },
                "patch_text": { "type": "string", "description": "The unified diff patch string to apply" }
            },
            "required": ["path", "patch_text"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let raw_path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let patch_text = args["patch_text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing patch_text"))?;

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

        let content = match fs::read_to_string(&resolved) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read file '{}': {}",
                    resolved.display(),
                    e
                )))
            }
        };

        let patch = match diffy::Patch::from_str(patch_text) {
            Ok(p) => p,
            Err(e) => return Ok(ToolResult::error(format!("Failed to parse patch: {}", e))),
        };

        let new_content = match diffy::apply(&content, &patch) {
            Ok(c) => c,
            Err(e) => return Ok(ToolResult::error(format!("Failed to apply patch: {}", e))),
        };

        let file_diff = diff::generate(&content, &new_content);

        match fs::write(&resolved, new_content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "Successfully applied patch to {}",
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
