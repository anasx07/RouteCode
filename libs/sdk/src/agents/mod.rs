pub mod anthropic;
pub mod cloudflare;
pub mod gemini;
pub mod opencode;
pub mod openai;
pub mod openrouter;
pub mod traits;
pub mod types;
pub mod utils;

pub use anthropic::AnthropicProvider;
pub use cloudflare::{CloudflareAIGateway, CloudflareWorkersAI};
pub use gemini::GeminiProvider;
pub use opencode::OpenCodeProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouter;
pub use traits::AIProvider;
pub use types::{StreamChunk, Usage};

pub fn resolve_provider(provider_name: &str, api_key: String) -> std::sync::Arc<dyn AIProvider> {
    match provider_name.to_lowercase().as_str() {
        "openrouter" => std::sync::Arc::new(OpenRouter::new(api_key)),
        "anthropic" => std::sync::Arc::new(AnthropicProvider::new(api_key)),
        "google" | "gemini" => std::sync::Arc::new(GeminiProvider::new(api_key)),
        "cloudflare-workers" | "cloudflare_workers" => {
            let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default();
            // If api_key contains a colon, it might be account_id:api_token
            if api_key.contains(':') {
                let parts: Vec<&str> = api_key.split(':').collect();
                std::sync::Arc::new(CloudflareWorkersAI::new(parts[0].to_string(), parts[1].to_string()))
            } else {
                std::sync::Arc::new(CloudflareWorkersAI::new(account_id, api_key))
            }
        }
        "cloudflare-gateway" | "cloudflare_gateway" => {
            let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default();
            let gateway_id = std::env::var("CLOUDFLARE_GATEWAY_ID").unwrap_or_default();
            // If api_key contains colons, it might be account_id:gateway_id:api_token
            let parts: Vec<&str> = api_key.split(':').collect();
            if parts.len() == 3 {
                std::sync::Arc::new(CloudflareAIGateway::new(parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
            } else {
                std::sync::Arc::new(CloudflareAIGateway::new(account_id, gateway_id, api_key))
            }
        }
        "deepseek" => std::sync::Arc::new(OpenAIProvider::new(
            api_key,
            "https://api.deepseek.com/v1".to_string(),
            "DeepSeek".to_string(),
        )),
        "nvidia" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            "https://integrate.api.nvidia.com/v1".to_string(),
            "NVIDIA".to_string(),
        )),
        "opencode-zen" | "opencode_zen" => std::sync::Arc::new(OpenCodeProvider::new(
            api_key,
            "https://opencode.ai/zen/go/v1".to_string(),
            "OpenCode Zen".to_string(),
            true,
        )),
        "opencode-go" | "opencode_go" => std::sync::Arc::new(OpenCodeProvider::new(
            api_key,
            "https://opencode.ai/zen/go/v1".to_string(),
            "OpenCode Go".to_string(),
            false,
        )),
        "openai" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            "https://api.openai.com/v1".to_string(),
            "OpenAI".to_string(),
        )),
        _ => {
            if provider_name.starts_with("http") {
                std::sync::Arc::new(openai::OpenAIProvider::new(
                    api_key,
                    provider_name.to_string(),
                    provider_name.to_string(),
                ))
            } else {
                std::sync::Arc::new(OpenRouter::new(api_key))
            }
        }
    }
}
