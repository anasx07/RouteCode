use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::utils::parse_sse_buffer;
use crate::core::{Message, ToolCall, Role};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

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
            client: Client::new(),
        }
    }

    fn get_prefixed_model(&self, model: &str) -> String {
        let prefix = if self.is_zen { "opencode-zen/" } else { "opencode-go/" };
        if model.starts_with(prefix) {
            model.to_string()
        } else {
            format!("{}{}", prefix, model)
        }
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

        // Fallback
        if self.is_zen {
            Ok(vec![
                "gpt-5.1-codex".to_string(),
                "claude-opus-4-7".to_string(),
                "gemini-3.1-pro".to_string(),
                "big-pickle".to_string(),
                "deepseek-v4-flash-free".to_string(),
            ])
        } else {
            Ok(vec![
                "glm-5.1".to_string(),
                "kimi-k2.6".to_string(),
                "minimax-m2.7".to_string(),
                "qwen3.6-plus".to_string(),
            ])
        }
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        _tools: Option<Vec<Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let prefixed_model = self.get_prefixed_model(model);
        let model_lower = model.to_lowercase();
        
        // Routing logic based on documentation screenshots
        let endpoint = if model_lower.starts_with("claude") {
            format!("{}/messages", self.base_url)
        } else if model_lower.starts_with("gpt") {
            format!("{}/responses", self.base_url)
        } else if model_lower.starts_with("gemini") {
            // Google style endpoint
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
            let mut system_prompt = String::new();
            for msg in messages {
                match msg.role {
                    Role::System => if let Some(content) = &msg.content { system_prompt.push_str(content); }
                    _ => {
                        let role_str = match msg.role {
                            Role::User => "user",
                            Role::Assistant => "assistant",
                            _ => "user",
                        };
                        anthropic_messages.push(json!({ "role": role_str, "content": msg.content.unwrap_or_default() }));
                    }
                }
            }
            let mut body = json!({ "model": prefixed_model, "messages": anthropic_messages, "stream": true, "max_tokens": 4096 });
            if !system_prompt.is_empty() { body["system"] = json!(system_prompt); }

            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
            if !response.status().is_success() { return Err(anyhow::anyhow!("OpenCode error: {}", response.text().await?)); }

            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let s = stream! {
                while let Some(item) = bytes_stream.next().await {
                    match item {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                            while let Some(line_end) = buffer.find('\n') {
                                let line = buffer[..line_end].to_string();
                                buffer.drain(..=line_end);
                                if let Some(data) = line.trim().strip_prefix("data: ") {
                                    if let Ok(val) = serde_json::from_str::<Value>(data) {
                                        if val["type"] == "content_block_delta" {
                                            if let Some(text) = val["delta"]["text"].as_str() { yield Ok(crate::agents::types::StreamChunk::Text { content: text.to_string() }); }
                                        }
                                    }
                                }
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
            for msg in messages {
                let role = match msg.role { Role::User => "user", Role::Assistant => "model", _ => "user" };
                contents.push(json!({ "role": role, "parts": [{"text": msg.content.unwrap_or_default()}] }));
            }
            let body = json!({ "contents": contents });
            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
            if !response.status().is_success() { return Err(anyhow::anyhow!("OpenCode error: {}", response.text().await?)); }

            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let s = stream! {
                while let Some(item) = bytes_stream.next().await {
                    match item {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));
                            if let Ok(val) = serde_json::from_str::<Value>(&buffer) {
                                if let Some(candidates) = val[0]["candidates"].as_array() {
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
            let body = json!({ "model": prefixed_model, "messages": messages, "stream": true });
            let response = self.client.post(&endpoint).header("Authorization", format!("Bearer {}", self.api_key)).json(&body).send().await?;
            if !response.status().is_success() { return Err(anyhow::anyhow!("OpenCode error: {}", response.text().await?)); }

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
