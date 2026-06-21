pub mod threshold;
pub mod micro;
pub mod summarize;
pub mod reactive;
pub mod tracking;

pub use threshold::{calculate_thresholds, get_context_window, CompactThresholds};
pub use micro::micro_compact;
pub use summarize::{
    build_post_compact_messages, compact_conversation, find_last_compact_boundary,
    find_safe_split_index,
};
pub use reactive::is_prompt_too_long_error;
pub use tracking::AutoCompactState;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Message, Role, ToolCall, FunctionCall};

    #[test]
    fn test_calculate_thresholds() {
        // Test standard Sonnet calculation
        let thresholds = calculate_thresholds("claude-3-5-sonnet", None);
        assert_eq!(thresholds.context_window, 200_000);
        assert_eq!(thresholds.effective_context_window, 200_000 - 16_384);
        assert_eq!(thresholds.auto_compact_threshold, thresholds.effective_context_window - 13_000);
        assert_eq!(thresholds.blocking_limit, thresholds.effective_context_window - 3_000);

        // Test with override
        let override_thresholds = calculate_thresholds("claude-3-5-sonnet", Some(100_000));
        assert_eq!(override_thresholds.context_window, 100_000);
        assert_eq!(override_thresholds.effective_context_window, 100_000 - 16_384);
    }

    #[test]
    fn test_micro_compact() {
        let mut messages = vec![
            Message::user("hello"),
            Message::tool("t1".to_string(), "bash".to_string(), "long_tool_result_content_that_exceeds_threshold_and_should_be_cleared_by_micro_compactor_to_reclaim_context_tokens_efficiently_and_smoothly_without_breaking"),
            Message::tool("t2".to_string(), "bash".to_string(), "short"),
            Message::tool("t3".to_string(), "bash".to_string(), "another_long_tool_result_content_that_exceeds_threshold_and_should_be_cleared_by_micro_compactor_to_reclaim_context_tokens_efficiently_and_smoothly_without_breaking"),
            Message::tool("t4".to_string(), "bash".to_string(), "short2"),
            Message::tool("t5".to_string(), "bash".to_string(), "short3"),
            Message::tool("t6".to_string(), "bash".to_string(), "short4"),
        ];

        // Compact with keeping last 2
        micro_compact(&mut messages, 2);

        // Last 2 tool messages (t5, t6) should be intact
        assert!(messages[6].content.as_ref().unwrap().contains("short4"));
        assert!(messages[5].content.as_ref().unwrap().contains("short3"));

        // t3 is older (3rd from end) and was long, so it should be cleared
        assert!(messages[3].content.as_ref().unwrap().contains("Old tool result content cleared"));

        // t2 is older but was short (< 120 chars), so it should be intact
        assert!(messages[2].content.as_ref().unwrap().contains("short"));

        // t1 is older and long, so it should be cleared
        assert!(messages[1].content.as_ref().unwrap().contains("Old tool result content cleared"));
    }

    #[test]
    fn test_find_safe_split_index() {
        let tcall = ToolCall {
            index: Some(0),
            id: "t1".to_string(),
            r#type: "function".to_string(),
            function: FunctionCall {
                name: "bash".to_string(),
                arguments: "ls".to_string(),
            },
        };

        let messages = vec![
            Message::user("hello"),
            Message::assistant(None, None, Some(vec![tcall])),
            Message::tool("t1".to_string(), "bash".to_string(), "result"),
            Message::user("next step"),
        ];

        // If target_keep is 2, split_idx should be 1 (after the User message at 0).
        // Preserved segment starts at 1, keeping both the Assistant and Tool together.
        let split_idx = find_safe_split_index(&messages, 2);
        assert_eq!(split_idx, 1);

        // If target_keep is 3, split_idx should be 1 because keeping 3 preserves Assistant (1), Tool (2), and User (3).
        let split_idx_3 = find_safe_split_index(&messages, 3);
        assert_eq!(split_idx_3, 1);
    }

    #[test]
    fn test_build_post_compact_messages() {
        let preserved = vec![Message::user("hi")];
        let post_compact = build_post_compact_messages("This is summary", &preserved);
        
        assert_eq!(post_compact.len(), 3);
        assert_eq!(post_compact[0].role, Role::System);
        assert_eq!(post_compact[0].content.as_deref(), Some("Conversation compacted"));
        
        assert_eq!(post_compact[1].role, Role::System);
        assert!(post_compact[1].content.as_ref().unwrap().contains("This is summary"));
        
        assert_eq!(post_compact[2].role, Role::User);
        assert_eq!(post_compact[2].content.as_deref(), Some("hi"));
    }
}
