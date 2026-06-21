//! Hook output aggregator.
//!
//! Combines the outputs of multiple hooks for the same event into
//! a single decision. Mirrors Claude Code's combine semantics:
//!
//! 1. If any hook returns `decision: block` (PreToolUse), the call
//!    is denied. The first non-empty `reason` wins.
//! 2. If any hook returns `continue: false`, the agent stops. The
//!    first non-empty `stop_reason` wins.
//! 3. `additional_context` from all hooks is concatenated, joined
//!    by blank lines, in the order received.
//! 4. `system_message` from all hooks is concatenated, joined by
//!    newlines, in the order received.
//! 5. For `hook_specific_output.updatedInput` (PreToolUse) and
//!    `updatedMCPToolOutput` (PostToolUse), last-write-wins (Claude
//!    Code's override-chain semantics).
//!
//! The aggregator never panics on an empty input; `aggregate_results`
//! of `[]` returns the default "no opinion" aggregate.

use serde_json::Value;

use crate::hooks::output::{HookOutput, HookSpecificOutput, PreToolUseDecision};

/// The combined result of running all hooks for a single event.
#[derive(Debug, Clone, Default)]
pub struct Aggregated {
    /// Permission decision. Default `Approve` (no opinion). Set to
    /// `Block` if any hook returned `decision: block`.
    pub decision: PreToolUseDecision,
    /// Whether the agent should keep running. Default true. Set to
    /// false if any hook returned `continue: false`.
    pub continue_: bool,
    /// Human-readable reason the agent stopped (shown when
    /// `continue_` is false).
    pub stop_reason: Option<String>,
    /// Human-readable reason the call was blocked (PreToolUse).
    pub reason: Option<String>,
    /// Concatenated additional context to inject into the model
    /// context, joined by blank lines.
    pub additional_context: Option<String>,
    /// Concatenated system messages for the UI, joined by newlines.
    pub system_message: Option<String>,
    /// Per-event structured output (e.g. `updatedInput` for
    /// PreToolUse). Last-write-wins across hooks.
    pub hook_specific_output: Option<HookSpecificOutput>,
    /// For PreToolUse: merged `updatedInput` (last-write-wins).
    /// Convenience accessor — same as
    /// `hook_specific_output.updatedInput`.
    pub updated_input: Option<Value>,
    /// For PostToolUse: merged `updatedMCPToolOutput` (last-write-wins).
    pub updated_mcp_tool_output: Option<Value>,
}

impl Aggregated {
    /// True if a PreToolUse hook blocked the call.
    pub fn should_block(&self) -> bool {
        self.decision == PreToolUseDecision::Block
    }

    /// True if any hook stopped the agent.
    pub fn should_stop(&self) -> bool {
        !self.continue_
    }

    /// True if there's any context to inject into the model
    /// context.
    pub fn has_context(&self) -> bool {
        self.additional_context.is_some()
            || self.updated_input.is_some()
            || self.updated_mcp_tool_output.is_some()
    }
}

/// Combine a list of hook outputs into a single `Aggregated`. Order
/// is preserved for concatenations; first-non-empty wins for
/// reason/stop_reason; last-non-empty wins for override-style
/// fields.
pub fn aggregate_results(results: Vec<HookOutput>) -> Aggregated {
    let mut out = Aggregated {
        continue_: true,
        ..Default::default()
    };

    let mut contexts: Vec<String> = Vec::new();
    let mut messages: Vec<String> = Vec::new();

    for r in results {
        // 1. Continue: false wins (any). If any said false, the
        //    agent stops.
        if r.continue_ == Some(false) {
            out.continue_ = false;
            if out.stop_reason.is_none() {
                out.stop_reason = r.stop_reason.clone();
            }
        }
        // 2. Decision: block wins. The first reason wins.
        if r.decision == Some(PreToolUseDecision::Block) {
            out.decision = PreToolUseDecision::Block;
            if out.reason.is_none() {
                out.reason = r.reason.clone();
            }
        }
        // 3. additionalContext: top-level + hookSpecificOutput.
        if let Some(ctx) = r.additional_context.as_deref() {
            let trimmed = ctx.trim();
            if !trimmed.is_empty() {
                contexts.push(trimmed.to_string());
            }
        }
        if let Some(ref hso) = r.hook_specific_output {
            match hso {
                HookSpecificOutput::PreToolUse { additionalContext, .. } => {
                    if let Some(ctx) = additionalContext.as_deref() {
                        let trimmed = ctx.trim();
                        if !trimmed.is_empty() {
                            contexts.push(trimmed.to_string());
                        }
                    }
                }
                HookSpecificOutput::PostToolUse { additionalContext, .. } => {
                    if let Some(ctx) = additionalContext.as_deref() {
                        let trimmed = ctx.trim();
                        if !trimmed.is_empty() {
                            contexts.push(trimmed.to_string());
                        }
                    }
                }
                HookSpecificOutput::PostToolUseFailure { additionalContext } => {
                    if let Some(ctx) = additionalContext.as_deref() {
                        let trimmed = ctx.trim();
                        if !trimmed.is_empty() {
                            contexts.push(trimmed.to_string());
                        }
                    }
                }
                HookSpecificOutput::Stop { additionalContext } => {
                    if let Some(ctx) = additionalContext.as_deref() {
                        let trimmed = ctx.trim();
                        if !trimmed.is_empty() {
                            contexts.push(trimmed.to_string());
                        }
                    }
                }
                HookSpecificOutput::StopFailure { additionalContext } => {
                    if let Some(ctx) = additionalContext.as_deref() {
                        let trimmed = ctx.trim();
                        if !trimmed.is_empty() {
                            contexts.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        // 4. systemMessage.
        if let Some(msg) = r.system_message.as_deref() {
            let trimmed = msg.trim();
            if !trimmed.is_empty() {
                messages.push(trimmed.to_string());
            }
        }
        // 5. override-style fields: last-write-wins.
        if let Some(hso) = r.hook_specific_output.clone() {
            // Update top-level mirrors for convenience.
            match &hso {
                HookSpecificOutput::PreToolUse { updatedInput, .. } => {
                    if updatedInput.is_some() {
                        out.updated_input = updatedInput.clone();
                    }
                }
                HookSpecificOutput::PostToolUse {
                    updatedMCPToolOutput,
                    ..
                } => {
                    if updatedMCPToolOutput.is_some() {
                        out.updated_mcp_tool_output = updatedMCPToolOutput.clone();
                    }
                }
                _ => {}
            }
            out.hook_specific_output = Some(hso);
        }
    }

    if !contexts.is_empty() {
        out.additional_context = Some(contexts.join("\n\n"));
    }
    if !messages.is_empty() {
        out.system_message = Some(messages.join("\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok() -> HookOutput {
        HookOutput::ok()
    }

    fn block(reason: &str) -> HookOutput {
        HookOutput::block(reason)
    }

    #[test]
    fn empty_input_is_default_approve_continue() {
        let agg = aggregate_results(Vec::new());
        assert_eq!(agg.decision, PreToolUseDecision::Approve);
        assert!(agg.continue_);
        assert!(!agg.should_block());
        assert!(!agg.should_stop());
        assert!(agg.additional_context.is_none());
    }

    #[test]
    fn one_block_wins() {
        let agg = aggregate_results(vec![ok(), block("nope")]);
        assert_eq!(agg.decision, PreToolUseDecision::Block);
        assert!(agg.should_block());
        assert_eq!(agg.reason.as_deref(), Some("nope"));
    }

    #[test]
    fn first_block_reason_wins() {
        let agg = aggregate_results(vec![block("first"), block("second")]);
        assert!(agg.should_block());
        assert_eq!(agg.reason.as_deref(), Some("first"));
    }

    #[test]
    fn stop_short_circuits_continue() {
        let mut a = ok();
        a.continue_ = Some(false);
        a.stop_reason = Some("user said so".into());
        let agg = aggregate_results(vec![a]);
        assert!(!agg.continue_);
        assert!(agg.should_stop());
        assert_eq!(agg.stop_reason.as_deref(), Some("user said so"));
    }

    #[test]
    fn stop_from_any_hook() {
        let mut a = ok();
        a.continue_ = Some(false);
        let agg = aggregate_results(vec![ok(), a, ok()]);
        assert!(!agg.continue_);
        assert!(agg.should_stop());
    }

    #[test]
    fn additional_context_concatenated_in_order() {
        let a = HookOutput::additional_context("alpha");
        let b = HookOutput::additional_context("beta");
        let agg = aggregate_results(vec![a, b]);
        let ctx = agg.additional_context.unwrap();
        assert!(ctx.contains("alpha"));
        assert!(ctx.contains("beta"));
        assert!(ctx.find("alpha").unwrap() < ctx.find("beta").unwrap());
    }

    #[test]
    fn system_messages_concatenated() {
        let mut a = ok();
        a.system_message = Some("one".into());
        let mut b = ok();
        b.system_message = Some("two".into());
        let agg = aggregate_results(vec![a, b]);
        assert_eq!(agg.system_message.as_deref(), Some("one\ntwo"));
    }

    #[test]
    fn updated_input_last_write_wins() {
        let hso1 = HookSpecificOutput::PreToolUse {
            permissionDecision: None,
            permissionDecisionReason: None,
            updatedInput: Some(json!({"command": "first"})),
            additionalContext: None,
        };
        let hso2 = HookSpecificOutput::PreToolUse {
            permissionDecision: None,
            permissionDecisionReason: None,
            updatedInput: Some(json!({"command": "second"})),
            additionalContext: None,
        };
        let mut a = ok();
        a.hook_specific_output = Some(hso1);
        let mut b = ok();
        b.hook_specific_output = Some(hso2);
        let agg = aggregate_results(vec![a, b]);
        assert_eq!(agg.updated_input, Some(json!({"command": "second"})));
    }

    #[test]
    fn updated_mcp_tool_output_last_write_wins() {
        let hso1 = HookSpecificOutput::PostToolUse {
            additionalContext: None,
            updatedMCPToolOutput: Some(json!({"v": 1})),
        };
        let hso2 = HookSpecificOutput::PostToolUse {
            additionalContext: None,
            updatedMCPToolOutput: Some(json!({"v": 2})),
        };
        let mut a = ok();
        a.hook_specific_output = Some(hso1);
        let mut b = ok();
        b.hook_specific_output = Some(hso2);
        let agg = aggregate_results(vec![a, b]);
        assert_eq!(agg.updated_mcp_tool_output, Some(json!({"v": 2})));
    }

    #[test]
    fn additional_context_in_hso_also_aggregated() {
        let hso = HookSpecificOutput::PreToolUse {
            permissionDecision: None,
            permissionDecisionReason: None,
            updatedInput: None,
            additionalContext: Some("from-hso".into()),
        };
        let mut a = HookOutput::additional_context("from-top");
        a.hook_specific_output = Some(hso);
        let agg = aggregate_results(vec![a]);
        let ctx = agg.additional_context.unwrap();
        assert!(ctx.contains("from-top"));
        assert!(ctx.contains("from-hso"));
    }

    #[test]
    fn block_after_ok_does_not_override_reason() {
        // Once blocked, additional reason from a later block does
        // not overwrite the first one.
        let agg = aggregate_results(vec![block("first"), ok(), block("ignored")]);
        assert!(agg.should_block());
        assert_eq!(agg.reason.as_deref(), Some("first"));
    }
}
