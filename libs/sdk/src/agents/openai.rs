use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::utils::parse_sse_buffer;
use crate::core::{Message, ToolCall};
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

pub struct OpenAIProvider {
    api_key: String,
    base_url: String,
    provider_name: String,
    client: Client,
}

impl OpenAIProvider {
    pub fn new(api_key: String, base_url: String, provider_name: String) -> Self {
        Self {
            api_key,
            base_url,
            provider_name,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        let url = if self.base_url.ends_with('/') {
            format!("{}models", self.base_url)
        } else {
            format!("{}/models", self.base_url)
        };

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("{} list_models error: {}", self.provider_name, err_text));
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

        let url = if self.base_url.ends_with('/') {
            format!("{}chat/completions", self.base_url)
        } else {
            format!("{}/chat/completions", self.base_url)
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!(
                "{} error: {}",
                self.provider_name,
                err_text
            ));
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
