use crate::agents::traits::{AIProvider, StreamResponse};
use crate::agents::types::StreamChunk;
use crate::agents::utils::parse_sse_buffer;
use crate::core::Message;
use async_stream::stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct CloudflareWorkersAI {
    account_id: String,
    api_token: String,
    client: Client,
}

impl CloudflareWorkersAI {
    pub fn new(account_id: String, api_token: String) -> Self {
        Self {
            account_id,
            api_token,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for CloudflareWorkersAI {
    fn name(&self) -> &str {
        "Cloudflare Workers AI"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/models/search?type=Text Generation",
            self.account_id
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?;

        if !response.status().is_success() {
            // Fallback to hardcoded list if API fails or permissions are missing for search
            return Ok(vec![
                "@cf/meta/llama-3-8b-instruct".to_string(),
                "@cf/meta/llama-3-70b-instruct".to_string(),
                "@cf/mistral/mistral-7b-instruct-v0.1".to_string(),
                "@cf/qwen/qwen1.5-7b-chat-awq".to_string(),
            ]);
        }

        let val: Value = match response.json().await {
            Ok(v) => v,
            Err(_) => return Ok(vec![
                "@cf/meta/llama-3-8b-instruct".to_string(),
                "@cf/meta/llama-3-70b-instruct".to_string(),
                "@cf/mistral/mistral-7b-instruct-v0.1".to_string(),
                "@cf/qwen/qwen1.5-7b-chat-awq".to_string(),
            ]),
        };

        if let Some(result) = val["result"].as_array() {
            let models: Vec<String> = result
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect();
            if models.is_empty() {
                 Ok(vec![
                    "@cf/meta/llama-3-8b-instruct".to_string(),
                    "@cf/meta/llama-3-70b-instruct".to_string(),
                    "@cf/mistral/mistral-7b-instruct-v0.1".to_string(),
                    "@cf/qwen/qwen1.5-7b-chat-awq".to_string(),
                ])
            } else {
                Ok(models)
            }
        } else {
            Ok(vec![
                "@cf/meta/llama-3-8b-instruct".to_string(),
                "@cf/meta/llama-3-70b-instruct".to_string(),
                "@cf/mistral/mistral-7b-instruct-v0.1".to_string(),
                "@cf/qwen/qwen1.5-7b-chat-awq".to_string(),
            ])
        }
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        _tools: Option<Vec<Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        // Workers AI has an OpenAI-compatible endpoint now, which is easier to use.
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/v1/chat/completions",
            self.account_id
        );

        let body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("Cloudflare Workers AI error: {}", err_text));
        }

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut active_tool_calls = HashMap::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, &String::from_utf8_lossy(&bytes));
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

pub struct CloudflareAIGateway {
    account_id: String,
    gateway_id: String,
    api_token: String,
    client: Client,
}

impl CloudflareAIGateway {
    pub fn new(account_id: String, gateway_id: String, api_token: String) -> Self {
        Self {
            account_id,
            gateway_id,
            api_token,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl AIProvider for CloudflareAIGateway {
    fn name(&self) -> &str {
        "Cloudflare AI Gateway"
    }

    async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
        Ok(vec!["unified/chat".to_string()])
    }

    async fn ask(
        &self,
        messages: Vec<Message>,
        model: &str,
        tools: Option<Vec<Value>>,
    ) -> Result<StreamResponse, anyhow::Error> {
        // AI Gateway works as a proxy.
        // If the model is in "provider/model" format, we use the /compat endpoint.
        let url = format!(
            "https://gateway.ai.cloudflare.com/v1/{}/{}/compat/chat/completions",
            self.account_id, self.gateway_id
        );

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
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await?;
            return Err(anyhow::anyhow!("Cloudflare AI Gateway error: {}", err_text));
        }

        let mut bytes_stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut active_tool_calls = HashMap::new();

        let s = stream! {
            while let Some(item) = bytes_stream.next().await {
                match item {
                    Ok(bytes) => {
                        let chunks = parse_sse_buffer(&mut buffer, &mut active_tool_calls, &String::from_utf8_lossy(&bytes));
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
