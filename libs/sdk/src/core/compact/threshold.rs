use std::cmp::min;

#[derive(Debug, Clone, Copy)]
pub struct CompactThresholds {
    pub context_window: usize,
    pub effective_context_window: usize,
    pub auto_compact_threshold: usize,
    pub warning_threshold: usize,
    pub blocking_limit: usize,
}

/// Estimates the context window for a given model.
pub fn get_context_window(model: &str) -> usize {
    let lower = model.to_lowercase();
    if lower.contains("claude-3-5") || lower.contains("claude-3") {
        200_000
    } else if lower.contains("gpt-4o") {
        128_000
    } else if lower.contains("gemini") {
        1_000_000
    } else {
        200_000 // default fallback (including o1, o3-mini, and unknown models)
    }
}

pub fn calculate_thresholds(model: &str, window_override: Option<usize>) -> CompactThresholds {
    let context_window = window_override.unwrap_or_else(|| get_context_window(model));
    
    // max_output_tokens is typically 16,384 or 8,192, we can assume 16,384 as standard,
    // or up to 20,000. Let's cap at 20,000 as Claude Code does.
    let max_output_tokens = 16_384;
    let reserved_output_tokens = min(max_output_tokens, 20_000);
    
    let effective_context_window = if context_window > reserved_output_tokens {
        context_window - reserved_output_tokens
    } else {
        context_window
    };

    // autoCompactThreshold = effectiveContextWindow - 13,000
    let auto_compact_threshold = if effective_context_window > 13_000 {
        effective_context_window - 13_000
    } else {
        effective_context_window * 8 / 10 // 80% fallback if tiny
    };

    // warningThreshold = auto_compact_threshold - 20,000
    let warning_threshold = if auto_compact_threshold > 20_000 {
        auto_compact_threshold - 20_000
    } else {
        auto_compact_threshold * 7 / 10
    };

    // blockingLimit = effectiveContextWindow - 3,000
    let blocking_limit = if effective_context_window > 3_000 {
        effective_context_window - 3_000
    } else {
        effective_context_window * 95 / 100
    };

    CompactThresholds {
        context_window,
        effective_context_window,
        auto_compact_threshold,
        warning_threshold,
        blocking_limit,
    }
}
