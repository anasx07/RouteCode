use super::walk;
use crate::core::ToolResult;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub struct GrepTool;

fn search(
    dir: &Path,
    search_root: &Path,
    pattern: &str,
    regex_pattern: Option<&regex::Regex>,
    glob_pattern: Option<&glob::Pattern>,
    results: &mut Vec<String>,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if walk::should_skip_dir(&name) {
            continue;
        }
        if path.is_dir() {
            search(&path, search_root, pattern, regex_pattern, glob_pattern, results)?;
        } else {
            if walk::is_binary(&path) {
                continue;
            }
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
    Ok(())
}

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
        let mut results = Vec::new();
        let search_root = Path::new(path);

        if let Err(e) = search(
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
            let total = results.len();
            if total > 50 {
                results.truncate(50);
                results.push(format!("\n... and {} more matches.", total - 50));
            }
            Ok(ToolResult::success(results.join("\n")))
        }
    }
}
