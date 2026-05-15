use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub total_cost: f64,
}

pub struct ModelRates {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
}

impl Usage {
    pub fn add(&mut self, input: u32, output: u32, model: &str) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens += input + output;

        let cost = calculate_cost(input, output, model);
        self.total_cost += cost;
    }
}

pub fn calculate_cost(input: u32, output: u32, model: &str) -> f64 {
    let rates = get_model_rates(model);
    let input_cost = (input as f64 / 1000.0) * rates.input_per_1k;
    let output_cost = (output as f64 / 1000.0) * rates.output_per_1k;
    input_cost + output_cost
}

fn get_model_rates(model: &str) -> ModelRates {
    // Default rates (GPT-4o style)
    let mut rates = ModelRates {
        input_per_1k: 0.005,
        output_per_1k: 0.015,
    };

    if model.contains("gpt-4o-mini") {
        rates.input_per_1k = 0.00015;
        rates.output_per_1k = 0.0006;
    } else if model.contains("claude-3-5-sonnet") {
        rates.input_per_1k = 0.003;
        rates.output_per_1k = 0.015;
    } else if model.contains("claude-3-opus") {
        rates.input_per_1k = 0.015;
        rates.output_per_1k = 0.075;
    } else if model.contains("deepseek-v3") || model.contains("deepseek-chat") {
        rates.input_per_1k = 0.0001;
        rates.output_per_1k = 0.0002;
    }

    rates
}
