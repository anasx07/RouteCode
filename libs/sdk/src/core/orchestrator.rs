use crate::agents::types::StreamChunk;
use crate::agents::AIProvider;
use crate::core::{Config, Message};
use crate::tools::ToolRegistry;
use crate::utils::costs::Usage;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AgentOrchestrator {
    provider: Mutex<Arc<dyn AIProvider>>,
    tool_registry: Arc<ToolRegistry>,
    pub config: Arc<Mutex<Config>>,
    pub usage: Arc<Mutex<Usage>>,
}

impl AgentOrchestrator {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: Arc<Mutex<Config>>,
    ) -> Self {
        Self {
            provider: Mutex::new(provider),
            tool_registry,
            config,
            usage: Arc::new(Mutex::new(Usage::default())),
        }
    }

    pub async fn get_provider_name(&self) -> String {
        let p = self.provider.lock().await;
        p.name().to_string()
    }

    pub async fn change_provider(&self, new_provider: Arc<dyn AIProvider>) {
        let mut p = self.provider.lock().await;
        *p = new_provider;
    }

    async fn prepare_messages(&self, history: &[Message]) -> Vec<Message> {
        let mut messages = Vec::new();

        // 1. Build System Prompt with Project Context
        let mut system_content = String::from(
            "You are RouteCode, a senior software engineer AI coding assistant.\n\
            You help users with their codebase through a terminal interface.\n",
        );

        // Inject Project Context
        if let Ok(readme) = std::fs::read_to_string("README.md") {
            system_content.push_str("\n--- PROJECT README ---\n");
            system_content.push_str(&readme);
        }
        if let Ok(routecode_md) = std::fs::read_to_string("ROUTECODE.md") {
            system_content.push_str("\n--- PROJECT INSTRUCTIONS (ROUTECODE.md) ---\n");
            system_content.push_str(&routecode_md);
        }

        messages.push(Message::system(system_content));

        // 2. Add history
        messages.extend(history.iter().cloned());

        // 3. Truncate if necessary (Sliding Window)
        // Most modern models handle 128k+, we'll target a safe 100k for the sliding window
        let max_tokens = 100_000;
        while crate::utils::tokens::count_tokens(&messages) > max_tokens && messages.len() > 2 {
            // Remove the oldest message after the system prompt (index 1)
            messages.remove(1);
        }

        messages
    }

    pub async fn run(
        &self,
        history: &mut Vec<Message>,
        model: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<(), anyhow::Error> {
        let tools = Some(self.tool_registry.get_all_schemas());
        let messages = self.prepare_messages(history).await;

        let stream = {
            let p = self.provider.lock().await;
            p.ask(messages, model, tools).await?
        };

        let mut stream = stream;

        let mut assistant_content = String::new();
        let mut assistant_thought = String::new();
        let mut tool_calls: Vec<crate::core::ToolCall> = Vec::new();

        while let Some(chunk_res) = stream.next().await {
            let chunk = chunk_res?;

            if let Some(ref tx) = tx {
                if let Err(e) = tx.send(chunk.clone()) {
                    log::error!("Failed to send chunk to UI: {}", e);
                }
            }

            match chunk {
                StreamChunk::Text { content } => {
                    assistant_content.push_str(&content);
                }
                StreamChunk::Thought { content } => {
                    assistant_thought.push_str(&content);
                }
                StreamChunk::ToolCall { tool_call } => {
                    if let Some(idx) = tool_call.index {
                        if let Some(existing) =
                            tool_calls.iter_mut().find(|tc| tc.index == Some(idx))
                        {
                            *existing = tool_call;
                        } else {
                            tool_calls.push(tool_call);
                        }
                    } else {
                        tool_calls.push(tool_call);
                    }
                }
                StreamChunk::Usage { usage } => {
                    let mut u = self.usage.lock().await;
                    u.add(usage.prompt_tokens, usage.completion_tokens, model);
                }
                StreamChunk::Error { content } => {
                    return Err(anyhow::anyhow!("Provider error: {}", content));
                }
                StreamChunk::ToolResult { .. } => {}
                StreamChunk::Done => {}
            }
        }

        let assistant_msg = Message::assistant(
            if assistant_content.is_empty() {
                None
            } else {
                Some(assistant_content)
            },
            if assistant_thought.is_empty() {
                None
            } else {
                Some(assistant_thought)
            },
            if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls.clone())
            },
        );

        history.push(assistant_msg);

        if !tool_calls.is_empty() {
            for tc in tool_calls {
                if let Some(tool) = self.tool_registry.get(&tc.function.name) {
                    let args = serde_json::from_str(&tc.function.arguments)?;
                    let result = tool.execute(args).await?;
                    let content = serde_json::to_string(&result)?;

                    let tool_msg =
                        Message::tool(tc.id.clone(), tc.function.name.clone(), content.clone());
                    history.push(tool_msg);

                    if let Some(ref tx) = tx {
                        if let Err(e) = tx.send(StreamChunk::ToolResult {
                            tool_call_id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            content: content.clone(),
                        }) {
                            log::error!("Failed to send tool result to UI: {}", e);
                        }
                    }
                }
            }
            // Recurse after tool execution
            return Box::pin(self.run(history, model, tx)).await;
        }

        if let Some(ref tx) = tx {
            if let Err(e) = tx.send(StreamChunk::Done) {
                log::error!("Failed to send Done chunk to UI: {}", e);
            }
        }

        Ok(())
    }
}
