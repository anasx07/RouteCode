use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::agents::utils::parse_sse_buffer;
use crate::core::{Message, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

pub struct OpenRouter {
    api_key: String,
    client: Client,
}

impl OpenRouter {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
        }
    }
}

#[async_trait]
impl AIProvider for OpenRouter {
    fn name(&self) -> &str {
        "OpenRouter"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        let response = self
            .client
            .get("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        let response = crate::utils::error::check_status(response).await?;

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
        messages: Arc<Vec<Message>>,
        model: &str,
        tools: Arc<Option<Vec<serde_json::Value>>>,
        thinking_level: Option<&str>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let mut body = json!({
            "model": model,
            "messages": &*messages,
            "stream": true,
        });

        if let Some(t) = tools.as_ref() {
            body["tools"] = json!(t);
        }

        if let Some(level) = thinking_level {
            if level != "default" {
                // OpenRouter often maps this to specific parameters or suffixes
                // For now, we'll try passing it as a provider-specific field
                body["provider"] = json!({ "thinking_level": level });
            }
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

        let response = crate::utils::error::check_status(response).await?;

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
                    yield Ok(StreamChunk::Done);
                    };
        Ok(Box::pin(s))
    }
}
