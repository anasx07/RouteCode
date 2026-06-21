use crate::agents::AIProvider;
use crate::core::orchestrator::AgentOrchestrator;
use crate::core::{Config, Message, ToolResult};
use crate::tools::registry::ToolRegistry;
use crate::tools::traits::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod permissions;
pub mod schema;

pub struct SubAgentTool {
    provider: Arc<dyn AIProvider>,
    tool_registry: Arc<ToolRegistry>,
    config: Arc<Mutex<Config>>,
}

impl SubAgentTool {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: Arc<Mutex<Config>>,
    ) -> Self {
        Self {
            provider,
            tool_registry,
            config,
        }
    }
}

#[async_trait]
impl Tool for SubAgentTool {
    fn name(&self) -> &str {
        schema::TOOL_NAME
    }

    fn description(&self) -> &str {
        schema::TOOL_DESCRIPTION
    }

    fn parameters(&self) -> Value {
        schema::parameters()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let prompt = schema::parse_prompt(&args)?;

        let orchestrator = AgentOrchestrator::new(
            self.provider.clone(),
            self.tool_registry.clone(),
            self.config.clone(),
        );

        let mut history = vec![Message::user(prompt.to_string())];
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let approve_handle = permissions::spawn_approver(rx);

        let model = {
            let config = self.config.lock().await;
            config.model.clone()
        };

        match orchestrator.run(&mut history, &model, Some(tx), None).await {
            Ok(_) => {
                let _ = approve_handle.await;
                if let Some(msg) = history.last() {
                    let content_str = msg.content.as_deref().unwrap_or_default().to_string();
                    Ok(ToolResult::success(content_str))
                } else {
                    Ok(ToolResult::error(
                        "Sub-agent completed but returned no final response.",
                    ))
                }
            }
            Err(e) => Ok(ToolResult::error(format!("Sub-agent failed: {}", e))),
        }
    }
}
