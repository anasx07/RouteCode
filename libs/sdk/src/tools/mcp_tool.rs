use crate::core::ToolResult;
use crate::tools::mcp::client::McpClient;
use crate::tools::traits::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct DynamicMcpTool {
    prefixed_name: String,
    actual_name: String,
    description: String,
    input_schema: Value,
    client: Arc<McpClient>,
}

impl DynamicMcpTool {
    pub fn new(
        prefixed_name: String,
        actual_name: String,
        description: String,
        input_schema: Value,
        client: Arc<McpClient>,
    ) -> Self {
        Self {
            prefixed_name,
            actual_name,
            description,
            input_schema,
            client,
        }
    }
}

#[async_trait]
impl Tool for DynamicMcpTool {
    fn name(&self) -> &str {
        &self.prefixed_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let params = json!({
            "name": self.actual_name,
            "arguments": args
        });

        match self.client.request("tools/call", Some(params)).await {
            Ok(res) => {
                let is_error = res.get("isError").and_then(|e| e.as_bool()).unwrap_or(false);
                
                let mut output = String::new();
                if let Some(content) = res.get("content").and_then(|c| c.as_array()) {
                    for item in content {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                            output.push_str(text);
                            output.push('\n');
                        }
                    }
                } else {
                    output = serde_json::to_string_pretty(&res)?;
                }
                
                let result_text = output.trim().to_string();
                if is_error {
                    Ok(ToolResult::error(&result_text))
                } else {
                    Ok(ToolResult::success(result_text))
                }
            }
            Err(e) => Ok(ToolResult::error(&format!("MCP Tool Execution Failed: {}", e))),
        }
    }
}
