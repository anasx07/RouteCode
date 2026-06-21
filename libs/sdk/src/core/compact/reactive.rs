/// Helper to detect context length or token limit errors from API providers.
pub fn is_prompt_too_long_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("413")
        || msg.contains("prompt too long")
        || msg.contains("context_length_exceeded")
        || msg.contains("context length exceeded")
        || msg.contains("too many tokens")
        || msg.contains("token limit")
        || msg.contains("context limit")
        || msg.contains("context window")
}
