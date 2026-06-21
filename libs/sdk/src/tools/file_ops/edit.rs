use super::{diff, path};
use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;

pub struct FileEditTool;

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "Surgically edit a file by replacing an old string with a new one"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The path to the file" },
                "old_string": { "type": "string", "description": "The exact literal text to replace" },
                "new_string": { "type": "string", "description": "The text to replace it with" },
                "allow_multiple": { "type": "boolean", "description": "Whether to replace multiple occurrences", "default": false }
            },
            "required": ["path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let raw_path = args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
        let old_string = args["old_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing old_string"))?;
        let new_string = args["new_string"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing new_string"))?;
        let allow_multiple = args["allow_multiple"].as_bool().unwrap_or(false);

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

        let matches = content.matches(old_string).count();
        if matches == 0 {
            return Ok(ToolResult::error(format!(
                "Could not find exact match for 'old_string' in {}",
                resolved.display()
            )));
        }
        if matches > 1 && !allow_multiple {
            return Ok(ToolResult::error(format!("Found {} occurrences of 'old_string' in {}, but 'allow_multiple' is false. Please provide more context.", matches, resolved.display())));
        }

        let new_content = if allow_multiple {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        let file_diff = diff::generate(old_string, new_string);

        match fs::write(&resolved, new_content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "Successfully replaced {} occurrence(s) in {}",
                matches,
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
