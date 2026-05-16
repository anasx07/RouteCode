use crate::core::ToolResult;
use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error>;

    fn is_concurrency_safe(&self) -> bool {
        false
    }

    fn to_json_schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters(),
            }
        })
    }
}
