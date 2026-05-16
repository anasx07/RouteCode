use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::{StreamChunk, Usage};
use crate::core::{Message, Role, ToolCall, FunctionCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct AnthropicProvider {
    api_key: String,
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "Anthropic"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        // Anthropic doesn't have a public models endpoint in the same way OpenAI does that's easily accessible without specific permissions
        // Returning a common set of models
        Ok(vec![
            "claude-3-5-sonnet-20240620".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ])
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        tools: Option<Vec<Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let mut anthropic_messages = Vec::new();
        let mut system_prompt = String::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    if let Some(content) = &msg.content {
                        system_prompt.push_str(content);
                    }
                }
                _ => {
                    let role_str = match msg.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        _ => "user",
                    };
                    anthropic_messages.push(json!({
                        "role": role_str,
                        "content": msg.content.unwrap_or_default(),
                    }));
                }
            }
        }

        let mut body = json!({
            "model": model,
            "messages": anthropic_messages,
            "stream": true,
            "max_tokens": 4096,
        });

        if !system_prompt.is_empty() {
            body["system"] = json!(system_prompt);
        }

        if let Some(t) = tools {
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

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("Anthropic error: {}", err_text));
        }

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut active_tool_calls: HashMap<String, ToolCall> = HashMap::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].to_string();
                            buffer.drain(..=line_end);
                            let line = line.trim();
                            if line.is_empty() { continue; }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(val) = serde_json::from_str::<Value>(data) {
                                    let event_type = val["type"].as_str().unwrap_or("");
                                    match event_type {
                                        "content_block_delta" => {
                                            if let Some(delta) = val.get("delta") {
                                                if let Some(text) = delta["text"].as_str() {
                                                    yield Ok(StreamChunk::Text { content: text.to_string() });
                                                }
                                                if let Some(_partial_json) = delta["partial_json"].as_str() {
                                                    // Handle partial tool call JSON
                                                    // In Anthropic, we get tool_use blocks
                                                }
                                            }
                                        }
                                        "content_block_start" => {
                                            if let Some(block) = val.get("content_block") {
                                                if block["type"] == "tool_use" {
                                                    let id = block["id"].as_str().unwrap_or("").to_string();
                                                    let name = block["name"].as_str().unwrap_or("").to_string();
                                                    active_tool_calls.insert(id.clone(), ToolCall {
                                                        id,
                                                        r#type: "function".to_string(),
                                                        index: None,
                                                        function: FunctionCall {
                                                            name,
                                                            arguments: String::new(),
                                                        },
                                                    });
                                                }
                                            }
                                        }
                                        "message_delta" => {
                                            if let Some(usage) = val.get("usage") {
                                                let prompt = usage["input_tokens"].as_u64().unwrap_or(0) as u32;
                                                let completion = usage["output_tokens"].as_u64().unwrap_or(0) as u32;
                                                yield Ok(StreamChunk::Usage {
                                                    usage: Usage {
                                                        prompt_tokens: prompt,
                                                        completion_tokens: completion,
                                                        total_tokens: prompt + completion,
                                                    }
                                                });
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
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
