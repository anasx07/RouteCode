pub mod anthropic;
pub mod cloudflare;
pub mod gemini;
pub mod opencode;
pub mod openai;
pub mod openrouter;
pub mod retry;
pub mod traits;
pub mod types;
pub mod utils;
pub mod vertex;

pub use anthropic::AnthropicProvider;
pub use cloudflare::{CloudflareAIGateway, CloudflareWorkersAI};
pub use gemini::GeminiProvider;
pub use opencode::OpenCodeProvider;
pub use openai::OpenAIProvider;
pub use openrouter::OpenRouter;
pub use retry::{run_retry_loop, RetryingProvider};
pub use traits::AIProvider;
pub use types::{StreamChunk, Usage};
pub use vertex::VertexAIProvider;

pub fn resolve_provider(provider_name: &str, api_key: String) -> std::sync::Arc<dyn AIProvider> {
    resolve_provider_with_config(provider_name, api_key, "", "")
}

pub fn resolve_provider_with_config(
    provider_name: &str,
    api_key: String,
    vertex_project: &str,
    vertex_location: &str,
) -> std::sync::Arc<dyn AIProvider> {
    let provider_info = crate::utils::models::get_provider_info(provider_name);
    let dynamic_api = provider_info.and_then(|p| p.api);

    match provider_name.to_lowercase().as_str() {
        "openrouter" => std::sync::Arc::new(OpenRouter::new(api_key)),
        "anthropic" => std::sync::Arc::new(AnthropicProvider::new(api_key, dynamic_api)),
        "google" | "gemini" => std::sync::Arc::new(GeminiProvider::new(api_key, dynamic_api)),
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
            dynamic_api.unwrap_or_else(|| "https://api.deepseek.com/v1".to_string()),
            "DeepSeek".to_string(),
        )),
        "nvidia" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            dynamic_api.unwrap_or_else(|| "https://integrate.api.nvidia.com/v1".to_string()),
            "NVIDIA".to_string(),
        )),
        "opencode-zen" | "opencode_zen" => std::sync::Arc::new(OpenCodeProvider::new(
            api_key,
            dynamic_api.unwrap_or_else(|| "https://opencode.ai/zen/v1".to_string()),
            "OpenCode Zen".to_string(),
            true,
        )),
        "opencode-go" | "opencode_go" => std::sync::Arc::new(OpenCodeProvider::new(
            api_key,
            dynamic_api.unwrap_or_else(|| "https://opencode.ai/zen/go/v1".to_string()),
            "OpenCode Go".to_string(),
            false,
        )),
        "openai" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            dynamic_api.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            "OpenAI".to_string(),
        )),
        "vertex" => {
            let env_project = std::env::var("GOOGLE_CLOUD_PROJECT")
                .or_else(|_| std::env::var("GCP_PROJECT"))
                .or_else(|_| std::env::var("GCLOUD_PROJECT"))
                .unwrap_or_default();
            
            let final_project = if vertex_project.is_empty() {
                &env_project
            } else {
                vertex_project
            };

            let env_location = std::env::var("GOOGLE_VERTEX_LOCATION")
                .or_else(|_| std::env::var("GOOGLE_CLOUD_LOCATION"))
                .or_else(|_| std::env::var("VERTEX_LOCATION"))
                .unwrap_or_else(|_| "us-central1".to_string());

            let final_location = if vertex_location.is_empty() {
                &env_location
            } else {
                vertex_location
            };

            if final_project.is_empty() {
                // Seamless fallback to Gemini API (Google AI Studio) if no GCP Project is available
                std::sync::Arc::new(GeminiProvider::new(api_key, dynamic_api))
            } else {
                std::sync::Arc::new(VertexAIProvider::new(
                    api_key,
                    final_project.to_string(),
                    final_location.to_string(),
                ))
            }
        }
        _ => {
            if provider_name.starts_with("http") {
                std::sync::Arc::new(openai::OpenAIProvider::new(
                    api_key,
                    provider_name.to_string(),
                    provider_name.to_string(),
                ))
            } else if let Some(api) = dynamic_api {
                // If it's an unknown provider but we found an API in the dynamic registry,
                // fallback to using it as an OpenAI-compatible endpoint!
                std::sync::Arc::new(openai::OpenAIProvider::new(
                    api_key,
                    api,
                    provider_name.to_string(),
                ))
            } else {
                std::sync::Arc::new(OpenRouter::new(api_key))
            }
        }
    }
}
