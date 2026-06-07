use crate::agents::AIProvider;
use crate::core::orchestrator::AgentOrchestrator;
use crate::core::Config;
use crate::core::ToolResult;
use crate::tools::registry::ToolRegistry;
use crate::tools::traits::Tool;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

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
        "delegate_sub_agent"
    }

    fn description(&self) -> &str {
        "Delegate a complex sub-task to an isolated, headless background agent. Useful for tedious research, searching codebases, or executing scripts where you want to wait for the final summarized result instead of doing it yourself step-by-step."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "The exact detailed instructions for the sub-agent. Give it context on what you want it to accomplish." }
            },
            "required": ["prompt"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, anyhow::Error> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing prompt parameter"))?;

        let orchestrator = AgentOrchestrator::new(
            self.provider.clone(),
            self.tool_registry.clone(),
            self.config.clone(),
        );

        let mut history = vec![crate::core::Message::user(prompt.to_string())];
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let approve_handle = tokio::spawn(async move {
            while let Some(chunk) = rx.recv().await {
                if let crate::agents::types::StreamChunk::RequestConfirmation {
                    tx: Some(resp_tx),
                    ..
                } = chunk
                {
                    let mut lock = resp_tx.lock().await;
                    if let Some(sender) = lock.take() {
                        let _ =
                            sender.send(crate::agents::types::ConfirmationResponse::AllowSession);
                    }
                }
            }
        });

        let config = self.config.lock().await;
        let model = config.model.clone();
        drop(config);

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
