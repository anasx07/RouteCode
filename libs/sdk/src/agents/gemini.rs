use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::core::{FunctionCall, Message, Role, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub struct GeminiProvider {
    api_key: String,
    client: Client,
    base_url: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
        }
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Google Gemini"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        if let Some(models) = crate::utils::models::get_models_for_provider("google") {
            return Ok(models);
        }

        let url = format!(
            "{}/models?key={}",
            self.base_url.trim_end_matches('/'),
            self.api_key
        );

        let response = self.client.get(&url).send().await?;

        let response = crate::utils::error::check_status(response).await?;

        let val: Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(models_arr) = val["models"].as_array() {
            for m in models_arr {
                if let Some(name) = m["name"].as_str() {
                    let id = name.strip_prefix("models/").unwrap_or(name);
                    models.push(id.to_string());
                }
            }
        }

        Ok(models)
    }

    async fn ask(
        &self,
        messages: Arc<Vec<Message>>,
        model: &str,
        _tools: Arc<Option<Vec<Value>>>,
        _thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}",
            self.base_url.trim_end_matches('/'),
            model,
            self.api_key
        );

        let mut contents = Vec::new();
        let mut system_instruction = String::new();

        for msg in messages.iter() {
            match msg.role {
                Role::System => {
                    if let Some(c) = &msg.content {
                        system_instruction.push_str(c);
                    }
                }
                Role::User => {
                    contents.push(json!({
                        "role": "user",
                        "parts": [{ "text": msg.content.as_deref().unwrap_or_default() }]
                    }));
                }
                Role::Assistant => {
                    let mut parts = Vec::new();
                    if let Some(c) = &msg.content {
                        parts.push(json!({ "text": c }));
                    }
                    if let Some(calls) = &msg.tool_calls {
                        for tc in calls {
                            let args: Value =
                                serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                            parts.push(json!({
                                "functionCall": {
                                    "name": tc.function.name,
                                    "args": args
                                }
                            }));
                        }
                    }
                    if !parts.is_empty() {
                        contents.push(json!({
                            "role": "model",
                            "parts": parts
                        }));
                    }
                }
                Role::Tool => {
                    let fn_name = msg.name.clone().unwrap_or_else(|| "tool".to_string());
                    contents.push(json!({
                        "role": "function",
                        "parts": [{
                            "functionResponse": {
                                "name": fn_name,
                                "response": { "result": msg.content.as_deref().unwrap_or_default() }
                            }
                        }]
                    }));
                }
            }
        }

        let mut body = json!({
            "contents": contents,
        });

        if !system_instruction.is_empty() {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system_instruction }]
            });
        }

        if let Some(level) = _thinking_level {
            if level != "default" {
                body["thinking_level"] = json!(level);
            }
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
            if !gemini_tools.is_empty() {
                body["tools"] = json!([{ "function_declarations": gemini_tools }]);
            }
        }

        let response = self.client.post(&url).json(&body).send().await?;

        let response = crate::utils::error::check_status(response).await?;

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        loop {
                            let trimmed = buffer.trim_start_matches(|c: char| c == '[' || c == ']' || c == ',' || c.is_whitespace());
                            if trimmed.len() < buffer.len() {
                                let cut = buffer.len() - trimmed.len();
                                buffer.drain(..cut);
                            }

                            if buffer.is_empty() {
                                break;
                            }

                            let mut stream = serde_json::Deserializer::from_str(&buffer).into_iter::<Value>();
                            match stream.next() {
                                Some(Ok(val)) => {
                                    let offset = stream.byte_offset();
                                    if let Some(candidates) = val["candidates"].as_array() {
                                        if let Some(content) = candidates[0].get("content") {
                                            if let Some(parts) = content["parts"].as_array() {
                                                for part in parts {
                                                    if let Some(text) = part["text"].as_str() {
                                                        yield Ok(StreamChunk::Text { content: text.to_string() });
                                                    }
                                                    if let Some(fn_call) = part.get("functionCall") {
                                                        let name = fn_call["name"].as_str().unwrap_or("").to_string();
                                                        let args = fn_call["args"].clone();
                                                        let arguments = serde_json::to_string(&args).unwrap_or_default();
                                                        let tool_call = ToolCall {
                                                            id: format!("call_{}", Uuid::new_v4().simple()),
                                                            r#type: "function".to_string(),
                                                            index: Some(0),
                                                            function: FunctionCall {
                                                                name,
                                                                arguments,
                                                            },
                                                        };
                                                        yield Ok(StreamChunk::ToolCall { tool_call });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    buffer.drain(..offset);
                                }
                                _ => {
                                    break;
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
