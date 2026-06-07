use crate::core::Message;
use once_cell::sync::Lazy;
use tiktoken_rs::{cl100k_base, CoreBPE};

static BPE: Lazy<Option<CoreBPE>> = Lazy::new(|| match cl100k_base() {
    Ok(b) => Some(b),
    Err(e) => {
        log::warn!(
            "tiktoken cl100k_base initialization failed: {}. Fallback estimation will be used.",
            e
        );
        None
    }
});

pub fn count_tokens(messages: &[Message]) -> usize {
    if let Some(bpe) = &*BPE {
        let mut total_tokens = 0;
        for m in messages {
            // Role overhead
            total_tokens += 4;

            if let Some(content) = &m.content {
                total_tokens += bpe.encode_with_special_tokens(content).len();
            }
            if let Some(thought) = &m.thought {
                total_tokens += bpe.encode_with_special_tokens(thought).len();
            }
            if let Some(tool_calls) = &m.tool_calls {
                for tc in tool_calls {
                    total_tokens += bpe.encode_with_special_tokens(&tc.function.name).len();
                    total_tokens += bpe.encode_with_special_tokens(&tc.function.arguments).len();
                    total_tokens += 10; // overhead
                }
            }
        }
        total_tokens
    } else {
        let mut total = 0;
        for m in messages {
            total += 4; // Role overhead
            if let Some(content) = &m.content {
                total += content.len() / 4;
            }
            if let Some(thought) = &m.thought {
                total += thought.len() / 4;
            }
            if let Some(tool_calls) = &m.tool_calls {
                for tc in tool_calls {
                    total += tc.function.name.len() / 4;
                    total += tc.function.arguments.len() / 4;
                    total += 10;
                }
            }
        }
        total
    }
}
