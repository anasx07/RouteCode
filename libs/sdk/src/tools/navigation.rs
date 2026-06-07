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

pub struct TreeTool;

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

        fn walk(
            dir: &std::path::Path,
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
                    let new_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
                    walk(&path, &new_prefix, current_depth + 1, max_depth, output)?;
                }
            }
            Ok(())
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
        let include = args["include"].as_str();

        let glob_pattern = if let Some(inc) = include {
            Some(
                glob::Pattern::new(inc)
                    .map_err(|e| anyhow::anyhow!("Invalid glob pattern '{}': {}", inc, e))?,
            )
        } else {
            None
        };

        let regex_pattern = regex::Regex::new(pattern).ok();

        // Using a simple recursive walk for grep
        let mut results = Vec::new();
        fn walk_and_search(
            dir: &std::path::Path,
            search_root: &std::path::Path,
            pattern: &str,
            regex_pattern: Option<&regex::Regex>,
            glob_pattern: Option<&glob::Pattern>,
            results: &mut Vec<String>,
        ) -> io::Result<()> {
            if dir.is_dir() {
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == ".git" || name == "node_modules" || name == "target" {
                        continue;
                    }
                    if path.is_dir() {
                        walk_and_search(
                            &path,
                            search_root,
                            pattern,
                            regex_pattern,
                            glob_pattern,
                            results,
                        )?;
                    } else {
                        if let Some(glob_pat) = glob_pattern {
                            let mut matches = false;
                            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                                if glob_pat.matches(filename) {
                                    matches = true;
                                }
                            }
                            if !matches {
                                if let Ok(rel_path) = path.strip_prefix(search_root) {
                                    if glob_pat.matches_path(rel_path) {
                                        matches = true;
                                    }
                                }
                            }
                            if !matches && glob_pat.matches_path(&path) {
                                matches = true;
                            }
                            if !matches {
                                continue;
                            }
                        }

                        let is_binary = || -> bool {
                            use std::io::Read;
                            if let Ok(mut file) = fs::File::open(&path) {
                                let mut buffer = [0; 1024];
                                if let Ok(bytes_read) = file.read(&mut buffer) {
                                    return buffer[..bytes_read].contains(&0);
                                }
                            }
                            false
                        };

                        if is_binary() {
                            continue;
                        }

                        if let Ok(content) = fs::read_to_string(&path) {
                            for (idx, line) in content.lines().enumerate() {
                                let is_match = if let Some(rx) = regex_pattern {
                                    rx.is_match(line)
                                } else {
                                    line.contains(pattern)
                                };
                                if is_match {
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
            }
            Ok(())
        }

        use std::io;
        let search_root = std::path::Path::new(path);
        if let Err(e) = walk_and_search(
            search_root,
            search_root,
            pattern,
            regex_pattern.as_ref(),
            glob_pattern.as_ref(),
            &mut results,
        ) {
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

        let file_path_rs = dir.path().join("test.rs");
        fs::write(&file_path_rs, "line 1: hello in rust").unwrap();

        let tool = GrepTool;

        // Test normal grep without include filter
        let args = json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap()
        });
        let res = tool.execute(args).await.unwrap();

        assert!(res.success);
        let content = res.content.unwrap();
        assert!(content.contains("test.txt:1: line 1: hello"));
        assert!(content.contains("test.txt:3: line 3: hello again"));
        assert!(content.contains("test.rs:1: line 1: hello in rust"));

        // Test grep with include filter (*.rs)
        let args_inc = json!({
            "pattern": "hello",
            "path": dir.path().to_str().unwrap(),
            "include": "*.rs"
        });
        let res_inc = tool.execute(args_inc).await.unwrap();

        assert!(res_inc.success);
        let content_inc = res_inc.content.unwrap();
        assert!(!content_inc.contains("test.txt"));
        assert!(content_inc.contains("test.rs:1: line 1: hello in rust"));

        // Test grep with Regex pattern (e.g. h[e-o]llo)
        let args_regex = json!({
            "pattern": "h[e-o]llo",
            "path": dir.path().to_str().unwrap()
        });
        let res_regex = tool.execute(args_regex).await.unwrap();

        assert!(res_regex.success);
        let content_regex = res_regex.content.unwrap();
        assert!(content_regex.contains("test.txt:1: line 1: hello"));
        assert!(content_regex.contains("test.txt:3: line 3: hello again"));
        assert!(content_regex.contains("test.rs:1: line 1: hello in rust"));
    }
}
