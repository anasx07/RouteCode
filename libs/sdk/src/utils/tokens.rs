use crate::core::Message;
use tiktoken_rs::cl100k_base;

pub fn count_tokens(messages: &[Message]) -> usize {
    let bpe = match cl100k_base() {
        Ok(b) => b,
        Err(_) => return 0,
    };

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
}
