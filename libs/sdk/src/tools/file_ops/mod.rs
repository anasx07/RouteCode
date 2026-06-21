pub mod apply_patch;
pub mod diff;
pub mod edit;
pub mod path;
pub mod read;
pub mod write;

pub use apply_patch::ApplyPatchTool;
pub use edit::FileEditTool;
pub use read::FileReadTool;
pub use write::FileWriteTool;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::Tool;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let content = "hello world";

        let write_tool = FileWriteTool;
        let write_args = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": content
        });
        write_tool.execute(write_args).await.unwrap();

        let read_tool = FileReadTool;
        let read_args = serde_json::json!({
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

        let args = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "apple",
            "new_string": "orange",
            "allow_multiple": false
        });
        let res = edit_tool.execute(args).await.unwrap();
        assert!(!res.success);

        let args = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "old_string": "apple",
            "new_string": "orange",
            "allow_multiple": true
        });
        let res = edit_tool.execute(args).await.unwrap();
        assert!(res.success);
        let final_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(final_content, "orange banana orange cherry");

        let args = serde_json::json!({
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
