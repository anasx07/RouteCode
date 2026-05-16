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

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "grep"
    }
    fn description(&self) -> &str {
        "Search for a pattern in files within a directory"
    }
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The regex or string pattern to search for" },
                "path": { "type": "string", "description": "The directory to search in (default: .)", "default": "." },
                "include": { "type": "string", "description": "Glob pattern for files to include (e.g., *.rs)" }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing pattern"))?;
        let path = args["path"].as_str().unwrap_or(".");

        // Using a simple recursive walk for grep
        let mut results = Vec::new();
        fn walk_and_search(
            dir: &std::path::Path,
            pattern: &str,
            results: &mut Vec<String>,
        ) -> io::Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        walk_and_search(&path, pattern, results)?;
                    } else if let Ok(content) = fs::read_to_string(&path) {
                        for (idx, line) in content.lines().enumerate() {
                            if line.contains(pattern) {
                                results.push(format!(
                                    "{}:{}: {}",
                                    path.display(),
                                    idx + 1,
                                    line.trim()
                                ));
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        use std::io;
        if let Err(e) = walk_and_search(std::path::Path::new(path), pattern, &mut results) {
            return Ok(ToolResult::error(format!("Search failed: {}", e)));
        }

        if results.is_empty() {
            Ok(ToolResult::success("No matches found.".to_string()))
        } else {
            // Limit output to first 50 results to avoid token overflow
            let total = results.len();
            if total > 50 {
                results.truncate(50);
                results.push(format!("\n... and {} more matches.", total - 50));
            }
            Ok(ToolResult::success(results.join("\n")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_ls_tool() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file1.txt"), "content").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let tool = LsTool;
        let args = json!({ "path": dir.path().to_str().unwrap() });
        let res = tool.execute(args).await.unwrap();

        assert!(res.success);
        let content = res.content.unwrap();
        assert!(content.contains("[FILE] file1.txt"));
        assert!(content.contains("[DIR] subdir"));
    }

    #[tokio::test]
    async fn test_grep_tool() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(
            &file_path,
            "line 1: hello\nline 2: world\nline 3: hello again",
        )
        .unwrap();

        let tool = GrepTool;
        let args = json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap()
        });
        let res = tool.execute(args).await.unwrap();

        assert!(res.success);
        let content = res.content.unwrap();
        assert!(content.contains("test.txt:1: line 1: hello"));
        assert!(content.contains("test.txt:3: line 3: hello again"));
        assert!(!content.contains("line 2: world"));
    }
}
