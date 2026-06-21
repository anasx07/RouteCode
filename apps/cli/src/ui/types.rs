use routecode_sdk::agents::types::ConfirmationResponse;
use routecode_sdk::agents::types::HookTrustEntry;
use routecode_sdk::agents::types::HookTrustResponse;
use routecode_sdk::agents::types::PlanApprovalResponse;
use routecode_sdk::core::DynamicModelInfo;

pub type ConfirmationSender =
    std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>>;

pub type PlanSender = std::sync::Arc<
    tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<PlanApprovalResponse>>>,
>;

pub type HookTrustSender = std::sync::Arc<
    tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<HookTrustResponse>>>,
>;

/// Plan approval dialog state: the plan markdown, its file path, the
/// list of allowed-prompt semantic permissions the AI requested, and
/// the one-shot channel back to the orchestrator.
pub type PendingPlan = (
    String,                // plan markdown
    String,                // plan file path
    Vec<(String, String)>, // (tool, prompt) allowed prompts
    PlanSender,
);

/// Hook trust dialog state: the project signature, project path,
/// list of hooks the project wants to register, and the response
/// channel.
pub struct PendingHookTrust {
    pub signature: String,
    pub project_path: String,
    pub hooks: Vec<HookTrustEntry>,
    pub tx: HookTrustSender,
}

impl Default for PendingHookTrust {
    fn default() -> Self {
        Self {
            signature: String::new(),
            project_path: String::new(),
            hooks: Vec::new(),
            tx: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
        }
    }
}

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
    /// Whether this provider requires a user-supplied API key. Keyless
    /// providers (e.g. OpenCode Zen/Go) skip the "no key configured" gate.
    pub requires_api_key: bool,
}

pub const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "openrouter",
        name: "OpenRouter",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "nvidia",
        name: "NVIDIA",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "opencode-zen",
        name: "OpenCode Zen",
        requires_api_key: false,
    },
    ProviderInfo {
        id: "opencode-go",
        name: "OpenCode Go",
        requires_api_key: false,
    },
    ProviderInfo {
        id: "openai",
        name: "OpenAI",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "anthropic",
        name: "Anthropic",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "gemini",
        name: "Google Gemini",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "deepseek",
        name: "DeepSeek",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "cloudflare-workers",
        name: "Cloudflare Workers AI",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "cloudflare-gateway",
        name: "Cloudflare AI Gateway",
        requires_api_key: true,
    },
    ProviderInfo {
        id: "vertex",
        name: "Google Vertex AI",
        requires_api_key: true,
    },
];

/// Look up a `ProviderInfo` by id. Returns `None` for unknown providers
/// (e.g. legacy config values) -- callers must treat `None` as "unknown,
/// require a key" for safety.
pub fn provider_info(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|p| p.id == id)
}

/// Whether the given provider requires a user-supplied API key. Unknown
/// providers default to `true` (require a key) so the user is prompted
/// rather than silently failing.
pub fn provider_requires_api_key(id: &str) -> bool {
    provider_info(id).is_none_or(|p| p.requires_api_key)
}

#[derive(Clone, Debug)]
pub enum ModelMenuItem {
    Header(String),
    Model(DynamicModelInfo),
}

pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

pub const COMMANDS: &[Command] = &[
    Command {
        name: "/model",
        description: "Switch model",
    },
    Command {
        name: "/resume",
        description: "Resume a session",
    },
    Command {
        name: "/sessions",
        description: "List saved sessions",
    },
    Command {
        name: "/clear",
        description: "Clear history",
    },
    Command {
        name: "/thinking",
        description: "Set thinking level (low/max)",
    },
    Command {
        name: "/help",
        description: "Show help",
    },
    Command {
        name: "/stop",
        description: "Stop AI generation",
    },
    Command {
        name: "/provider",
        description: "Manage providers",
    },
    Command {
        name: "/settings",
        description: "Manage settings",
    },
    Command {
        name: "/exit",
        description: "Exit application",
    },
];

#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum ApprovalMode {
    Normal,
    Plan,
    YOLO,
    Shell,
}

impl ApprovalMode {
    pub fn next(&self) -> Self {
        match self {
            ApprovalMode::Normal => ApprovalMode::Plan,
            ApprovalMode::Plan => ApprovalMode::YOLO,
            ApprovalMode::YOLO => ApprovalMode::Shell,
            ApprovalMode::Shell => ApprovalMode::Normal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ApprovalMode::Normal => "NORMAL",
            ApprovalMode::Plan => "PLAN",
            ApprovalMode::YOLO => "YOLO",
            ApprovalMode::Shell => "SHELL",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Screen {
    Welcome,
    Session,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ApiKeyInputStage {
    None,
    ApiKey,
    VertexProject,
    VertexLocation,
    CloudflareAccountId,
    CloudflareGatewayId,
    CloudflareApiKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsMenuItem {
    Header(String),
    Option {
        name: String,
        val: String,
        key: String,
    },
}

/// State of the most recent QIR retry event emitted by the SDK's
/// `RetryingProvider` wrapper. Mirrors the desktop app's `qirRetryStatus`
/// state and is rendered in the status bar so the user can see retries in
/// real time even when the relevant chunks have scrolled out of history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QirStatus {
    Retrying { attempt: u32 },
    StreamInterrupted { attempt: u32 },
    Recovered { attempt: u32 },
}

impl QirStatus {
    pub fn label(&self) -> String {
        match self {
            QirStatus::Retrying { attempt } => format!("retring (attempt {})", attempt),
            QirStatus::StreamInterrupted { attempt } => {
                format!("stream interrupted (attempt {})", attempt)
            }
            QirStatus::Recovered { attempt } => format!("recovered on attempt {}", attempt),
        }
    }

    pub fn is_recovered(&self) -> bool {
        matches!(self, QirStatus::Recovered { .. })
    }
}

/// Parse a `StreamChunk::Status` content string emitted by the SDK's
/// `RetryingProvider` wrapper into a `QirStatus`. Recognized prefixes
/// (stable, see `libs/sdk/src/agents/retry.rs`):
///
/// - `"QIR retrying (attempt N) -- ..."`
/// - `"QIR stream interrupted (attempt N) -- ..."`
/// - `"QIR recovered on attempt N"`
///
/// Returns `None` for any other content (so non-QIR status chunks are
/// ignored).
pub fn parse_qir_status(content: &str) -> Option<QirStatus> {
    let attempt_of = |s: &str| -> Option<u32> {
        const KEY: &str = "attempt ";
        let i = s.find(KEY)? + KEY.len();
        let rest = &s[i..];
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].parse().ok()
    };
    if content.starts_with("QIR recovered") {
        Some(QirStatus::Recovered {
            attempt: attempt_of(content)?,
        })
    } else if content.starts_with("QIR stream interrupted") {
        Some(QirStatus::StreamInterrupted {
            attempt: attempt_of(content)?,
        })
    } else if content.starts_with("QIR retrying") {
        Some(QirStatus::Retrying {
            attempt: attempt_of(content)?,
        })
    } else {
        None
    }
}

/// Format a `StreamChunk::Error` content string for display.
///
/// The content originates in `orchestrator.run()` wrapping `anyhow::Error`
/// via `Display`. The SDK's `HttpStatusError` produces
/// `"HTTP <code> <reason>: <body>"` -- we detect that prefix, surface the
/// status code (so the user can tell 401/403/429/5xx apart at a glance),
/// and extract a human-readable message from the JSON body when possible.
/// Anything that doesn't match the prefix is returned unchanged (covers
/// transport errors, `StreamChunk::Error` mid-stream, etc.).
pub fn format_error_for_display(content: &str) -> String {
    let Some(rest) = content.strip_prefix("HTTP ") else {
        return content.to_string();
    };
    let Some(colon_idx) = rest.find(':') else {
        return rest.trim().to_string();
    };
    let status_line = rest[..colon_idx].trim();
    let body = rest[colon_idx + 1..].trim();
    let message = extract_message_from_json(body).unwrap_or_else(|| body.to_string());
    if message.is_empty() {
        status_line.to_string()
    } else {
        format!("{}: {}", status_line, message)
    }
}

/// Try to extract a human message from a JSON body, walking the common
/// provider shapes. Returns `None` if the body is not JSON or matches no
/// known shape (callers fall through to the raw body).
fn extract_message_from_json(body: &str) -> Option<String> {
    if !body.starts_with('{') {
        return None;
    }
    let val: serde_json::Value = serde_json::from_str(body).ok()?;
    if let Some(msg) = val["error"]["message"].as_str() {
        return Some(msg.to_string());
    }
    if let Some(error_obj) = val["error"].as_object() {
        if let Some(msg) = error_obj["message"].as_str() {
            return Some(msg.to_string());
        }
    }
    if let Some(msg) = val["error"].as_str() {
        return Some(msg.to_string());
    }
    if let Some(msg) = val["message"].as_str() {
        return Some(msg.to_string());
    }
    if let Some(errors) = val["errors"].as_array() {
        if let Some(msg) = errors.first().and_then(|e| e["message"].as_str()) {
            return Some(msg.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_qir_status_retrying() {
        let s = parse_qir_status("QIR retrying (attempt 3) -- 503 Service Unavailable");
        assert_eq!(s, Some(QirStatus::Retrying { attempt: 3 }));
    }

    #[test]
    fn test_parse_qir_status_stream_interrupted() {
        let s = parse_qir_status("QIR stream interrupted (attempt 7) -- connection reset");
        assert_eq!(s, Some(QirStatus::StreamInterrupted { attempt: 7 }));
    }

    #[test]
    fn test_parse_qir_status_recovered() {
        let s = parse_qir_status("QIR recovered on attempt 4");
        assert_eq!(s, Some(QirStatus::Recovered { attempt: 4 }));
    }

    #[test]
    fn test_parse_qir_status_ignores_unrelated() {
        assert_eq!(parse_qir_status("Thinking..."), None);
        assert_eq!(parse_qir_status(""), None);
        assert_eq!(parse_qir_status("QIR something else"), None);
    }

    #[test]
    fn test_qir_status_label_and_recovered() {
        let r = QirStatus::Retrying { attempt: 2 };
        assert!(!r.is_recovered());
        assert!(r.label().contains("attempt 2"));
        let ok = QirStatus::Recovered { attempt: 3 };
        assert!(ok.is_recovered());
        assert!(ok.label().contains("attempt 3"));
    }

    #[test]
    fn test_format_error_http_with_json_body() {
        // OpenAI-style: {"error": {"message": "Incorrect API key provided"}}
        let body = r#"HTTP 401 Unauthorized: {"error":{"message":"Incorrect API key provided","type":"invalid_request_error"}}"#;
        let s = format_error_for_display(body);
        assert!(s.contains("401 Unauthorized"), "{}", s);
        assert!(s.contains("Incorrect API key provided"), "{}", s);
        assert!(
            !s.contains("invalid_request_error"),
            "raw JSON leaked: {}",
            s
        );
    }

    #[test]
    fn test_format_error_http_with_plain_body() {
        let s = format_error_for_display("HTTP 503 Service Unavailable: rate limited");
        assert!(s.contains("503 Service Unavailable"), "{}", s);
        assert!(s.contains("rate limited"), "{}", s);
    }

    #[test]
    fn test_format_error_http_with_top_level_message() {
        // Some providers put `message` at the top level.
        let body = r#"HTTP 400 Bad Request: {"message":"prompt too long"}"#;
        let s = format_error_for_display(body);
        assert!(s.contains("400 Bad Request"), "{}", s);
        assert!(s.contains("prompt too long"), "{}", s);
    }

    #[test]
    fn test_format_error_http_with_errors_array() {
        // Some providers return an array of errors.
        let body = r#"HTTP 422 Unprocessable Entity: {"errors":[{"message":"field x required"}]}"#;
        let s = format_error_for_display(body);
        assert!(s.contains("422"), "{}", s);
        assert!(s.contains("field x required"), "{}", s);
    }

    #[test]
    fn test_format_error_http_no_body() {
        let s = format_error_for_display("HTTP 500 Internal Server Error");
        assert!(s.contains("500 Internal Server Error"), "{}", s);
    }

    #[test]
    fn test_format_error_non_http_passthrough() {
        // Transport errors, mid-stream "Provider error: ..." etc.
        assert_eq!(
            format_error_for_display("Provider error: rate-limited"),
            "Provider error: rate-limited"
        );
        assert_eq!(
            format_error_for_display("connection reset"),
            "connection reset"
        );
        assert_eq!(format_error_for_display(""), "");
    }

    #[test]
    fn test_keyless_providers_are_marked() {
        assert!(!provider_requires_api_key("opencode-zen"));
        assert!(!provider_requires_api_key("opencode-go"));
    }

    #[test]
    fn test_known_providers_require_key() {
        for id in [
            "openrouter",
            "openai",
            "anthropic",
            "gemini",
            "deepseek",
            "vertex",
            "cloudflare-workers",
            "cloudflare-gateway",
            "nvidia",
        ] {
            assert!(
                provider_requires_api_key(id),
                "{} should require an API key",
                id
            );
        }
    }

    #[test]
    fn test_unknown_provider_defaults_to_require_key() {
        // Fail-closed: an unknown provider id (e.g. legacy config) must
        // not silently skip the "enter your key" gate.
        assert!(provider_requires_api_key("does-not-exist"));
    }

    #[test]
    fn test_every_provider_id_is_unique() {
        // Catches copy-paste additions to the PROVIDERS table.
        let mut seen: Vec<&str> = PROVIDERS.iter().map(|p| p.id).collect();
        seen.sort();
        let original_len = seen.len();
        seen.dedup();
        assert_eq!(
            seen.len(),
            original_len,
            "duplicate provider id in PROVIDERS"
        );
    }
}
