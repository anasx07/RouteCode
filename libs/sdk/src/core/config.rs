use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicModelInfo {
    pub name: String,
    pub provider_id: String,
}

/// Strategy used to handle transient provider failures.
///
/// Tagged enum, e.g. `{"strategy": "qir"}`. The `Disabled` variant is the
/// default; switch to `Qir` for the experimental no-delay, no-limit retry
/// loop, or `ExponentialBackoff` for a bounded classic backoff (not yet
/// implemented at the call site — the orchestrator currently only honors
/// `Qir` vs not-`Qir`, so picking `ExponentialBackoff` is reserved for
/// future use).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum RetryPolicy {
    /// No automatic retry on failure. Default.
    Disabled,
    /// Quick Infinite Retry: re-send immediately with no delay and no limit.
    /// Experimental. Use at your own risk — aggressive retrying can
    /// rate-limit, suspend, or permanently ban your account with AI providers.
    Qir,
    /// Exponential backoff with a maximum attempt count.
    /// Reserved for future use.
    ExponentialBackoff {
        max_attempts: u32,
        /// Base delay between attempts, in seconds.
        base_secs: f64,
        /// Add random jitter to each delay.
        #[serde(default)]
        jitter: bool,
    },
}

/// Tool-confirmation policy. Persisted in `Config` so the user's preference
/// survives across sessions.
///
/// * `Normal` (default): every tool call from the agent shows a confirmation
///   modal. The user must explicitly allow or deny.
/// * `Yolo`: tool calls are auto-allowed without any UI prompt. Use for
///   trusted, sandboxed runs. The LLM can still ask for input via the
///   `/plan` sub-agent flow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Confirm every tool call (default).
    Normal,
    /// Auto-allow every tool call. The user is responsible for the agent's
    /// actions; nothing is sandboxed beyond what the OS already provides.
    Yolo,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy::Disabled
    }
}

impl Default for ApprovalMode {
    fn default() -> Self {
        ApprovalMode::Normal
    }
}

impl RetryPolicy {
    /// True if this policy will retry on a transient failure.
    /// `Disabled` returns false; `Qir` and `ExponentialBackoff` return true.
    pub fn is_retry_enabled(&self) -> bool {
        !matches!(self, RetryPolicy::Disabled)
    }

    /// True if this is the `Qir` policy (no delay, no limit).
    pub fn is_qir(&self) -> bool {
        matches!(self, RetryPolicy::Qir)
    }
}

impl ApprovalMode {
    /// True if tool calls should be auto-allowed without prompting.
    pub fn is_yolo(&self) -> bool {
        matches!(self, ApprovalMode::Yolo)
    }
}

/// Accepts both the new tagged shape (`{"strategy": "qir"}`) and the
/// bare-bool legacy shape (`true` / `false`). Logs a deprecation warning
/// when a bare bool is seen.
fn deserialize_retry_policy<'de, D>(d: D) -> Result<RetryPolicy, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    let v = Value::deserialize(d)?;
    match v {
        Value::Bool(true) => {
            log::warn!(
                "`retry_policy: <bool>` is deprecated; use `{{\"strategy\": \"qir\"}}` instead"
            );
            Ok(RetryPolicy::Qir)
        }
        Value::Bool(false) => Ok(RetryPolicy::Disabled),
        other => serde_json::from_value(other)
            .map_err(|e| serde::de::Error::custom(format!("invalid `retry_policy`: {e}"))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model: String,
    pub provider: String,
    pub theme: String,
    pub api_keys: HashMap<String, String>,
    #[serde(default)]
    pub allowlist: Vec<String>,
    #[serde(default)]
    pub last_update_check: f64,
    #[serde(default)]
    pub favorites: Vec<DynamicModelInfo>,
    #[serde(default)]
    pub recent_models: Vec<DynamicModelInfo>,
    #[serde(default = "default_thinking_level")]
    pub thinking_level: String,
    #[serde(default = "default_logo_animation")]
    pub logo_animation: String,
    #[serde(default = "default_logo_animation_color")]
    pub logo_animation_color: String,
    #[serde(default)]
    pub vertex_project: String,
    #[serde(default)]
    pub vertex_location: String,
    #[serde(default = "default_sub_agents_enabled")]
    pub sub_agents_enabled: bool,
    /// Retry policy for failed provider requests. Default: `Disabled`.
    #[serde(
        default = "default_retry_policy",
        deserialize_with = "deserialize_retry_policy"
    )]
    pub retry_policy: RetryPolicy,
    /// Tool-confirmation policy. Default: `Normal` (confirm every tool call).
    /// Toggle to `Yolo` to auto-allow all tool calls; the LLM can still ask
    /// for input by spawning a `/plan` sub-agent when it needs approval.
    #[serde(default)]
    pub approval_mode: ApprovalMode,
    /// DEPRECATED: superseded by `retry_policy`. If present on load, it
    /// overrides `retry_policy` (`true` → `Qir`, `false` → `Disabled`) and a
    /// deprecation warning is logged. Never serialized back out. Will be
    /// removed in a future release.
    #[serde(default, skip_serializing)]
    pub quick_infinite_retry: Option<bool>,
}

fn default_sub_agents_enabled() -> bool {
    true
}

fn default_thinking_level() -> String {
    "default".to_string()
}

fn default_logo_animation() -> String {
    "always".to_string()
}

fn default_logo_animation_color() -> String {
    "rainbow".to_string()
}

fn default_retry_policy() -> RetryPolicy {
    RetryPolicy::Disabled
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            theme: "default".to_string(),
            api_keys: HashMap::new(),
            allowlist: Vec::new(),
            last_update_check: 0.0,
            favorites: Vec::new(),
            recent_models: Vec::new(),
            thinking_level: "default".to_string(),
            logo_animation: "always".to_string(),
            logo_animation_color: "rainbow".to_string(),
            vertex_project: String::new(),
            vertex_location: "us-central1".to_string(),
            sub_agents_enabled: true,
            retry_policy: RetryPolicy::Disabled,
            quick_infinite_retry: None,
            approval_mode: ApprovalMode::Normal,
        }
    }
}

impl Config {
    pub fn get_api_key(&self) -> Option<&String> {
        self.api_keys.get(&self.provider)
    }

    /// Migrate the deprecated `quick_infinite_retry: bool` field to the
    /// new `retry_policy` enum. Logs a deprecation warning and clears the
    /// legacy field. Call this after deserialization to apply the migration.
    pub fn normalize(&mut self) {
        if let Some(b) = self.quick_infinite_retry.take() {
            log::warn!(
                "`quick_infinite_retry` is deprecated; use `retry_policy` instead. \
                 Migrating automatically: {} -> {{ \"strategy\": \"{}\" }}.",
                b,
                if b { "qir" } else { "disabled" }
            );
            self.retry_policy = if b { RetryPolicy::Qir } else { RetryPolicy::Disabled };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_policy_default_is_disabled() {
        let p: RetryPolicy = Default::default();
        assert_eq!(p, RetryPolicy::Disabled);
        assert!(!p.is_retry_enabled());
        assert!(!p.is_qir());
    }

    #[test]
    fn retry_policy_helpers() {
        assert!(!RetryPolicy::Disabled.is_retry_enabled());
        assert!(RetryPolicy::Qir.is_retry_enabled());
        assert!(RetryPolicy::Qir.is_qir());
        let eb = RetryPolicy::ExponentialBackoff { max_attempts: 5, base_secs: 1.0, jitter: true };
        assert!(eb.is_retry_enabled());
        assert!(!eb.is_qir());
    }

    #[test]
    fn retry_policy_round_trip_tagged() {
        let json = r#"{"strategy":"qir"}"#;
        let p: RetryPolicy = serde_json::from_str(json).unwrap();
        assert_eq!(p, RetryPolicy::Qir);
        let back = serde_json::to_string(&p).unwrap();
        assert_eq!(back, json);
    }

    #[test]
    fn retry_policy_deserializes_legacy_bool() {
        // The custom deserializer is invoked via `deserialize_with` from
        // Config, not by direct `from_str::<RetryPolicy>` (which would use
        // the auto-derived impl and reject bare bools).
        let mut de = serde_json::Deserializer::from_str("true");
        let p = deserialize_retry_policy(&mut de).unwrap();
        assert_eq!(p, RetryPolicy::Qir);
        let mut de = serde_json::Deserializer::from_str("false");
        let p = deserialize_retry_policy(&mut de).unwrap();
        assert_eq!(p, RetryPolicy::Disabled);
    }

    #[test]
    fn retry_policy_deserializes_exponential_backoff() {
        let json = r#"{"strategy":"exponential_backoff","max_attempts":5,"base_secs":1.5,"jitter":true}"#;
        let p: RetryPolicy = serde_json::from_str(json).unwrap();
        match p {
            RetryPolicy::ExponentialBackoff { max_attempts, base_secs, jitter } => {
                assert_eq!(max_attempts, 5);
                assert!((base_secs - 1.5).abs() < f64::EPSILON);
                assert!(jitter);
            }
            _ => panic!("expected ExponentialBackoff"),
        }
    }

    #[test]
    fn config_normalize_migrates_legacy_field() {
        let mut cfg = Config::default();
        cfg.retry_policy = RetryPolicy::Disabled;
        cfg.quick_infinite_retry = Some(true);
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
        assert!(cfg.quick_infinite_retry.is_none());

        cfg.quick_infinite_retry = Some(false);
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Disabled);
        assert!(cfg.quick_infinite_retry.is_none());
    }

    #[test]
    fn config_normalize_preserves_new_field_when_legacy_none() {
        let mut cfg = Config::default();
        cfg.retry_policy = RetryPolicy::Qir;
        cfg.quick_infinite_retry = None;
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
    }

    #[test]
    fn config_normalize_legacy_overrides_new() {
        // If both fields are present, the legacy bool takes precedence so
        // existing users with `quick_infinite_retry: true` keep their QIR.
        let mut cfg = Config::default();
        cfg.retry_policy = RetryPolicy::Disabled;
        cfg.quick_infinite_retry = Some(true);
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
    }

    #[test]
    fn config_round_trip_omits_legacy_field() {
        let mut cfg = Config::default();
        cfg.retry_policy = RetryPolicy::Qir;
        cfg.quick_infinite_retry = Some(true); // simulate stale state
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(!json.contains("quick_infinite_retry"));
        assert!(json.contains("\"retry_policy\""));
    }

    #[test]
    fn config_deserializes_legacy_top_level_bool_and_migrates() {
        // Simulates a config.json from an old build that still has
        // `quick_infinite_retry: true` at the top level (no `retry_policy`).
        let json = r#"{
            "model": "gpt-4o",
            "provider": "openai",
            "theme": "default",
            "api_keys": {},
            "quick_infinite_retry": true
        }"#;
        let mut cfg: Config = serde_json::from_str(json).unwrap();
        // The legacy field is loaded but not yet migrated.
        assert_eq!(cfg.quick_infinite_retry, Some(true));
        assert_eq!(cfg.retry_policy, RetryPolicy::Disabled);
        // normalize() migrates: legacy bool wins over missing new field.
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
        assert!(cfg.quick_infinite_retry.is_none());
    }

    #[test]
    fn config_deserializes_new_shape_without_legacy() {
        let json = r#"{
            "model": "gpt-4o",
            "provider": "openai",
            "theme": "default",
            "api_keys": {},
            "retry_policy": {"strategy": "qir"}
        }"#;
        let mut cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.quick_infinite_retry, None);
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
        // normalize() is a no-op when the new field is present and legacy is None.
        cfg.normalize();
        assert_eq!(cfg.retry_policy, RetryPolicy::Qir);
    }

    #[test]
    fn approval_mode_default_is_normal() {
        let mode: ApprovalMode = Default::default();
        assert_eq!(mode, ApprovalMode::Normal);
        assert!(!mode.is_yolo());
    }

    #[test]
    fn approval_mode_round_trip_tagged() {
        let json = r#"{"strategy":"yolo"}"#;
        let mode: ApprovalMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, ApprovalMode::Yolo);
        assert!(mode.is_yolo());
        let back = serde_json::to_string(&mode).unwrap();
        assert_eq!(back, json);
    }

    #[test]
    fn config_default_approval_mode_is_normal() {
        let cfg = Config::default();
        assert_eq!(cfg.approval_mode, ApprovalMode::Normal);
    }

    #[test]
    fn config_round_trip_preserves_approval_mode() {
        let mut cfg = Config::default();
        cfg.approval_mode = ApprovalMode::Yolo;
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"approval_mode\""));
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back.approval_mode, ApprovalMode::Yolo);
    }

    #[test]
    fn config_deserializes_legacy_config_without_approval_mode() {
        // Backward-compat: an old config.json that doesn't have
        // `approval_mode` at all should deserialize to Normal.
        let json = r#"{
            "model": "gpt-4o",
            "provider": "openai",
            "theme": "default",
            "api_keys": {}
        }"#;
        let cfg: Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.approval_mode, ApprovalMode::Normal);
    }
}
