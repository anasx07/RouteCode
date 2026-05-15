pub mod openai;
pub mod openrouter;
pub mod traits;
pub mod types;
pub mod utils;

pub use openai::OpenAIProvider;
pub use openrouter::OpenRouter;
pub use traits::AIProvider;
pub use types::{StreamChunk, Usage};

pub fn resolve_provider(provider_name: &str, api_key: String) -> std::sync::Arc<dyn AIProvider> {
    match provider_name.to_lowercase().as_str() {
        "openrouter" => std::sync::Arc::new(OpenRouter::new(api_key)),
        "nvidia" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            "https://integrate.api.nvidia.com/v1".to_string(),
            "NVIDIA".to_string(),
        )),
        "opencode-zen" | "opencode_zen" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            "https://api.opencode.ai/zen/v1".to_string(),
            "OpenCode Zen".to_string(),
        )),
        "opencode-go" | "opencode_go" => std::sync::Arc::new(openai::OpenAIProvider::new(
            api_key,
            "https://api.opencode.ai/go/v1".to_string(),
            "OpenCode Go".to_string(),
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
