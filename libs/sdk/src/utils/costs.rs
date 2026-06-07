use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub total_cost: f64,
    /// Number of QIR retry attempts that occurred in this session.
    /// Each retry is a billable request even if the user only sees the
    /// final successful response, so this is surfaced to the UI to
    /// prevent "invisible" cost from QIR usage.
    #[serde(default)]
    pub qir_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRates {
    pub input_per_1m: f64,
    pub output_per_1m: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsDevProvider {
    pub models: HashMap<String, ModelsDevModel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsDevModel {
    pub cost: Option<ModelsDevCost>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsDevCost {
    pub input: f64,
    pub output: f64,
}

static RATE_CACHE: Lazy<Arc<RwLock<HashMap<String, ModelRates>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

impl Usage {
    pub async fn add(&mut self, input: u32, output: u32, model: &str) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens += input + output;

        let cost = calculate_cost(input, output, model).await;
        self.total_cost += cost;
    }

    pub fn record_qir_attempt(&mut self) {
        self.qir_attempts += 1;
    }
}

pub async fn calculate_cost(input: u32, output: u32, model: &str) -> f64 {
    let rates = get_model_rates(model).await;
    let input_cost = (input as f64 / 1_000_000.0) * rates.input_per_1m;
    let output_cost = (output as f64 / 1_000_000.0) * rates.output_per_1m;
    input_cost + output_cost
}

pub async fn refresh_rates() -> anyhow::Result<()> {
    log::debug!("Refreshing model rates from models.dev...");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get("https://models.dev/api.json").send().await?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Failed to fetch rates: {}", response.status()));
    }

    let data: HashMap<String, ModelsDevProvider> = response.json().await?;
    let mut new_rates = HashMap::new();

    for (_provider_id, provider) in data {
        for (model_id, model) in provider.models {
            if let Some(cost) = model.cost {
                new_rates.insert(model_id, ModelRates {
                    input_per_1m: cost.input,
                    output_per_1m: cost.output,
                });
            }
        }
    }

    if !new_rates.is_empty() {
        let mut cache = RATE_CACHE.write().unwrap_or_else(|e| e.into_inner());
        *cache = new_rates;
        log::info!("Successfully updated {} model rates from models.dev", cache.len());
    }

    Ok(())
}

use std::sync::atomic::{AtomicBool, Ordering};

static REFRESH_TRIGGERED: AtomicBool = AtomicBool::new(false);

async fn get_model_rates(model: &str) -> ModelRates {
    // 1. Check cache
    {
        let cache = RATE_CACHE.read().unwrap_or_else(|e| e.into_inner());
        if let Some(rates) = cache.get(model) {
            return rates.clone();
        }
        
        // Try fuzzy match if exact match fails (e.g. "gpt-4o-2024-05-13" vs "gpt-4o")
        for (cached_id, rates) in cache.iter() {
            if model.contains(cached_id) || cached_id.contains(model) {
                return rates.clone();
            }
        }
    }

    // 2. If cache is empty, trigger one-time background refresh
    let cache_is_empty = {
        let cache = RATE_CACHE.read().unwrap_or_else(|e| e.into_inner());
        cache.is_empty()
    };

    if cache_is_empty && !REFRESH_TRIGGERED.swap(true, Ordering::SeqCst) {
        tokio::spawn(async {
            let _ = refresh_rates().await;
        });
    }

    // 3. Fallback to hardcoded defaults for common models if API fails or model is missing
    get_fallback_rates(model)
}

fn get_fallback_rates(model: &str) -> ModelRates {
    if model.contains("gpt-4o-mini") {
        ModelRates { input_per_1m: 0.15, output_per_1m: 0.60 }
    } else if model.contains("gpt-4o") {
        ModelRates { input_per_1m: 5.0, output_per_1m: 15.0 }
    } else if model.contains("claude-3-5-sonnet") {
        ModelRates { input_per_1m: 3.0, output_per_1m: 15.0 }
    } else if model.contains("deepseek-v3") || model.contains("deepseek-chat") {
        ModelRates { input_per_1m: 0.14, output_per_1m: 0.28 }
    } else {
        if !model.is_empty() {
            log::warn!("Unknown model '{}' for cost calculation. Using default fallback rates.", model);
        }
        // Default GPT-4o style fallback
        ModelRates { input_per_1m: 5.0, output_per_1m: 15.0 }
    }
}
