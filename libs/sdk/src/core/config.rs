use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicModelInfo {
    pub name: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub provider: String,
    pub theme: String,
    pub api_keys: HashMap<String, String>,
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub last_update_check: f64,
    #[serde(default)]
    pub favorites: Vec<DynamicModelInfo>,
    #[serde(default)]
    pub recent_models: Vec<DynamicModelInfo>,
    #[serde(default = "default_thinking_level")]
    pub thinking_level: String,
    #[serde(default = "default_logo_animation")]
    pub logo_animation: String,
    #[serde(default = "default_logo_animation_color")]
    pub logo_animation_color: String,
    #[serde(default)]
    pub vertex_project: String,
    #[serde(default)]
    pub vertex_location: String,
}

fn default_thinking_level() -> String {
    "default".to_string()
}

fn default_logo_animation() -> String {
    "always".to_string()
}

fn default_logo_animation_color() -> String {
    "rainbow".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            theme: "default".to_string(),
            api_keys: HashMap::new(),
            allowlist: Vec::new(),
            last_update_check: 0.0,
            favorites: Vec::new(),
            recent_models: Vec::new(),
            thinking_level: "default".to_string(),
            logo_animation: "always".to_string(),
            logo_animation_color: "rainbow".to_string(),
            vertex_project: String::new(),
            vertex_location: "us-central1".to_string(),
        }
    }
}

impl Config {
    pub fn get_api_key(&self) -> Option<&String> {
        self.api_keys.get(&self.provider)
    }
}
