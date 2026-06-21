use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct TreeTool;

fn walk(
    dir: &Path,
    prefix: &str,
    current_depth: usize,
    max_depth: usize,
    output: &mut String,
) -> std::io::Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    let entries: Vec<_> = fs::read_dir(dir)?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name != ".git" && name != "node_modules" && name != "target"
        })
        .collect();

    let count = entries.len();
    for (idx, entry) in entries.into_iter().enumerate() {
        let is_last = idx == count - 1;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        let connector = if is_last { "└── " } else { "├── " };
        output.push_str(&format!("{}{}{}\n", prefix, connector, name));

        if path.is_dir() {
            let new_prefix = format!(
                "{}{}",
                prefix,
                if is_last { "    " } else { "│   " }
            );
            walk(&path, &new_prefix, current_depth + 1, max_depth, output)?;
        }
    }
    Ok(())
}

#[async_trait]
impl Tool for TreeTool {
    fn name(&self) -> &str {
        "tree"
    }

    fn description(&self) -> &str {
        "List files and directories recursively in a tree-like format"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "The directory path to start from (default: .)", "default": "." },
                "depth": { "type": "integer", "description": "Max recursion depth (default: 3)", "default": 3 }
            }
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let path_str = args["path"].as_str().unwrap_or(".");
        let max_depth = args["depth"].as_u64().unwrap_or(3) as usize;

        let mut output = String::new();
        let path = std::path::Path::new(path_str);

        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "Path '{}' does not exist",
                path_str
            )));
        }

        output.push_str(&format!("{}\n", path_str));
        if let Err(e) = walk(path, "", 1, max_depth, &mut output) {
            return Ok(ToolResult::error(format!(
                "Failed to walk directory: {}",
                e
            )));
        }

        Ok(ToolResult::success(output))
    }
}
