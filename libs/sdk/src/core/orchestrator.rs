use crate::agents::types::StreamChunk;
use crate::agents::AIProvider;
use crate::core::{Config, Message};
use crate::tools::ToolRegistry;
use crate::utils::costs::Usage;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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
            "You are RouteCode, a senior software engineer AI coding assistant. \
            You help users work with their codebase through a terminal interface.\n\
            \n\
            # Tool use\n\
            - You have access to tools for file operations, navigation, and bash commands.\n\
            - Read files before modifying them. Use the smallest tool that solves the task.\n\
            - When a tool fails, diagnose the root cause before retrying; do not blindly re-run.\n\
            - Chain tools deliberately: if one tool's output determines the next call, do not speculatively parallelize.\n\
            \n\
            # Response style\n\
            - Be concise. Prefer short, direct answers over long preambles.\n\
            - When you create or modify a file, briefly explain why in the chat.\n\
            - Show final code in the chat (not only via tool calls) so the user can review without expanding the file.\n\
            - Use markdown sparingly: fenced code blocks for code, bullet lists for 3+ items, plain text otherwise.\n\
            \n\
            # Language\n\
            - Reply in the same language the user uses. If the user writes in Spanish, respond in Spanish; \
            if in Japanese, respond in Japanese. Default to English only when the user writes in English.\n\
            - Keep code, identifiers, file paths, and error messages in their original language (usually English) \
            regardless of your response language.\n\
            \n\
            # Anticipate edge cases\n\
            - Before answering, consider what the user may not have thought of: empty inputs, null values, \
            error states, boundary conditions, concurrency, large inputs, encoding, locale, permissions.\n\
            - If an edge case could break the user's code or change behavior, mention it briefly with a one-line fix.\n\
            - Do not pad answers with exhaustive edge-case lists when the question is simple. Mention only what matters.\n\
            \n\
            # Safety\n\
            - Never run destructive commands (rm -rf, force push, dropping databases, etc.) without explicit user confirmation.\n\
            - Never exfiltrate secrets, API keys, or PII from the workspace.\n\
            - If a request is ambiguous or potentially harmful, ask one short clarifying question before acting.\n\
            \n\
            # Project context\n\
            - The user's README.md and ROUTECODE.md are appended below. Follow any project-specific rules in them \
            as if they were part of this prompt.\n\
            - Project conventions (libraries, code style, file layout) take precedence over generic best practices.\n",
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
        cancel: Option<CancellationToken>,
    ) -> Result<(), anyhow::Error> {
        match self
            .run_with_depth(history, model, tx.clone(), 0, cancel.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let was_cancelled = cancel.as_ref().is_some_and(|c| c.is_cancelled());
                if let Some(ref tx) = tx {
                    if was_cancelled {
                        let _ = tx.send(StreamChunk::Status {
                            content: "Request cancelled by user".to_string(),
                        });
                    } else {
                        let _ = tx.send(StreamChunk::Error {
                            content: e.to_string(),
                        });
                    }
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
        cancel: Option<CancellationToken>,
    ) -> Result<(), anyhow::Error> {
        if depth >= 25 {
            return Err(anyhow::anyhow!(
                "Maximum tool recursion depth (25) reached. Aborting to prevent infinite loop."
            ));
        }
        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return Err(anyhow::anyhow!("Request cancelled by user"));
        }

        // Wrap messages and tools in Arc once. The retry wrapper reuses the
        // same allocation across attempts (Arc::clone is just a refcount bump)
        // so we don't pay the cost of cloning a 100k-token history on every
        // retry.
        let tools: Arc<Option<Vec<serde_json::Value>>> =
            Arc::new(Some(self.tool_registry.get_all_schemas()));
        let messages: Arc<Vec<Message>> = Arc::new(self.prepare_messages(history).await);

        log::debug!(
            "Sending AI request to model: {} (messages: {})",
            model,
            messages.len()
        );

        // Snapshot the retry policy, thinking-level, and provider at the start
        // of the run. A mid-request policy change is intentionally NOT honored
        // — an in-flight retry loop keeps its current policy for the rest of
        // the request to avoid half-applied policy and to make the run's retry
        // behavior deterministic for the user. Users who want to stop a
        // runaway loop should use cancellation, not toggle.
        let (thinking_level, retry_policy, provider_arc) = {
            let config = self.config.lock().await;
            let p = self.provider.lock().await;
            (
                config.thinking_level.clone(),
                config.retry_policy.clone(),
                Arc::clone(&p),
            )
        };

        // Wrap the provider in a RetryingProvider for this request. This is
        // where the retry logic lives now — the orchestrator just consumes
        // the returned stream.
        let retrying = crate::agents::RetryingProvider::new(provider_arc, retry_policy);

        let (assistant_content, assistant_thought, tool_calls) = {
            let mut s = retrying
                .ask_with_retry(
                    Arc::clone(&messages),
                    model,
                    Arc::clone(&tools),
                    Some(&thinking_level),
                    cancel.clone(),
                )
                .await?;

            let mut local_content = String::new();
            let mut local_thought = String::new();
            let mut local_tool_calls: Vec<crate::core::ToolCall> = Vec::new();
            let mut chunk_buffer: Vec<StreamChunk> = Vec::new();

            while let Some(chunk_res) = s.next().await {
                // Check for user cancellation between stream chunks.
                if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
                    return Err(anyhow::anyhow!("Request cancelled by user"));
                }

                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(e);
                    }
                };

                // Detect retry events emitted by the wrapper and bump the
                // session-aggregate UI state in real time. The status text is
                // produced by the wrapper, so the strings are stable.
                if let StreamChunk::Status { content } = &chunk {
                    if content.starts_with("QIR retrying")
                        || content.starts_with("QIR stream interrupted")
                    {
                        let mut u = self.usage.lock().await;
                        u.record_qir_attempt();
                        if let Some(ref tx) = tx {
                            let _ = tx.send(StreamChunk::SessionStats {
                                total_tokens: u.total_tokens,
                                total_cost: u.total_cost,
                                qir_attempts: u.qir_attempts,
                            });
                        }
                    }
                }

                // Capture usage for the session aggregate as we go (success
                // path will have flushed chunks; this catches them in
                // non-buffered flows too).
                if let StreamChunk::Usage { usage } = &chunk {
                    let mut u = self.usage.lock().await;
                    u.add(usage.prompt_tokens, usage.completion_tokens, model)
                        .await;
                }

                // Accumulate the final assistant message.
                match &chunk {
                    StreamChunk::Text { content } => {
                        local_content.push_str(content);
                    }
                    StreamChunk::Thought { content } => {
                        local_thought.push_str(content);
                    }
                    StreamChunk::ToolCall { tool_call } => {
                        if let Some(idx) = tool_call.index {
                            if let Some(existing) =
                                local_tool_calls.iter_mut().find(|tc| tc.index == Some(idx))
                            {
                                *existing = tool_call.clone();
                            } else {
                                local_tool_calls.push(tool_call.clone());
                            }
                        } else {
                            local_tool_calls.push(tool_call.clone());
                        }
                    }
                    // A `StreamChunk::Error` reaching the orchestrator means
                    // either (a) the retry wrapper was bypassed (e.g. policy
                    // is `Disabled` and the provider emitted one mid-stream),
                    // or (b) the wrapper is being misused as a passthrough.
                    // In both cases the spec is "surface to the UI and abort
                    // cleanly". `run()` above wraps the returned Err with a
                    // `StreamChunk::Error` + `Done` so the consumer can
                    // finalize without hanging.
                    StreamChunk::Error { content } => {
                        return Err(anyhow::anyhow!("Provider error: {}", content));
                    }
                    _ => {}
                }

                // Buffer Done / Status / etc. for the final flush below.
                chunk_buffer.push(chunk);
            }

            // Final SessionStats snapshot so the UI sees the up-to-date
            // aggregate after a successful (possibly retried) attempt.
            if let Some(ref tx) = tx {
                let u = self.usage.lock().await;
                let _ = tx.send(StreamChunk::SessionStats {
                    total_tokens: u.total_tokens,
                    total_cost: u.total_cost,
                    qir_attempts: u.qir_attempts,
                });
            }

            // Flush all buffered chunks to the UI (this is the chunks from the
            // successful attempt; the wrapper buffered failed attempts'
            // chunks internally and only flushed the final one).
            for chunk in chunk_buffer {
                if let Some(ref tx) = tx {
                    if let Err(e) = tx.send(chunk) {
                        log::error!("Failed to send chunk to UI: {}", e);
                    }
                }
            }

            (local_content, local_thought, local_tool_calls)
        };

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
                    let args: serde_json::Value = match serde_json::from_str(&tc.function.arguments)
                    {
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
                                let command_str =
                                    args["command"].as_str().unwrap_or("").to_string();
                                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                                let tx_wrapped =
                                    Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));

                                if let Err(e) = sender.send(StreamChunk::RequestConfirmation {
                                    message:
                                        "The AI agent wants to execute the following bash command:"
                                            .to_string(),
                                    target: command_str,
                                    tx: Some(tx_wrapped),
                                }) {
                                    log::error!("Failed to send RequestConfirmation to UI: {}", e);
                                }

                                match oneshot_rx.await {
                                    Ok(crate::agents::types::ConfirmationResponse::AllowOnce) => {}
                                    Ok(
                                        crate::agents::types::ConfirmationResponse::AllowSession,
                                    )
                                    | Ok(
                                        crate::agents::types::ConfirmationResponse::AllowWorkspace,
                                    ) => {
                                        self.allow_session_commands.store(true, Ordering::SeqCst);
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Deny) => {
                                        execute_allowed = false;
                                        custom_error_msg =
                                            Some("Command execution denied by user.".to_string());
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Feedback(
                                        msg,
                                    )) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!(
                                            "Command execution denied by user with feedback: {}",
                                            msg
                                        ));
                                    }
                                    Err(_) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some("Command execution cancelled (confirmation channel closed).".to_string());
                                    }
                                }
                            }
                        }
                    } else if ["file_read", "file_write", "file_edit", "ls", "tree", "grep"]
                        .contains(&tc.function.name.as_str())
                        && !self.allow_session_outside_access.load(Ordering::SeqCst)
                    {
                        let path_str = args["path"].as_str().unwrap_or(".");
                        if crate::utils::storage::is_path_outside_workspace(path_str) {
                            if let Some(ref sender) = tx {
                                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                                let tx_wrapped =
                                    Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));

                                if let Err(e) = sender.send(StreamChunk::RequestConfirmation {
                                    message: "The AI agent wants to access a path OUTSIDE the current workspace:".to_string(),
                                    target: path_str.to_string(),
                                    tx: Some(tx_wrapped),
                                }) {
                                    log::error!("Failed to send RequestConfirmation to UI: {}", e);
                                }

                                match oneshot_rx.await {
                                    Ok(crate::agents::types::ConfirmationResponse::AllowOnce) => {}
                                    Ok(
                                        crate::agents::types::ConfirmationResponse::AllowSession,
                                    )
                                    | Ok(
                                        crate::agents::types::ConfirmationResponse::AllowWorkspace,
                                    ) => {
                                        self.allow_session_outside_access
                                            .store(true, Ordering::SeqCst);
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Deny) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!(
                                            "Access to outside path '{}' denied by user.",
                                            path_str
                                        ));
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Feedback(
                                        msg,
                                    )) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!("Access to outside path '{}' denied by user with feedback: {}", path_str, msg));
                                    }
                                    Err(_) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(
                                            "Access cancelled (confirmation channel closed)."
                                                .to_string(),
                                        );
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
                            Err(e) => crate::core::ToolResult::error(format!(
                                "Tool execution failed: {}",
                                e
                            )),
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
            return Box::pin(self.run_with_depth(history, model, tx, depth + 1, cancel)).await;
        }

        if let Some(ref tx) = tx {
            let _ = tx.send(StreamChunk::FinalHistory {
                history: history.clone(),
            });
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
    use crate::core::{FunctionCall, Message, Role, ToolCall, ToolResult};
    use crate::tools::traits::Tool;
    use async_trait::async_trait;
    use futures::stream;
    use serde_json::json;

    struct MockProvider {
        responses: Mutex<Vec<Vec<StreamChunk>>>,
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec!["mock".to_string()])
        }
        async fn ask(
            &self,
            _msgs: Arc<Vec<Message>>,
            _model: &str,
            _tools: Arc<Option<Vec<serde_json::Value>>>,
            _thinking_level: Option<&str>,
        ) -> Result<crate::agents::traits::StreamResponse, anyhow::Error> {
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
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "A mock tool"
        }
        fn parameters(&self) -> serde_json::Value {
            json!({})
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, anyhow::Error> {
            Ok(ToolResult::success("success"))
        }
    }

    #[tokio::test]
    async fn test_orchestrator_simple_chat() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(vec![vec![
                StreamChunk::Text {
                    content: "Hello!".to_string(),
                },
                StreamChunk::Done,
            ]]),
        });
        let tool_registry = ToolRegistry::new();
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Hi")];
        orchestrator
            .run(&mut history, "mock", None, None)
            .await
            .unwrap();

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
                        },
                    },
                    StreamChunk::Done,
                ],
                // Second response: finalize
                vec![
                    StreamChunk::Text {
                        content: "Tool executed!".to_string(),
                    },
                    StreamChunk::Done,
                ],
            ]),
        });

        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(Arc::new(MockTool));
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Run tool")];
        orchestrator
            .run(&mut history, "mock", None, None)
            .await
            .unwrap();

        // History: User -> Assistant (ToolCall) -> ToolResult -> Assistant (Final)
        assert_eq!(history.len(), 4);
        assert_eq!(history[1].role, Role::Assistant);
        assert!(history[1].tool_calls.is_some());
        assert_eq!(history[2].role, Role::Tool);
        assert_eq!(history[3].role, Role::Assistant);
        assert_eq!(history[3].content.as_deref(), Some("Tool executed!"));
    }
}
