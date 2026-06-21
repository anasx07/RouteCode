pub mod grep;
pub mod ls;
pub mod tree;
pub mod walk;

pub use grep::GrepTool;
pub use ls::LsTool;
pub use tree::TreeTool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::Tool;
    use serde_json::json;
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
