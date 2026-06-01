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
    pub allow_session_commands: std::sync::atomic::AtomicBool,
    pub allow_session_outside_access: std::sync::atomic::AtomicBool,
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
            allow_session_commands: std::sync::atomic::AtomicBool::new(false),
            allow_session_outside_access: std::sync::atomic::AtomicBool::new(false),
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
            You help users with their codebase through a terminal interface.\n\
            You have access to tools for file operations, navigation, and bash commands.\n\
            When you need to explore or modify the codebase, use the appropriate tools.\n",
        );

        let project_root = crate::utils::storage::find_project_root();

        // Inject Project Context
        if let Ok(readme) = std::fs::read_to_string(project_root.join("README.md")) {
            system_content.push_str("\n--- PROJECT README ---\n");
            system_content.push_str(&readme);
        }
        if let Ok(routecode_md) = std::fs::read_to_string(project_root.join("ROUTECODE.md")) {
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
        match self.run_with_depth(history, model, tx.clone(), 0).await {
            Ok(_) => Ok(()),
            Err(e) => {
                if let Some(ref tx) = tx {
                    let _ = tx.send(StreamChunk::Error { content: e.to_string() });
                    let _ = tx.send(StreamChunk::Done);
                }
                Err(e)
            }
        }
    }

    async fn run_with_depth(
        &self,
        history: &mut Vec<Message>,
        model: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
        depth: usize,
    ) -> Result<(), anyhow::Error> {
        if depth >= 25 {
            return Err(anyhow::anyhow!("Maximum tool recursion depth (25) reached. Aborting to prevent infinite loop."));
        }

        let tools = Some(self.tool_registry.get_all_schemas());
        let messages = self.prepare_messages(history).await;

        log::debug!("Sending AI request to model: {} (messages: {})", model, messages.len());

        let thinking_level = {
            let config = self.config.lock().await;
            config.thinking_level.clone()
        };

        let stream = {
            let p = self.provider.lock().await;
            p.ask(messages, model, tools, Some(&thinking_level)).await?
        };

        let mut stream = stream;

        let mut assistant_content = String::new();
        let mut assistant_thought = String::new();
        let mut tool_calls: Vec<crate::core::ToolCall> = Vec::new();

        while let Some(chunk_res) = stream.next().await {
            let chunk = match chunk_res {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Stream error: {}", e);
                    return Err(e);
                }
            };
            log::debug!("Received chunk: {:?}", chunk);

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
                    u.add(usage.prompt_tokens, usage.completion_tokens, model).await;
                }
                StreamChunk::Error { content } => {
                    return Err(anyhow::anyhow!("Provider error: {}", content));
                }
                StreamChunk::ToolResult { .. } => {}
                StreamChunk::Done => {}
                StreamChunk::FinalHistory { .. } => {}
                StreamChunk::Models { .. } => {}
                StreamChunk::ModelsDone => {}
                StreamChunk::RequestConfirmation { .. } => {}
                StreamChunk::UpdateAvailable { .. } => {}
            }
        }

        let assistant_msg = Message::assistant(
            if assistant_content.is_empty() {
                if !assistant_thought.is_empty() || !tool_calls.is_empty() {
                    Some(std::sync::Arc::from(""))
                } else {
                    None
                }
            } else {
                Some(std::sync::Arc::from(assistant_content))
            },
            if assistant_thought.is_empty() {
                None
            } else {
                Some(std::sync::Arc::from(assistant_thought))
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
                    let args: serde_json::Value = match serde_json::from_str(&tc.function.arguments) {
                        Ok(a) => a,
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to parse tool arguments: {}. \
                                This usually means the AI's response was truncated because it reached its output token limit. \
                                Try asking for a smaller part of the task or increasing the limit.",
                                e
                            ));
                        }
                    };
                    let mut execute_allowed = true;
                    let mut custom_error_msg = None;
                    
                    use std::sync::atomic::Ordering;

                    if tc.function.name == "bash" {
                        if !self.allow_session_commands.load(Ordering::SeqCst) {
                            if let Some(ref sender) = tx {
                                let command_str = args["command"].as_str().unwrap_or("").to_string();
                                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                                let tx_wrapped = Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));
                                
                                if let Err(e) = sender.send(StreamChunk::RequestConfirmation {
                                    message: "The AI agent wants to execute the following bash command:".to_string(),
                                    target: command_str,
                                    tx: Some(tx_wrapped),
                                }) {
                                    log::error!("Failed to send RequestConfirmation to UI: {}", e);
                                }

                                match oneshot_rx.await {
                                    Ok(crate::agents::types::ConfirmationResponse::AllowOnce) => {}
                                    Ok(crate::agents::types::ConfirmationResponse::AllowSession) | Ok(crate::agents::types::ConfirmationResponse::AllowWorkspace) => {
                                        self.allow_session_commands.store(true, Ordering::SeqCst);
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Deny) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some("Command execution denied by user.".to_string());
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Feedback(msg)) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!("Command execution denied by user with feedback: {}", msg));
                                    }
                                    Err(_) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some("Command execution cancelled (confirmation channel closed).".to_string());
                                    }
                                }
                            }
                        }
                    } else if ["file_read", "file_write", "file_edit", "ls", "tree", "grep"].contains(&tc.function.name.as_str())
                        && !self.allow_session_outside_access.load(Ordering::SeqCst)
                    {
                        let path_str = args["path"].as_str().unwrap_or(".");
                        if crate::utils::storage::is_path_outside_workspace(path_str) {
                            if let Some(ref sender) = tx {
                                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                                let tx_wrapped = Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));

                                if let Err(e) = sender.send(StreamChunk::RequestConfirmation {
                                    message: "The AI agent wants to access a path OUTSIDE the current workspace:".to_string(),
                                    target: path_str.to_string(),
                                    tx: Some(tx_wrapped),
                                }) {
                                    log::error!("Failed to send RequestConfirmation to UI: {}", e);
                                }

                                match oneshot_rx.await {
                                    Ok(crate::agents::types::ConfirmationResponse::AllowOnce) => {}
                                    Ok(crate::agents::types::ConfirmationResponse::AllowSession) | Ok(crate::agents::types::ConfirmationResponse::AllowWorkspace) => {
                                        self.allow_session_outside_access.store(true, Ordering::SeqCst);
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Deny) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!("Access to outside path '{}' denied by user.", path_str));
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Feedback(msg)) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!("Access to outside path '{}' denied by user with feedback: {}", path_str, msg));
                                    }
                                    Err(_) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some("Access cancelled (confirmation channel closed).".to_string());
                                    }
                                }
                            } else {
                                // If there's no UI (headless), just block it by default.
                                execute_allowed = false;
                                custom_error_msg = Some(format!("Access to outside path '{}' denied (no UI confirmation available).", path_str));
                            }
                        }
                    }

                    let result = if execute_allowed {
                        match tool.execute(args).await {
                            Ok(res) => res,
                            Err(e) => crate::core::ToolResult::error(format!("Tool execution failed: {}", e)),
                        }
                    } else {
                        crate::core::ToolResult::error(custom_error_msg.unwrap_or_default())
                    };
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
            return Box::pin(self.run_with_depth(history, model, tx, depth + 1)).await;
        }

        if let Some(ref tx) = tx {
            let _ = tx.send(StreamChunk::FinalHistory { history: history.clone() });
            if let Err(e) = tx.send(StreamChunk::Done) {
                log::error!("Failed to send Done chunk to UI: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::StreamChunk;
    use crate::core::{Message, Role, ToolCall, FunctionCall, ToolResult};
    use crate::tools::traits::Tool;
    use async_trait::async_trait;
    use futures::stream;
    use serde_json::json;

    struct MockProvider {
        responses: Mutex<Vec<Vec<StreamChunk>>>,
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str { "Mock" }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> { Ok(vec!["mock".to_string()]) }
        async fn ask(&self, _msgs: Vec<Message>, _model: &str, _tools: Option<Vec<serde_json::Value>>, _thinking_level: Option<&str>) -> Result<crate::agents::traits::StreamResponse, anyhow::Error> {
            let mut resps = self.responses.lock().await;
            if resps.is_empty() {
                return Err(anyhow::anyhow!("No more mock responses"));
            }
            let chunks = resps.remove(0);
            let s = stream::iter(chunks.into_iter().map(Ok));
            Ok(Box::pin(s))
        }
    }

    struct MockTool;
    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str { "mock_tool" }
        fn description(&self) -> &str { "A mock tool" }
        fn parameters(&self) -> serde_json::Value { json!({}) }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, anyhow::Error> {
            Ok(ToolResult::success("success"))
        }
    }

    #[tokio::test]
    async fn test_orchestrator_simple_chat() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(vec![vec![
                StreamChunk::Text { content: "Hello!".to_string() },
                StreamChunk::Done,
            ]]),
        });
        let tool_registry = ToolRegistry::new();
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Hi")];
        orchestrator.run(&mut history, "mock", None).await.unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[1].role, Role::Assistant);
        assert_eq!(history[1].content.as_deref(), Some("Hello!"));
    }

    #[tokio::test]
    async fn test_orchestrator_tool_use() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(vec![
                // First response: call tool
                vec![
                    StreamChunk::ToolCall {
                        tool_call: ToolCall {
                            id: "call_1".to_string(),
                            r#type: "function".to_string(),
                            index: Some(0),
                            function: FunctionCall {
                                name: "mock_tool".to_string(),
                                arguments: "{}".to_string(),
                            },
                        }
                    },
                    StreamChunk::Done,
                ],
                // Second response: finalize
                vec![
                    StreamChunk::Text { content: "Tool executed!".to_string() },
                    StreamChunk::Done,
                ]
            ]),
        });
        
        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(Arc::new(MockTool));
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Run tool")];
        orchestrator.run(&mut history, "mock", None).await.unwrap();

        // History: User -> Assistant (ToolCall) -> ToolResult -> Assistant (Final)
        assert_eq!(history.len(), 4);
        assert_eq!(history[1].role, Role::Assistant);
        assert!(history[1].tool_calls.is_some());
        assert_eq!(history[2].role, Role::Tool);
        assert_eq!(history[3].role, Role::Assistant);
        assert_eq!(history[3].content.as_deref(), Some("Tool executed!"));
    }
}
