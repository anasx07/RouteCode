use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::core::{Message, Role};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};

pub struct GeminiProvider {
    api_key: String,
    client: Client,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Google Gemini"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        Ok(vec![
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
            "gemini-1.0-pro".to_string(),
        ])
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        _tools: Option<Vec<Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        let mut contents = Vec::new();
        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "model",
                _ => "user", // Default to user for others
            };
            contents.push(json!({
                "role": role,
                "parts": [{"text": msg.content.unwrap_or_default()}]
            }));
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            model, self.api_key
        );

        let body = json!({
            "contents": contents,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("Gemini error: {}", err_text));
        }

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        // Gemini returns a JSON array of objects over the stream, but not standard SSE
                        // It's actually a bit tricky to parse manually without a proper stream decoder if it's large
                        // For now, let's assume it's small enough or comes in chunks of JSON objects
                        if let Ok(val) = serde_json::from_str::<Value>(&buffer) {
                             if let Some(candidates) = val[0]["candidates"].as_array() {
                                if let Some(text) = candidates[0]["content"]["parts"][0]["text"].as_str() {
                                    yield Ok(StreamChunk::Text { content: text.to_string() });
                                }
                             }
                             buffer.clear();
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
