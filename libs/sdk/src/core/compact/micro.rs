use crate::core::Message;
use std::sync::Arc;

/// Micro-compactor that clears out the body of old tool results to reclaim context tokens.
/// Keeps the most recent `keep_recent` tool results fully intact, and replaces the content
/// of older ones with a lightweight placeholder JSON.
pub fn micro_compact(messages: &mut [Message], keep_recent: usize) {
    let mut tool_count = 0;
    
    // Iterate in reverse to count tool messages from newest to oldest
    for msg in messages.iter_mut().rev() {
        if msg.role == crate::core::Role::Tool {
            tool_count += 1;
            if tool_count > keep_recent {
                // If the message has content that's non-trivial, clear it.
                if let Some(content) = &msg.content {
                    if content.len() > 120 {
                        let cleared_json = serde_json::json!({
                            "success": true,
                            "content": "[Old tool result content cleared to save context space]"
                        });
                        msg.content = Some(Arc::from(cleared_json.to_string()));
                    }
                }
            }
        }
    }
}
