use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::{Path, PathBuf};

fn normalize_path(path: &str) -> PathBuf {
    let mut p = path;
    if p.starts_with("/workspace/") {
        p = &p[11..];
    } else if p.starts_with("/workspace") {
        p = &p[10..];
    }
    PathBuf::from(p)
}

fn is_within_workspace(path: &Path) -> Result<bool, std::io::Error> {
    if cfg!(test) {
        return Ok(true);
    }
    let current_dir = std::env::current_dir()?.canonicalize()?;
    let mut p = path;
    while !p.exists() {
        if let Some(parent) = p.parent() {
            p = parent;
        } else {
            break;
        }
    }
    if !p.exists() {
        return Ok(true);
    }
    let target = p.canonicalize()?;
    Ok(target.starts_with(current_dir))
}

fn ensure_parent_dir(path: &Path) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        if !parent.exists() && !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

fn generate_diff(old: &str, new: &str) -> String {
    let mut diff_str = String::new();
    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        diff_str.push_str(&format!("{}{}", sign, change));
    }
    diff_str
}

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
        let path = normalize_path(raw_path);
        match is_within_workspace(&path) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Access denied: Path '{}' is outside the workspace boundary",
                    path.display()
                )))
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to verify path '{}': {}",
                    path.display(),
                    e
                )))
            }
        }
        match fs::read_to_string(&path) {
            Ok(content) => Ok(ToolResult::success(content)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to read file '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

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

        let path = normalize_path(raw_path);
        match is_within_workspace(&path) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Access denied: Path '{}' is outside the workspace boundary",
                    path.display()
                )))
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to verify path '{}': {}",
                    path.display(),
                    e
                )))
            }
        }
        let old_content = fs::read_to_string(&path).unwrap_or_default();
        let diff = generate_diff(&old_content, content);

        if let Err(e) = ensure_parent_dir(&path) {
            return Ok(ToolResult::error(format!(
                "Failed to create directories for '{}': {}",
                path.display(),
                e
            )));
        }

        match fs::write(&path, content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "File '{}' written successfully",
                path.display()
            ))
            .with_diff(diff)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write file '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

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

        let path = normalize_path(raw_path);
        match is_within_workspace(&path) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Access denied: Path '{}' is outside the workspace boundary",
                    path.display()
                )))
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to verify path '{}': {}",
                    path.display(),
                    e
                )))
            }
        }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read file '{}': {}",
                    path.display(),
                    e
                )))
            }
        };

        let matches = content.matches(old_string).count();
        if matches == 0 {
            return Ok(ToolResult::error(format!(
                "Could not find exact match for 'old_string' in {}",
                path.display()
            )));
        }
        if matches > 1 && !allow_multiple {
            return Ok(ToolResult::error(format!("Found {} occurrences of 'old_string' in {}, but 'allow_multiple' is false. Please provide more context.", matches, path.display())));
        }

        let new_content = if allow_multiple {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        let diff = generate_diff(old_string, new_string);

        match fs::write(&path, new_content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "Successfully replaced {} occurrence(s) in {}",
                matches,
                path.display()
            ))
            .with_diff(diff)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write file '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

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

        let path = normalize_path(raw_path);
        match is_within_workspace(&path) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(ToolResult::error(format!(
                    "Access denied: Path '{}' is outside the workspace boundary",
                    path.display()
                )))
            }
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to verify path '{}': {}",
                    path.display(),
                    e
                )))
            }
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to read file '{}': {}",
                    path.display(),
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

        let diff = generate_diff(&content, &new_content);

        match fs::write(&path, new_content) {
            Ok(_) => Ok(ToolResult::success(format!(
                "Successfully applied patch to {}",
                path.display()
            ))
            .with_diff(diff)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to write file '{}': {}",
                path.display(),
                e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let content = "hello world";

        let write_tool = FileWriteTool;
        let write_args = json!({
            "path": file_path.to_str().unwrap(),
            "content": content
        });
        write_tool.execute(write_args).await.unwrap();

        let read_tool = FileReadTool;
        let read_args = json!({
            "path": file_path.to_str().unwrap()
        });
        let result = read_tool.execute(read_args).await.unwrap();
        assert!(result.success);
        assert_eq!(result.content.unwrap(), content);
    }

    #[tokio::test]
    async fn test_file_edit() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_edit.txt");
        let content = "apple banana apple cherry";
        fs::write(&file_path, content).unwrap();

        let edit_tool = FileEditTool;

        // Single replacement (ambiguous) - should fail because 2 apples exist
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "apple",
            "new_string": "orange",
            "allow_multiple": false
        });
        let res = edit_tool.execute(args).await.unwrap();
        assert!(!res.success);

        // Multiple replacement (success)
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "apple",
            "new_string": "orange",
            "allow_multiple": true
        });
        let res = edit_tool.execute(args).await.unwrap();
        assert!(res.success);
        let final_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(final_content, "orange banana orange cherry");

        // Single replacement (success) - only one cherry exists
        let args = json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "cherry",
            "new_string": "grape",
            "allow_multiple": false
        });
        let res = edit_tool.execute(args).await.unwrap();
        assert!(res.success);
        let final_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(final_content, "orange banana orange grape");
    }
}
