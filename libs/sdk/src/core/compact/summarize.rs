use crate::core::Message;
use crate::agents::AIProvider;
use crate::agents::types::StreamChunk;
use futures::StreamExt;
use std::sync::Arc;

/// Find a safe split index to ensure we never split an assistant tool-call from its subsequent tool results.
pub fn find_safe_split_index(messages: &[Message], target_keep: usize) -> usize {
    if messages.len() <= target_keep {
        return 0;
    }
    let mut split_idx = messages.len() - target_keep;
    
    // 1. Ensure we don't start the preserved part with a Role::Tool.
    // If messages[split_idx] is a Tool message, move backwards until we find the Assistant message that spawned it.
    while split_idx > 0 && messages[split_idx].role == crate::core::Role::Tool {
        split_idx -= 1;
    }
    
    // 2. Ensure we don't split between an Assistant message with tool_calls and the Tool message immediately following it.
    while split_idx > 0 
        && messages[split_idx - 1].role == crate::core::Role::Assistant 
        && messages[split_idx - 1].tool_calls.is_some() 
    {
        split_idx -= 1;
    }
    
    split_idx
}

/// Calls the AI provider to summarize a segment of the conversation.
pub async fn compact_conversation(
    provider: Arc<dyn AIProvider>,
    model: &str,
    messages_to_summarize: &[Message],
) -> Result<String, anyhow::Error> {
    let mut compact_messages = messages_to_summarize.to_vec();
    compact_messages.push(Message::user(
        "Please provide a comprehensive and detailed technical summary of our conversation so far. \
         Structure the summary to include:\n\
         1. Primary Request and Intent\n\
         2. Key Technical Concepts\n\
         3. Files and Code Sections read or modified\n\
         4. Errors encountered and their fixes\n\
         5. Pending tasks and current state\n\
         6. Next steps\n\n\
         Do not include any conversational filler, intro, or outro. Start directly with the summary."
    ));

    let stream_res = provider.ask(
        Arc::new(compact_messages),
        model,
        Arc::new(None),
        None,
    ).await?;

    let mut summary = String::new();
    let mut s = stream_res;
    while let Some(chunk_res) = s.next().await {
        match chunk_res {
            Ok(StreamChunk::Text { content }) => {
                summary.push_str(&content);
            }
            Ok(StreamChunk::Error { content }) => {
                return Err(anyhow::anyhow!("Summarization error: {}", content));
            }
            Err(e) => {
                return Err(e);
            }
            _ => {}
        }
    }

    if summary.trim().is_empty() {
        return Err(anyhow::anyhow!("Summarizer returned an empty response."));
    }

    Ok(summary)
}

/// Builds the post-compact message list:
/// 1. The boundary marker: system message with content "Conversation compacted"
/// 2. The formatted summary: system message containing the LLM summary
/// 3. The preserved recent messages
pub fn build_post_compact_messages(summary: &str, preserved_messages: &[Message]) -> Vec<Message> {
    let boundary_marker = Message::system("Conversation compacted");
    
    let summary_text = format!(
        "This session is being continued from a previous conversation that ran out of context.\n\
         The summary below covers the earlier portion of the conversation.\n\n\
         # Summary\n\
         {}\n\n\
         Recent messages are preserved verbatim.\n\n\
         Continue the conversation from where it left off without asking the user any \
         further questions. Resume directly — do not acknowledge the summary, do not \
         recap what was happening, do not preface with \"I'll continue\" or similar. \
         Pick up the last task as if the break never happened.",
        summary
    );
    let summary_msg = Message::system(summary_text);

    let mut result = vec![boundary_marker, summary_msg];
    result.extend(preserved_messages.iter().cloned());
    result
}

pub fn find_last_compact_boundary(messages: &[Message]) -> Option<usize> {
    messages.iter().rposition(|m| {
        m.role == crate::core::Role::System && m.content.as_deref() == Some("Conversation compacted")
    })
}
