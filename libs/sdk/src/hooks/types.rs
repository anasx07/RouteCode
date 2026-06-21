use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use super::events::HookEvent;
use super::input::HookInput;
use super::output::HookOutput;

/// A shell-command hook. Spawns a process, pipes the hook input
/// JSON to stdin, reads JSON output from stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandHook {
    /// Always `Command`. The outer `HookEntry` enum already
    /// discriminates by `type`, so this is a marker field.
    #[serde(default, skip_serializing_if = "is_command_kind")]
    pub kind: CommandHookKind,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub if_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub shell: Option<ShellKind>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub once: Option<bool>,
    /// If true, run in the background. The hook output is not
    /// awaited; its effects apply to the NEXT event the orchestrator
    /// processes (e.g. an async PostToolUse can `updatedMCPToolOutput`
    /// for the following tool call).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub async_: Option<bool>,
}

fn is_command_kind(k: &CommandHookKind) -> bool {
    matches!(k, CommandHookKind::Command)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommandHookKind {
    #[default]
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellKind {
    Bash,
    Sh,
    #[cfg(windows)]
    Cmd,
    #[cfg(windows)]
    Powershell,
}

impl ShellKind {
    pub fn default_for_platform() -> Self {
        if cfg!(windows) {
            ShellKind::Cmd
        } else {
            ShellKind::Bash
        }
    }

    pub fn program(&self) -> &'static str {
        match self {
            ShellKind::Bash => "bash",
            ShellKind::Sh => "sh",
            #[cfg(windows)]
            ShellKind::Cmd => "cmd",
            #[cfg(windows)]
            ShellKind::Powershell => "powershell",
        }
    }
}

impl Default for CommandHook {
    fn default() -> Self {
        Self {
            kind: CommandHookKind::Command,
            command: String::new(),
            if_: None,
            timeout: Some(60),
            shell: None,
            status_message: None,
            once: None,
            async_: None,
        }
    }
}

/// A Rust-trait callback hook. Used for internal SDK hooks (logging,
/// attribution, future plugin system). NOT persisted to disk — added
/// programmatically via `HookRegistry::register_callback`.
#[async_trait]
pub trait HookCallback: Send + Sync {
    /// A human-readable name (e.g. "log-tool-use", "analytics").
    fn name(&self) -> &str;
    /// Run the hook. Return a HookOutput. Returning `HookOutput::ok()`
    /// is the "no opinion" case.
    async fn run(
        &self,
        input: &HookInput,
    ) -> Result<HookOutput, anyhow::Error>;
}

/// A boxed callback hook. The trait object is wrapped in Arc so it
/// can be cloned cheaply across the registry.
pub type BoxedCallback = Arc<dyn HookCallback>;

/// A matcher + its hooks. Mirrors `HookMatcherSchema` in Claude
/// Code.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookMatcherConfig {
    /// Optional matcher pattern. For tool events, controls which
    /// tools fire the hook. For non-tool events (Stop, StopFailure),
    /// the matcher is ignored.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub matcher: Option<String>,
    /// List of hooks to run when the matcher matches.
    #[serde(default)]
    pub hooks: Vec<HookEntry>,
}

/// One hook entry. A settings file can mix `CommandHook` entries
/// (serialized) and `Callback` entries (programmatic, never
/// serialized). The runtime representation is `HookEntry::Command`
/// (serializable). Callbacks are stored in a separate map on the
/// registry and are looked up by name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HookEntry {
    Command(CommandHook),
}

/// In-memory-only wrapper for a callback hook. Stored in the
/// registry's separate `callbacks` map.
pub struct HookEntryCallback(pub BoxedCallback);

impl std::fmt::Debug for HookEntryCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookEntryCallback")
            .field("name", &self.0.name())
            .finish()
    }
}

/// Top-level hooks configuration. Mirrors Claude Code's
/// `HooksSchema`: a map from event name to list of matchers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HooksConfig {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub hooks: HashMap<HookEvent, Vec<HookMatcherConfig>>,
}

impl HooksConfig {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Get the matchers for an event, or an empty slice.
    pub fn matchers_for(&self, event: HookEvent) -> &[HookMatcherConfig] {
        self.hooks.get(&event).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Merge another config on top of this one. Used to combine
    /// user-level + per-project configs (project wins for
    /// overlapping entries).
    pub fn merged_with(&self, other: &HooksConfig) -> HooksConfig {
        let mut out = self.clone();
        for (event, matchers) in &other.hooks {
            let entry = out.hooks.entry(*event).or_default();
            // For Phase 1, project matchers REPLACE user matchers for
            // the same event. A more sophisticated merge could
            // concatenate; Claude Code does a per-matcher merge.
            // Replacement is the safe default and easier to reason
            // about for the user ("my project hook takes priority").
            *entry = matchers.clone();
        }
        out
    }
}

/// Path resolution for hook config files. Centralized here so the
/// CLI and orchestrator can agree on locations.
pub fn user_settings_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    Some(home.join(".routecode").join("settings.json"))
}

pub fn project_settings_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    Some(cwd.join(".routecode").join("settings.json"))
}

pub fn project_trust_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    Some(cwd.join(".routecode").join("trusted_hooks.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trip_json() {
        let mut hooks = HashMap::new();
        hooks.insert(
            HookEvent::PreToolUse,
            vec![HookMatcherConfig {
                matcher: Some("Bash".into()),
                hooks: vec![HookEntry::Command(CommandHook {
                    kind: CommandHookKind::Command,
                    command: "jq -e .".into(),
                    timeout: Some(5),
                    ..Default::default()
                })],
            }],
        );
        let cfg = HooksConfig { hooks };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: HooksConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn merged_with_replaces_per_event() {
        let mut user_hooks = HashMap::new();
        user_hooks.insert(HookEvent::Stop, vec![HookMatcherConfig::default()]);
        let user = HooksConfig { hooks: user_hooks };

        let mut proj_hooks = HashMap::new();
        proj_hooks.insert(
            HookEvent::Stop,
            vec![HookMatcherConfig {
                matcher: Some("Write".into()),
                ..Default::default()
            }],
        );
        let proj = HooksConfig { hooks: proj_hooks };

        let merged = user.merged_with(&proj);
        // Project replaces user for the same event
        assert_eq!(
            merged.matchers_for(HookEvent::Stop)[0].matcher,
            Some("Write".into())
        );
    }

    #[test]
    fn merged_with_preserves_disjoint_events() {
        let mut user_hooks = HashMap::new();
        user_hooks.insert(HookEvent::Stop, vec![HookMatcherConfig::default()]);
        let user = HooksConfig { hooks: user_hooks };

        let mut proj_hooks = HashMap::new();
        proj_hooks.insert(HookEvent::PreToolUse, vec![HookMatcherConfig::default()]);
        let proj = HooksConfig { hooks: proj_hooks };

        let merged = user.merged_with(&proj);
        assert_eq!(merged.matchers_for(HookEvent::Stop).len(), 1);
        assert_eq!(merged.matchers_for(HookEvent::PreToolUse).len(), 1);
    }
}
