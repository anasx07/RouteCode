use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::utils::{parse_sse_buffer, parse_anthropic_sse};
use crate::core::{Message, ToolCall, Role};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub struct OpenCodeProvider {
    api_key: String,
    base_url: String,
    is_zen: bool,
    provider_name: String,
    client: Client,
}

impl OpenCodeProvider {
    pub fn new(api_key: String, base_url: String, provider_name: String, is_zen: bool) -> Self {
        Self {
            api_key,
            base_url,
            is_zen,
            provider_name,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    fn get_prefixed_model(&self, model: &str) -> String {
        // Based on documentation and error reports, the OpenCode API 
        // expects the raw model ID (e.g. "gpt-5.5") because the 
        // provider (Zen/Go) is already determined by the base URL.
        model.to_string()
    }
}

#[async_trait]
impl AIProvider for OpenCodeProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        let url = format!("{}/models", self.base_url);

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        if let Ok(resp) = response {
            if resp.status().is_success() {
                if let Ok(val) = resp.json::<Value>().await {
                    let mut models = Vec::new();
                    if let Some(data) = val["data"].as_array() {
                        for model in data {
                            if let Some(id) = model["id"].as_str() {
                                let clean_id = id.strip_prefix("opencode-zen/").or_else(|| id.strip_prefix("opencode-go/")).unwrap_or(id);
                                models.push(clean_id.to_string());
                            }
                        }
                    }
                    if !models.is_empty() {
                        return Ok(models);
                    }
                }
            }
        }

        // Fallback based on documentation screenshots
        if self.is_zen {
            Ok(vec![
                "gpt-5.5".to_string(),
                "gpt-5.5-pro".to_string(),
                "gpt-5.4".to_string(),
                "gpt-5.4-pro".to_string(),
                "gpt-5.4-mini".to_string(),
                "gpt-5.4-nano".to_string(),
                "gpt-5.3-codex".to_string(),
                "gpt-5.3-codex-spark".to_string(),
                "gpt-5.2".to_string(),
                "gpt-5.2-codex".to_string(),
                "gpt-5.1".to_string(),
                "gpt-5.1-codex".to_string(),
                "gpt-5.1-codex-max".to_string(),
                "gpt-5.1-codex-mini".to_string(),
                "gpt-5".to_string(),
                "gpt-5-codex".to_string(),
                "gpt-5-nano".to_string(),
                "claude-opus-4-7".to_string(),
                "claude-opus-4-6".to_string(),
                "claude-opus-4-5".to_string(),
                "claude-opus-4-1".to_string(),
                "claude-sonnet-4-6".to_string(),
                "claude-sonnet-4-5".to_string(),
                "claude-sonnet-4".to_string(),
                "claude-haiku-4-5".to_string(),
                "claude-3-5-haiku".to_string(),
                "gemini-3.1-pro".to_string(),
                "gemini-3-flash".to_string(),
                "qwen3.6-plus".to_string(),
                "qwen3.6-plus-free".to_string(),
                "qwen3.5-plus".to_string(),
                "minimax-m2.7".to_string(),
                "minimax-m2.5".to_string(),
                "minimax-m2.5-free".to_string(),
                "glm-5.1".to_string(),
                "glm-5".to_string(),
                "kimi-k2.6".to_string(),
                "kimi-k2.5".to_string(),
                "big-pickle".to_string(),
                "deepseek-v4-flash-free".to_string(),
            ])
        } else {
            Ok(vec![
                "glm-5".to_string(),
                "glm-5.1".to_string(),
                "kimi-k2.5".to_string(),
                "kimi-k2.6".to_string(),
                "mimo-v2.5".to_string(),
                "mimo-v2.5-pro".to_string(),
                "minimax-m2.5".to_string(),
                "minimax-m2.7".to_string(),
                "qwen3.5-plus".to_string(),
                "qwen3.6-plus".to_string(),
                "deepseek-v4-pro".to_string(),
                "deepseek-v4-flash".to_string(),
            ])
        }
    }

    async fn ask(
        &self,
        messages: Arc<Vec<Message>>,
        model: &str,
        _tools: Arc<Option<Vec<Value>>>,
        thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let prefixed_model = self.get_prefixed_model(model);
        let model_lower = model.to_lowercase();
        
        // Routing logic based on documentation screenshots
        let endpoint = if model_lower.starts_with("claude") {
            format!("{}/messages", self.base_url)
        } else if model_lower.starts_with("gpt") {
            format!("{}/responses", self.base_url)
        } else if model_lower.starts_with("gemini") {
            // Google style endpoint - append streaming suffix
            format!("{}/models/{}:streamGenerateContent", self.base_url, prefixed_model)
        } else if !self.is_zen && model_lower.contains("minimax") {
            // MiniMax in Go uses /messages
            format!("{}/messages", self.base_url)
        } else {
            // Default
            format!("{}/chat/completions", self.base_url)
        };

        // Determine format: Anthropic vs OpenAI vs Gemini
        if endpoint.ends_with("/messages") {
            // Anthropic Format
            let mut anthropic_messages = Vec::new();
            let mut global_system = String::new();
            for msg in messages.iter() {
                match msg.role {
                    Role::System => {
                        if let Some(c) = &msg.content { global_system.push_str(c); }
                    }
                    Role::User => {
                        anthropic_messages.push(json!({ "role": "user", "content": msg.content.as_deref().unwrap_or_default() }));
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
                        anthropic_messages.push(json!({ "role": "assistant", "content": content }));
                    }
                    Role::Tool => {
                        // In Anthropic format, tool results are sent as 'user' role with 'tool_result' content
                        anthropic_messages.push(json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": msg.tool_call_id.as_deref().unwrap_or_default(),
                                "content": msg.content.as_deref().unwrap_or_default(),
                            }]
                        }));
                    }
                }
            }
            let mut body = json!({ "model": prefixed_model, "messages": anthropic_messages, "stream": true, "max_tokens": 16384 });
            if !global_system.is_empty() { body["system"] = json!(global_system); }
            
            if let Some(level) = thinking_level {
                if level != "default" { body["thinking_level"] = json!(level); }
            }

            if let Some(t) = _tools.as_ref() {
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

            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
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
                yield Ok(crate::agents::types::StreamChunk::Done);
            };
            Ok(Box::pin(s))
        } else if endpoint.contains(":streamGenerateContent") {
            // Gemini/Google Format
            let mut contents = Vec::new();
            for msg in messages.iter() {
                let role = match msg.role { Role::User => "user", Role::Assistant => "model", _ => "user" };
                contents.push(json!({ "role": role, "parts": [{"text": msg.content.as_deref().unwrap_or_default()}] }));
            }
            let mut body = json!({ "contents": contents });
            
            if let Some(level) = thinking_level {
                if level != "default" { body["thinking_level"] = json!(level); }
            }

            if let Some(t) = _tools.as_ref() {
                let mut gemini_tools = Vec::new();
                for tool in t {
                    if let Some(f) = tool.get("function") {
                        gemini_tools.push(json!({
                            "name": f["name"],
                            "description": f["description"],
                            "parameters": f["parameters"],
                        }));
                    }
                }
                body["tools"] = json!([{ "function_declarations": gemini_tools }]);
            }

            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
            let response = crate::utils::error::check_status(response).await?;

            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let s = stream! {
                while let Some(item) = bytes_stream.next().await {
                    match item {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                            // Google stream is often a JSON array or multiple objects
                            if let Ok(val) = serde_json::from_str::<Value>(&buffer) {
                                if let Some(candidates) = val[0]["candidates"].as_array() {
                                    if let Some(text) = candidates[0]["content"]["parts"][0]["text"].as_str() { yield Ok(crate::agents::types::StreamChunk::Text { content: text.to_string() }); }
                                } else if let Some(candidates) = val["candidates"].as_array() {
                                    // Single object format
                                    if let Some(text) = candidates[0]["content"]["parts"][0]["text"].as_str() { yield Ok(crate::agents::types::StreamChunk::Text { content: text.to_string() }); }
                                }
                                buffer.clear();
                            }
                        }
                        Err(e) => yield Err(anyhow::Error::from(e)),
                    }
                }
                yield Ok(crate::agents::types::StreamChunk::Done);
            };
            Ok(Box::pin(s))
        } else {
            // OpenAI Format (Default + GPT /responses)
            let mut body = json!({ "model": prefixed_model, "messages": &*messages, "stream": true, "max_tokens": 16384 });
            if let Some(t) = _tools.as_ref() { body["tools"] = json!(t); }
            
            if let Some(level) = thinking_level {
                if level != "default" { body["thinking_level"] = json!(level); }
            }

            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
            let response = crate::utils::error::check_status(response).await?;

            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut active_tool_calls: HashMap<usize, ToolCall> = HashMap::new();
            let s = stream! {
                while let Some(item) = bytes_stream.next().await {
                    match item {
                        Ok(bytes) => {
                            let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, &String::from_utf8_lossy(&bytes));
                            for chunk in chunks { yield Ok(chunk); }
                        }
                        Err(e) => yield Err(anyhow::Error::from(e)),
                    }
                }
                yield Ok(crate::agents::types::StreamChunk::Done);
            };
            Ok(Box::pin(s))
        }
    }
}
