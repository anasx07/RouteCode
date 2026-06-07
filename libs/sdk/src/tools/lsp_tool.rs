use crate::core::ToolResult;
use crate::tools::lsp::manager::LspManager;
use crate::tools::traits::Tool;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use lsp_types::{
    GotoDefinitionParams, HoverParams, Position, ReferenceContext, ReferenceParams,
    TextDocumentIdentifier, TextDocumentPositionParams, Url,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default)]
pub struct LspTool {
    manager: Arc<LspManager>,
}

impl LspTool {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(LspManager::new()),
        }
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "lsp"
    }

    fn description(&self) -> &str {
        "Query a Language Server (LSP) for semantic code information (goToDefinition, findReferences, hover). Provide operation, filePath, line, and character (1-indexed)."
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["goToDefinition", "findReferences", "hover"]
                },
                "filePath": {
                    "type": "string",
                    "description": "Absolute path to the file"
                },
                "line": {
                    "type": "integer",
                    "description": "1-based line number"
                },
                "character": {
                    "type": "integer",
                    "description": "1-based character offset"
                }
            },
            "required": ["operation", "filePath", "line", "character"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<ToolResult> {
        let operation = params["operation"].as_str().unwrap_or("").to_string();
        let file_path = params["filePath"].as_str().unwrap_or("");
        let line = params["line"].as_u64().unwrap_or(1).saturating_sub(1) as u32; // 0-indexed internally
        let character = params["character"].as_u64().unwrap_or(1).saturating_sub(1) as u32;

        let path = PathBuf::from(file_path);
        if !path.exists() {
            return Ok(ToolResult::error("File does not exist"));
        }

        let uri = Url::from_file_path(&path).map_err(|_| anyhow!("Invalid path"))?;

        // Open Document notification (LSP servers require the file to be "opened" to answer queries)
        let client = self.manager.get_or_spawn_client(&path).await?;

        let content = std::fs::read_to_string(&path)?;
        let did_open_params = lsp_types::DidOpenTextDocumentParams {
            text_document: lsp_types::TextDocumentItem {
                uri: uri.clone(),
                language_id: path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                version: 1,
                text: content,
            },
        };
        // We notify but don't await response, it's just a notification
        let _ = client
            .notify(
                "textDocument/didOpen",
                serde_json::to_value(did_open_params)?,
            )
            .await;

        let text_doc_pos = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position { line, character },
        };

        let result = match operation.as_str() {
            "goToDefinition" => {
                let params = GotoDefinitionParams {
                    text_document_position_params: text_doc_pos,
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                };
                client
                    .request("textDocument/definition", serde_json::to_value(params)?)
                    .await?
            }
            "findReferences" => {
                let params = ReferenceParams {
                    text_document_position: text_doc_pos,
                    context: ReferenceContext {
                        include_declaration: true,
                    },
                    work_done_progress_params: Default::default(),
                    partial_result_params: Default::default(),
                };
                client
                    .request("textDocument/references", serde_json::to_value(params)?)
                    .await?
            }
            "hover" => {
                let params = HoverParams {
                    text_document_position_params: text_doc_pos,
                    work_done_progress_params: Default::default(),
                };
                client
                    .request("textDocument/hover", serde_json::to_value(params)?)
                    .await?
            }
            _ => return Ok(ToolResult::error("Unsupported operation")),
        };

        Ok(ToolResult::success(serde_json::to_string_pretty(&result)?))
    }
}
