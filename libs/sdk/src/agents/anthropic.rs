use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::agents::utils::parse_anthropic_sse;
use crate::core::{Message, Role, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct AnthropicProvider {
    api_key: String,
    client: Client,
    base_url: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string()),
        }
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        if let Some(models) = crate::utils::models::get_models_for_provider("anthropic") {
            return Ok(models);
        }
        // Anthropic doesn't have a public models endpoint in the same way OpenAI does that's easily accessible without specific permissions
        // Returning a common set of models
        Ok(vec![
            "claude-sonnet-4-5".to_string(),
            "claude-sonnet-4-20250514".to_string(),
            "claude-opus-4-20250514".to_string(),
            "claude-3-5-sonnet-20240620".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ])
    }

    async fn ask(
        &self,
        messages: Arc<Vec<Message>>,
        model: &str,
        tools: Arc<Option<Vec<Value>>>,
        _thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let mut anthropic_messages = Vec::new();
        let mut system_prompt = String::new();

        for msg in messages.iter() {
            match msg.role {
                Role::System => {
                    if let Some(content) = &msg.content {
                        system_prompt.push_str(content);
                    }
                }
                Role::User => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": msg.content.as_deref().unwrap_or_default(),
                    }));
                }
                Role::Assistant => {
                    let mut content = Vec::new();
                    if let Some(t) = &msg.thought {
                        content.push(json!({ "type": "thought", "thought": t }));
                    }
                    if let Some(c) = &msg.content {
                        content.push(json!({ "type": "text", "text": c }));
                    }
                    if let Some(calls) = &msg.tool_calls {
                        for tc in calls {
                            let input: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                            content.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.function.name,
                                "input": input,
                            }));
                        }
                    }
                    anthropic_messages.push(json!({
                        "role": "assistant",
                        "content": content,
                    }));
                }
                Role::Tool => {
                    anthropic_messages.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": msg.tool_call_id.as_deref().unwrap_or_default(),
                            "content": msg.content.as_deref().unwrap_or_default(),
                        }],
                    }));
                }
            }
        }

        let mut body = json!({
            "model": model,
            "messages": anthropic_messages,
            "stream": true,
            "max_tokens": 16384,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }

        if let Some(t) = tools.as_ref() {
            let mut anthropic_tools = Vec::new();
            for tool in t {
                if let Some(f) = tool.get("function") {
                    anthropic_tools.push(json!({
                        "name": f["name"],
                        "description": f["description"],
                        "input_schema": f["parameters"],
                    }));
                }
            }
            body["tools"] = json!(anthropic_tools);
        }

        let mut url = self.base_url.clone();
        if !url.ends_with("/v1/messages") {
            if url.ends_with("/v1/") || url.ends_with("/v1") {
                url = format!("{}/messages", url.trim_end_matches('/'));
            } else {
                url = format!("{}/v1/messages", url.trim_end_matches('/'));
            }
        }
        
        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let response = crate::utils::error::check_status(response).await?;

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut active_tool_calls: HashMap<usize, ToolCall> = HashMap::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        let chunks = parse_anthropic_sse(&mut buffer, &mut active_tool_calls, &String::from_utf8_lossy(&bytes));
                        for chunk in chunks {
                            yield Ok(chunk);
                        }
                    }
                    Err(e) => yield Err(anyhow::Error::from(e)),
                }
            }
            yield Ok(StreamChunk::Done);
        };

        Ok(Box::pin(s))
    }
}
