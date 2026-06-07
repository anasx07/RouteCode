use async_trait::async_trait;
use futures::stream;
use routecode_sdk::agents::resolve_provider;
use routecode_sdk::agents::types::StreamChunk;
use routecode_sdk::agents::AIProvider;
use routecode_sdk::core::orchestrator::AgentOrchestrator;
use routecode_sdk::core::{
    Config, DynamicModelInfo, FunctionCall, Message, Role, ToolCall, ToolResult,
};
use routecode_sdk::tools::bash::BashTool;
use routecode_sdk::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
use routecode_sdk::tools::navigation::{GrepTool, LsTool, TreeTool};
use routecode_sdk::tools::registry::ToolRegistry;
use routecode_sdk::tools::traits::Tool;
use routecode_sdk::utils::costs::{calculate_cost, Usage};
use routecode_sdk::utils::storage::{
    find_project_root, is_path_outside_workspace, load_config, sanitize_session_name, save_config,
};
use serde_json::json;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

fn workspace_temp_dir() -> TempDir {
    let cwd = std::env::current_dir().expect("current dir");
    let dir = cwd.join(".routecode_test_tmp");
    fs::create_dir_all(&dir).unwrap();
    TempDir::new_in(&dir).unwrap()
}

#[tokio::test]
async fn test_tool_file_write_read_edit_pipeline() {
    let dir = workspace_temp_dir();
    let file_path = dir.path().join("pipeline.txt");

    let write_tool = FileWriteTool;
    let write_args = json!({
        "path": file_path.to_str().unwrap(),
        "content": "line 1: hello\nline 2: world\nline 3: goodbye"
    });
    let res = write_tool.execute(write_args).await.unwrap();
    assert!(res.success, "FileWrite failed: {:?}", res.error);

    let read_tool = FileReadTool;
    let read_args = json!({ "path": file_path.to_str().unwrap() });
    let res = read_tool.execute(read_args.clone()).await.unwrap();
    assert!(res.success, "FileRead after write failed: {:?}", res.error);
    assert_eq!(
        res.content.unwrap(),
        "line 1: hello\nline 2: world\nline 3: goodbye"
    );

    let edit_tool = FileEditTool;
    let edit_args = json!({
        "path": file_path.to_str().unwrap(),
        "old_string": "goodbye",
        "new_string": "aloha"
    });
    let res = edit_tool.execute(edit_args).await.unwrap();
    assert!(res.success, "FileEdit failed: {:?}", res.error);

    let res = read_tool.execute(read_args.clone()).await.unwrap();
    assert!(res.success, "FileRead after edit failed: {:?}", res.error);
    assert_eq!(
        res.content.unwrap(),
        "line 1: hello\nline 2: world\nline 3: aloha"
    );
}

#[tokio::test]
async fn test_tool_ls_tree_grep() {
    let dir = workspace_temp_dir();
    fs::write(dir.path().join("alpha.rs"), "fn alpha() {}").unwrap();
    fs::write(dir.path().join("beta.txt"), "beta content").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("gamma.rs"), "fn gamma() {}").unwrap();

    let ls_tool = LsTool;
    let res = ls_tool
        .execute(json!({ "path": dir.path().to_str().unwrap() }))
        .await
        .unwrap();
    assert!(res.success);
    let content = res.content.unwrap();
    assert!(content.contains("alpha.rs"));
    assert!(content.contains("beta.txt"));
    assert!(content.contains("sub"));

    let tree_tool = TreeTool;
    let res = tree_tool
        .execute(json!({
            "path": dir.path().to_str().unwrap(),
            "depth": 2
        }))
        .await
        .unwrap();
    assert!(res.success);
    assert!(res.content.unwrap().contains("gamma.rs"));

    let grep_tool = GrepTool;
    let res = grep_tool
        .execute(json!({
            "pattern": "gamma",
            "path": dir.path().to_str().unwrap()
        }))
        .await
        .unwrap();
    assert!(res.success);
    assert!(res.content.unwrap().contains("gamma.rs"));

    let res = grep_tool
        .execute(json!({
            "pattern": "alpha",
            "path": dir.path().to_str().unwrap(),
            "include": "*.rs"
        }))
        .await
        .unwrap();
    assert!(res.success);
    assert!(res.content.unwrap().contains("alpha.rs"));

    let res = grep_tool
        .execute(json!({
            "pattern": "beta",
            "path": dir.path().to_str().unwrap(),
            "include": "*.rs"
        }))
        .await
        .unwrap();
    assert!(res.success);
    assert_eq!(res.content.unwrap(), "No matches found.");
}

#[tokio::test]
async fn test_tool_read_nonexistent_file() {
    let tool = FileReadTool;
    let args = json!({ "path": "/nonexistent/path/file.txt" });
    let res = tool.execute(args).await.unwrap();
    assert!(!res.success);
}

#[tokio::test]
async fn test_tool_edit_no_match() {
    let dir = workspace_temp_dir();
    let path = dir.path().join("edit_test.txt");
    fs::write(&path, "hello world").unwrap();

    let tool = FileEditTool;
    let args = json!({
        "path": path.to_str().unwrap(),
        "old_string": "zzzzz",
        "new_string": "yyyyy"
    });
    let res = tool.execute(args).await.unwrap();
    assert!(!res.success, "Expected error, got: {:?}", res.error);
}

#[tokio::test]
async fn test_tool_edit_ambiguous() {
    let dir = workspace_temp_dir();
    let path = dir.path().join("ambiguous.txt");
    fs::write(&path, "apple apple cherry").unwrap();

    let tool = FileEditTool;
    let args = json!({
        "path": path.to_str().unwrap(),
        "old_string": "apple",
        "new_string": "orange"
    });
    let res = tool.execute(args).await.unwrap();
    assert!(!res.success, "Expected error, got: {:?}", res.error);
}

#[tokio::test]
async fn test_tool_bash_simple() {
    if cfg!(target_os = "windows") {
        let tool = BashTool;
        let res = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(res.success);
        assert!(res.content.unwrap_or_default().contains("hello"));
    } else {
        let tool = BashTool;
        let res = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(res.success);
        assert!(res.content.unwrap_or_default().contains("hello"));
    }
}

#[tokio::test]
async fn test_tool_bash_failure() {
    let tool = BashTool;
    let res = tool.execute(json!({"command": "exit 42"})).await.unwrap();
    assert!(!res.success);
    let err = res.error.unwrap();
    assert!(err.contains("42") || err.contains("exit code"));
}

#[tokio::test]
async fn test_tool_missing_parameters() {
    let tool = FileReadTool;
    let res = tool.execute(json!({})).await;
    assert!(res.is_err());

    let tool = BashTool;
    let res = tool.execute(json!({})).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_tool_registry_integration() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(FileReadTool));
    registry.register(Arc::new(FileWriteTool));
    registry.register(Arc::new(FileEditTool));
    registry.register(Arc::new(LsTool));
    registry.register(Arc::new(TreeTool));
    registry.register(Arc::new(GrepTool));
    registry.register(Arc::new(BashTool));

    assert!(registry.get("file_read").is_some());
    assert!(registry.get("file_write").is_some());
    assert!(registry.get("file_edit").is_some());
    assert!(registry.get("ls").is_some());
    assert!(registry.get("tree").is_some());
    assert!(registry.get("grep").is_some());
    assert!(registry.get("bash").is_some());
    assert!(registry.get("nonexistent").is_none());

    let schemas = registry.get_all_schemas();
    assert_eq!(schemas.len(), 7);
    for schema in &schemas {
        assert_eq!(schema["type"], "function");
        assert!(schema["function"]["name"].as_str().is_some());
        assert!(schema["function"]["description"].as_str().is_some());
        assert!(schema["function"]["parameters"].is_object());
    }
}

#[tokio::test]
async fn test_orchestrator_recursion_depth_limit() {
    struct InfiniteLoopProvider;
    #[async_trait]
    impl AIProvider for InfiniteLoopProvider {
        fn name(&self) -> &str {
            "InfiniteLoop"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec!["mock".into()])
        }
        async fn ask(
            &self,
            _msgs: Arc<Vec<Message>>,
            _model: &str,
            _tools: Arc<Option<Vec<serde_json::Value>>>,
            _thinking_level: Option<&str>,
        ) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
            let chunks = vec![
                Ok(StreamChunk::ToolCall {
                    tool_call: ToolCall {
                        id: "call_inf".into(),
                        r#type: "function".into(),
                        index: Some(0),
                        function: FunctionCall {
                            name: "mock_tool".into(),
                            arguments: "{}".into(),
                        },
                    },
                }),
                Ok(StreamChunk::Done),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    struct RecursiveMockTool;
    #[async_trait]
    impl Tool for RecursiveMockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "recursive mock"
        }
        fn parameters(&self) -> serde_json::Value {
            json!({})
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, anyhow::Error> {
            Ok(ToolResult::success("ok"))
        }
    }

    let provider = Arc::new(InfiniteLoopProvider);
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(RecursiveMockTool));
    let config = Arc::new(tokio::sync::Mutex::new(Config::default()));
    let orchestrator = AgentOrchestrator::new(provider, Arc::new(registry), config);

    let mut history = vec![Message::user("loop")];
    let result = orchestrator.run(&mut history, "mock", None, None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Maximum tool recursion depth (25)"));
}

#[tokio::test]
async fn test_orchestrator_stream_channel() {
    struct StreamingProvider {
        call_count: Arc<AtomicUsize>,
    }
    #[async_trait]
    impl AIProvider for StreamingProvider {
        fn name(&self) -> &str {
            "Streaming"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec!["mock".into()])
        }
        async fn ask(
            &self,
            _msgs: Arc<Vec<Message>>,
            _model: &str,
            _tools: Arc<Option<Vec<serde_json::Value>>>,
            _thinking_level: Option<&str>,
        ) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let chunks = vec![
                Ok(StreamChunk::Text {
                    content: "Hello ".to_string(),
                }),
                Ok(StreamChunk::Text {
                    content: "World!".to_string(),
                }),
                Ok(StreamChunk::Done),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    let call_count = Arc::new(AtomicUsize::new(0));
    let provider = Arc::new(StreamingProvider {
        call_count: call_count.clone(),
    });
    let config = Arc::new(tokio::sync::Mutex::new(Config::default()));
    let orchestrator = AgentOrchestrator::new(provider, Arc::new(ToolRegistry::new()), config);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut history = vec![Message::user("hi")];
    orchestrator
        .run(&mut history, "mock", Some(tx), None)
        .await
        .unwrap();

    let mut text_parts = Vec::new();
    while let Some(chunk) = rx.recv().await {
        match chunk {
            StreamChunk::Text { content } => text_parts.push(content),
            StreamChunk::Done => break,
            _ => {}
        }
    }
    assert_eq!(text_parts.concat(), "Hello World!");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].role, Role::Assistant);
    assert_eq!(history[1].content.as_deref(), Some("Hello World!"));
}

#[tokio::test]
async fn test_orchestrator_handles_mid_stream_error_chunk() {
    // Provider emits a couple of Text chunks, then a `StreamChunk::Error`
    // mid-stream (e.g. a rate-limit notice or upstream 5xx), then Done.
    // This is the case where the retry wrapper would normally catch and
    // retry, but to exercise the orchestrator's defensive handler we run
    // with `RetryPolicy::Disabled` so the stream is passed through
    // untouched.
    struct MidStreamErrorProvider;
    #[async_trait]
    impl AIProvider for MidStreamErrorProvider {
        fn name(&self) -> &str {
            "MidStreamError"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec!["mock".into()])
        }
        async fn ask(
            &self,
            _msgs: Arc<Vec<Message>>,
            _model: &str,
            _tools: Arc<Option<Vec<serde_json::Value>>>,
            _thinking_level: Option<&str>,
        ) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
            let chunks = vec![
                Ok(StreamChunk::Text {
                    content: "partial".to_string(),
                }),
                Ok(StreamChunk::Error {
                    content: "rate-limited".to_string(),
                }),
                Ok(StreamChunk::Done),
            ];
            Ok(Box::pin(stream::iter(chunks)))
        }
    }

    let provider: Arc<dyn AIProvider> = Arc::new(MidStreamErrorProvider);
    let mut config = Config::default();
    config.retry_policy = routecode_sdk::core::config::RetryPolicy::Disabled;
    let config = Arc::new(tokio::sync::Mutex::new(config));
    let orchestrator = AgentOrchestrator::new(provider, Arc::new(ToolRegistry::new()), config);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut history = vec![Message::user("hi")];

    // The orchestrator must return Err (the StreamChunk::Error aborts the
    // stream consumer). The wrapper's `run()` will then send a
    // StreamChunk::Error and StreamChunk::Done to the UI so the consumer
    // can finalize cleanly without hanging.
    let result = orchestrator.run(&mut history, "mock", Some(tx), None).await;
    assert!(
        result.is_err(),
        "orchestrator should propagate mid-stream error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("rate-limited"),
        "error should carry provider content: {}",
        err_msg
    );

    // Drain the UI channel and assert the cleanup shape.
    let mut got_error = false;
    let mut got_done = false;
    let mut text_parts = Vec::new();
    while let Some(chunk) = rx.recv().await {
        match chunk {
            StreamChunk::Text { content } => text_parts.push(content),
            StreamChunk::Error { content } => {
                got_error = true;
                assert!(
                    content.contains("rate-limited"),
                    "UI error content: {}",
                    content
                );
            }
            StreamChunk::Done => {
                got_done = true;
            }
            _ => {}
        }
    }
    assert!(
        got_error,
        "UI should receive StreamChunk::Error after abort"
    );
    assert!(got_done, "UI should receive StreamChunk::Done after abort");
    // The Text("partial") that arrived before the error may or may not be
    // flushed depending on buffering; the contract is just that the
    // orchestrator does not hang.
    let _ = text_parts; // may be empty or ["partial"]
}

#[tokio::test]
async fn test_message_serialization_roundtrip() {
    let msgs = vec![
        Message::system("system prompt"),
        Message::user("user message"),
        Message::assistant(
            Some(std::sync::Arc::from("assistant reply")),
            Some(std::sync::Arc::from("thinking...")),
            Some(vec![ToolCall {
                index: Some(0),
                id: "call_1".into(),
                r#type: "function".into(),
                function: FunctionCall {
                    name: "file_read".into(),
                    arguments: r#"{"path":"."}"#.into(),
                },
            }]),
        ),
        Message::tool("call_1".into(), "file_read".into(), "file content"),
    ];

    let json = serde_json::to_string_pretty(&msgs).unwrap();
    let deserialized: Vec<Message> = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.len(), 4);
    assert_eq!(deserialized[0].role, Role::System);
    assert_eq!(deserialized[1].role, Role::User);
    assert_eq!(deserialized[2].role, Role::Assistant);
    assert_eq!(deserialized[3].role, Role::Tool);
    assert_eq!(deserialized[2].thought.as_deref(), Some("thinking..."));
    assert_eq!(deserialized[2].tool_calls.as_ref().unwrap().len(), 1);
    assert_eq!(deserialized[3].tool_call_id.as_deref(), Some("call_1"));
}

#[tokio::test]
async fn test_config_serialization_roundtrip() {
    let mut config = Config::default();
    config.model = "gpt-4o-mini".into();
    config.provider = "openai".into();
    config.thinking_level = "deep".into();
    config.favorites.push(DynamicModelInfo {
        name: "claude-3-5-sonnet".into(),
        provider_id: "anthropic".into(),
    });

    let json = serde_json::to_string_pretty(&config).unwrap();
    let deserialized: Config = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.model, "gpt-4o-mini");
    assert_eq!(deserialized.provider, "openai");
    assert_eq!(deserialized.thinking_level, "deep");
    assert_eq!(deserialized.favorites.len(), 1);
    assert_eq!(deserialized.favorites[0].name, "claude-3-5-sonnet");
}

#[tokio::test]
async fn test_config_default_values() {
    let config = Config::default();
    assert_eq!(config.model, "gpt-4o");
    assert_eq!(config.provider, "openai");
    assert_eq!(config.thinking_level, "default");
    assert_eq!(config.logo_animation, "always");
    assert!(config.api_keys.is_empty());
    assert!(config.favorites.is_empty());
}

#[tokio::test]
async fn test_cost_calculation() {
    let cost = calculate_cost(1_000_000, 500_000, "gpt-4o").await;
    assert!((cost - 12.5).abs() < 0.01);

    let cost = calculate_cost(1_000_000, 500_000, "gpt-4o-mini").await;
    assert!((cost - 0.45).abs() < 0.01);

    let cost = calculate_cost(1_000_000, 500_000, "deepseek-chat").await;
    assert!((cost - 0.28).abs() < 0.01);

    let cost = calculate_cost(0, 0, "gpt-4o").await;
    assert!(cost.abs() < 0.001);
}

#[tokio::test]
async fn test_usage_tracking() {
    let mut usage = Usage::default();
    assert_eq!(usage.total_tokens, 0);
    assert_eq!(usage.total_cost, 0.0);

    usage.add(1000, 500, "gpt-4o").await;
    assert_eq!(usage.input_tokens, 1000);
    assert_eq!(usage.output_tokens, 500);
    assert_eq!(usage.total_tokens, 1500);
    assert!(usage.total_cost > 0.0);

    usage.add(0, 0, "gpt-4o").await;
    assert_eq!(usage.total_tokens, 1500);
}

#[tokio::test]
async fn test_sanitize_session_name() {
    assert_eq!(sanitize_session_name("hello-world"), "hello-world");
    assert_eq!(sanitize_session_name("hello world"), "helloworld");
    assert_eq!(sanitize_session_name("foo/bar\\baz"), "foobarbaz");
    assert_eq!(sanitize_session_name(""), "");
}

#[tokio::test]
async fn test_is_path_outside_workspace() {
    let result = is_path_outside_workspace("/workspace/foo.rs");
    assert!(!result);

    let result = is_path_outside_workspace("/workspace");
    assert!(!result);
}

#[tokio::test]
async fn test_find_project_root() {
    let root = find_project_root();
    assert!(root.exists());
    assert!(root.join("Cargo.toml").exists() || root.join(".git").exists());
}

#[tokio::test]
async fn test_save_and_load_config() {
    let mut config = Config::default();
    config.model = "test-model".into();
    config.provider = "test-provider".into();
    let result = save_config(&config);
    assert!(result.is_ok());

    let loaded = load_config().unwrap();
    assert_eq!(loaded.model, "test-model");
    assert_eq!(loaded.provider, "test-provider");

    save_config(&Config::default()).unwrap();
}

#[tokio::test]
async fn test_resolve_provider() {
    let provider = resolve_provider("openai", "sk-test-key".into());
    assert_eq!(provider.name(), "OpenAI");

    let provider = resolve_provider("anthropic", "sk-test-key".into());
    assert_eq!(provider.name(), "Anthropic");

    let provider = resolve_provider("openrouter", "sk-test-key".into());
    assert_eq!(provider.name(), "OpenRouter");

    let provider = resolve_provider("deepseek", "sk-test-key".into());
    assert_eq!(provider.name(), "DeepSeek");

    let provider = resolve_provider("nvidia", "sk-test-key".into());
    assert_eq!(provider.name(), "NVIDIA");

    let provider = resolve_provider("gemini", "sk-test-key".into());
    assert_eq!(provider.name(), "Google Gemini");
}

#[tokio::test]
async fn test_tool_execution_order_within_registry() {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(LsTool));
    registry.register(Arc::new(GrepTool));

    let ls_schema = registry.get("ls").unwrap().to_json_schema();
    let grep_schema = registry.get("grep").unwrap().to_json_schema();
    assert_eq!(ls_schema["function"]["name"], "ls");
    assert_eq!(grep_schema["function"]["name"], "grep");
}

#[tokio::test]
async fn test_tool_result_serialization() {
    let result = ToolResult::success("all good");
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("all good"));
    assert!(json.contains("true"));

    let result = ToolResult::error("something broke");
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("something broke"));
    assert!(json.contains("false"));

    let result = ToolResult::success("with diff").with_diff("--- a\n+++ b\n".into());
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("with diff"));
    assert!(json.contains("--- a"));
}

#[tokio::test]
async fn test_tool_call_serialization() {
    let tc = ToolCall {
        index: Some(0),
        id: "call_abc123".into(),
        r#type: "function".into(),
        function: FunctionCall {
            name: "bash".into(),
            arguments: r#"{"command":"ls -la"}"#.into(),
        },
    };
    let json = serde_json::to_string_pretty(&tc).unwrap();
    let deserialized: ToolCall = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, "call_abc123");
    assert_eq!(deserialized.function.name, "bash");
    assert_eq!(deserialized.index, Some(0));
}
