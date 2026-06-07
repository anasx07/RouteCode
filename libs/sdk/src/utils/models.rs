use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub family: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub api: Option<String>,
    pub env: Option<Vec<String>>,
    #[serde(default)]
    pub models: HashMap<String, ModelInfo>,
}

pub fn get_models_cache_path() -> PathBuf {
    crate::utils::storage::get_base_dir().join("models.json")
}

pub async fn fetch_and_cache_models() -> anyhow::Result<()> {
    let url = std::env::var("ROUTECODE_MODELS_URL")
        .unwrap_or_else(|_| "https://models.dev/api.json".to_string());
    
    // Check if cache is fresh enough (e.g., < 24 hours old)
    let cache_path = get_models_cache_path();
    if cache_path.exists() {
        if let Ok(metadata) = fs::metadata(&cache_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = SystemTime::now().duration_since(modified) {
                    if duration.as_secs() < 24 * 3600 {
                        // Cache is fresh, no need to fetch
                        return Ok(());
                    }
                }
            }
        }
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
        
    let response = client.get(&url).send().await?;
    
    if response.status().is_success() {
        let text = response.text().await?;
        // Verify it parses correctly before saving
        let _parsed: HashMap<String, ProviderInfo> = serde_json::from_str(&text)?;
        
        let dir = crate::utils::storage::get_base_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        
        fs::write(cache_path, text)?;
    } else {
        anyhow::bail!("Failed to fetch models: HTTP {}", response.status());
    }
    
    Ok(())
}

pub fn get_models_for_provider(provider_id: &str) -> Option<Vec<String>> {
    let cache_path = get_models_cache_path();
    if !cache_path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(cache_path).ok()?;
    let registry: HashMap<String, ProviderInfo> = serde_json::from_str(&content).ok()?;
    
    // opencode sometimes uses provider ids like "google-vertex"
    // Let's find the provider that matches or contains the provider_id
    for (id, provider) in registry {
        if id == provider_id || id.contains(provider_id) || provider_id.contains(&id) {
            let mut models: Vec<String> = provider.models.keys().cloned().collect();
            // Sort to ensure consistent ordering
            models.sort();
            return Some(models);
        }
    }
    
    None
}

pub fn get_provider_info(provider_id: &str) -> Option<ProviderInfo> {
    let cache_path = get_models_cache_path();
    if !cache_path.exists() {
        return None;
    }
    
    let content = fs::read_to_string(cache_path).ok()?;
    let registry: HashMap<String, ProviderInfo> = serde_json::from_str(&content).ok()?;
    
    for (id, provider) in registry {
        if id == provider_id || id.contains(provider_id) || provider_id.contains(&id) {
            return Some(provider);
        }
    }
    
    None
}
