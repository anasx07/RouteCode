use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::core::{FunctionCall, Message, Role, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct VertexAIProvider {
    api_key: String,
    project: String,
    location: String,
    client: Client,
}

impl VertexAIProvider {
    pub fn new(api_key: String, project: String, location: String) -> Self {
        Self {
            api_key,
            project,
            location: if location.is_empty() { "us-central1".to_string() } else { location },
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }

    fn endpoint(&self, model: &str) -> String {
        format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:streamGenerateContent",
            self.location, self.project, self.location, model
        )
    }

    fn is_api_key(&self) -> bool {
        self.api_key.starts_with("AIza") || !self.api_key.contains('.')
    }
}

#[async_trait]
impl AIProvider for VertexAIProvider {
    fn name(&self) -> &str {
        "Google Vertex AI"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        Ok(vec![
            "gemini-2.0-flash-001".to_string(),
            "gemini-2.0-flash-exp".to_string(),
            "gemini-1.5-pro-002".to_string(),
            "gemini-1.5-pro-001".to_string(),
            "gemini-1.5-flash-002".to_string(),
            "gemini-1.5-flash-001".to_string(),
        ])
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        _tools: Option<Vec<Value>>,
        _thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let url = self.endpoint(model);
        let url_with_auth = if self.is_api_key() {
            format!("{}?key={}", url, self.api_key)
        } else {
            url
        };

        let mut contents = Vec::new();
        let mut system_instruction = String::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    if let Some(c) = &msg.content {
                        system_instruction.push_str(c);
                    }
                }
                Role::User => {
                    contents.push(json!({
                        "role": "user",
                        "parts": [{ "text": msg.content.unwrap_or_default() }]
                    }));
                }
                Role::Assistant => {
                    let mut parts = Vec::new();
                    if let Some(c) = &msg.content {
                        parts.push(json!({ "text": c }));
                    }
                    if let Some(calls) = &msg.tool_calls {
                        for tc in calls {
                            let args: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
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
                                "response": { "result": msg.content.unwrap_or_default() }
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

        let mut request = self.client.post(&url_with_auth).json(&body);
        if !self.is_api_key() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("Vertex AI error: {}", err_text));
        }

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

                            let mut json_stream = serde_json::Deserializer::from_str(&buffer).into_iter::<Value>();
                            match json_stream.next() {
                                Some(Ok(val)) => {
                                    let offset = json_stream.byte_offset();
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
                                    if offset > 0 {
                                        buffer.drain(..offset);
                                    } else {
                                        break;
                                    }
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
