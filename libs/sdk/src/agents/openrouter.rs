use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::utils::parse_sse_buffer;
use crate::core::{Message, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

pub struct OpenRouter {
    api_key: String,
    client: Client,
}

impl OpenRouter {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for OpenRouter {
    fn name(&self) -> &str {
        "OpenRouter"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        let response = self.client
            .get("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenRouter list_models error: {}", err_text));
        }

        let val: serde_json::Value = response.json().await?;
        let mut models = Vec::new();

        if let Some(data) = val["data"].as_array() {
            for model in data {
                if let Some(id) = model["id"].as_str() {
                    models.push(id.to_string());
                }
            }
        }

        Ok(models)
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        tools: Option<Vec<serde_json::Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let mut body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        if let Some(t) = tools {
            body["tools"] = json!(t);
        }

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/anasx07/routecode")
            .header("X-Title", "RouteCode")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("OpenRouter error: {}", err_text));
        }

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut active_tool_calls: HashMap<usize, ToolCall> = HashMap::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, &String::from_utf8_lossy(&bytes));
                        for chunk in chunks {
                            yield Ok(chunk);
                        }
                    }
                    Err(e) => {
                        yield Err(anyhow::Error::from(e));
                    }
                }
            }
        };

        Ok(Box::pin(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::StreamChunk;

    #[test]
    fn test_parse_sse_buffer_text() {
        let mut buffer = String::new();
        let mut active_tool_calls = HashMap::new();

        // Partial data
        let data1 = "data: {\"choices\": [{\"delta\": {\"content\": \"Hello\"}}]}\n";
        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, data1);
        assert_eq!(chunks.len(), 1);
        if let StreamChunk::Text { content } = &chunks[0] {
            assert_eq!(content, "Hello");
        }

        // Split data across chunks
        let data2 = "data: {\"choices\": [{\"delta\": {\"content\": \" world\"}}]}"; // No newline
        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, data2);
        assert_eq!(chunks.len(), 0); // Should be buffered

        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, "\n");
        assert_eq!(chunks.len(), 1);
        if let StreamChunk::Text { content } = &chunks[0] {
            assert_eq!(content, " world");
        }
    }

    #[test]
    fn test_parse_sse_buffer_tool_calls() {
        let mut buffer = String::new();
        let mut active_tool_calls = HashMap::new();

        let data = "data: {\"choices\": [{\"delta\": {\"tool_calls\": [{\"index\": 0, \"id\": \"call_1\", \"function\": {\"name\": \"ls\"}}]}}]}\ndata: {\"choices\": [{\"delta\": {\"tool_calls\": [{\"index\": 0, \"function\": {\"arguments\": \"{\\\"path\\\": \\\".\\\"}\"}}]}}]}\n";

        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, data);
        assert_eq!(chunks.len(), 2);

        if let StreamChunk::ToolCall { tool_call } = &chunks[1] {
            assert_eq!(tool_call.id, "call_1");
            assert_eq!(tool_call.function.name, "ls");
            assert_eq!(tool_call.function.arguments, "{\"path\": \".\"}");
        }
    }
}
