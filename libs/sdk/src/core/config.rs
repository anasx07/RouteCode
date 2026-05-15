use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        }
    }
}

impl Config {
    pub fn get_api_key(&self) -> Option<&String> {
        self.api_keys.get(&self.provider)
    }
}
