use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::events::HookEvent;
use super::types::{
    user_settings_path, HookEntry, HooksConfig,
};

/// On-disk format for a single project settings file. Mirrors Claude
/// Code's `Settings` schema's `hooks` sub-object.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SettingsFile {
    #[serde(default)]
    pub hooks: HooksConfig,
}

/// Trust file: a JSON list of (project_path_hash, hook_signature)
/// pairs the user has previously approved. Stored at
/// `.routecode/trusted_hooks.json` in the project root.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustFile {
    /// Project paths whose hooks the user has trusted.
    #[serde(default)]
    pub trusted_projects: HashSet<String>,
}

impl TrustFile {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }
}

/// The full set of hook sources: user-level, per-project, and
/// runtime callbacks.
#[derive(Default)]
pub struct HookRegistry {
    pub user_config: HooksConfig,
    pub project_config: HooksConfig,
    /// The merged config (user + project), cached.
    pub merged: HooksConfig,
    /// Runtime callback hooks (not from disk). Keyed by name for
    /// easy removal.
    pub callbacks: std::collections::HashMap<String, super::types::BoxedCallback>,
    /// Trust state. None = not loaded; Some = loaded from disk.
    trust: Option<TrustFile>,
    project_root: Option<PathBuf>,
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("user_config", &self.user_config)
            .field("project_config", &self.project_config)
            .field("merged", &self.merged)
            .field("callbacks", &self.callbacks.keys().collect::<Vec<_>>())
            .field("trust", &self.trust.as_ref().map(|t| t.trusted_projects.len()))
            .field("project_root", &self.project_root)
            .finish()
    }
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load the registry from default locations: user
    /// `~/.routecode/settings.json` and project
    /// `./.routecode/settings.json`. If the project has hooks that
    /// haven't been trusted, `load()` returns a registry with the
    /// project config cleared and a `pending_trust_request()` that
    /// the caller can present to the user.
    pub fn load() -> Self {
        Self::load_at(project_root())
    }

    /// Load with an explicit project root. Used by tests and by the
    /// orchestrator when CWD is not the project root.
    pub fn load_at(project_root_path: Option<PathBuf>) -> Self {
        let user_config = user_settings_path()
            .as_ref()
            .and_then(|p| load_settings_file(p).ok())
            .map(|s| s.hooks)
            .unwrap_or_default();
        let project_root = project_root_path
            .clone()
            .or_else(|| std::env::current_dir().ok());
        let project_settings = project_root
            .as_ref()
            .map(|r| r.join(".routecode").join("settings.json"));
        let _ = project_settings; // (re-)derived on each call via self method
        let trust_path = project_root
            .as_ref()
            .map(|r| r.join(".routecode").join("trusted_hooks.json"));
        let trust = trust_path.as_ref().map(|p| TrustFile::load(p));

        let mut reg = Self {
            user_config,
            project_config: HooksConfig::empty(),
            merged: HooksConfig::empty(),
            callbacks: Default::default(),
            trust,
            project_root,
        };
        reg.reload_project();
        reg
    }

    /// Re-read project config and trust. Call this after the user
    /// approves or denies a trust prompt.
    pub fn reload_project(&mut self) {
        if let Some(path) = self.project_settings_path() {
            if let Ok(file) = load_settings_file(&path) {
                let project_id = project_signature(&path);
                let trusted = self
                    .trust
                    .as_ref()
                    .map(|t| t.trusted_projects.contains(&project_id))
                    .unwrap_or(false);
                if trusted || file.hooks.hooks.is_empty() {
                    self.project_config = file.hooks;
                } else {
                    // Not trusted and has hooks: skip project hooks
                    // for now. The orchestrator should detect this
                    // and send a trust prompt.
                    self.project_config = HooksConfig::empty();
                }
            }
        }
        self.merged = self.user_config.merged_with(&self.project_config);
    }

    /// Mark the current project's hooks as trusted and reload.
    pub fn trust_project(&mut self) {
        let Some(path) = self.project_settings_path() else {
            return;
        };
        let project_id = project_signature(&path);
        let trust_path = self.project_trust_path();
        let mut trust = self
            .trust
            .clone()
            .unwrap_or_default();
        trust.trusted_projects.insert(project_id);
        if let Some(ref tp) = trust_path {
            let _ = trust.save(tp);
        }
        self.trust = Some(trust);
        self.reload_project();
    }

    /// True if the current project has hooks that need user trust
    /// approval before they can run.
    pub fn needs_trust_approval(&self) -> bool {
        let Some(path) = self.project_settings_path() else {
            return false;
        };
        let Ok(file) = load_settings_file(&path) else {
            return false;
        };
        if file.hooks.hooks.is_empty() {
            return false;
        }
        let project_id = project_signature(&path);
        self.trust
            .as_ref()
            .map(|t| !t.trusted_projects.contains(&project_id))
            .unwrap_or(true)
    }

    /// Register a runtime callback hook.
    pub fn register_callback(&mut self, cb: super::types::BoxedCallback) {
        self.callbacks.insert(cb.name().to_string(), cb);
    }

    /// Compute a signature for a project based on its absolute path
    /// + the file mtime. When the settings file changes, the
    /// signature changes too, and the user must re-approve.
    pub fn pending_trust_signature(&self) -> Option<String> {
        self.project_settings_path()
            .map(|p| project_signature(&p))
    }

    /// Summary of the hooks the project wants to register, for the
    /// trust prompt. Each entry is the event name + matcher + a
    /// human-readable description of the hook.
    pub fn pending_trust_summary(&self) -> Vec<(HookEvent, String, String)> {
        let Some(path) = self.project_settings_path() else {
            return Vec::new();
        };
        let Ok(file) = load_settings_file(&path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for (event, matchers) in &file.hooks.hooks {
            for matcher in matchers {
                let matcher_str = matcher.matcher.clone().unwrap_or_else(|| "*".into());
                for hook in &matcher.hooks {
                    let desc = match hook {
                        HookEntry::Command(c) => {
                            format!("command: {}", c.command)
                        }
                    };
                    out.push((*event, matcher_str.clone(), desc));
                }
            }
        }
        out
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    fn project_settings_path(&self) -> Option<PathBuf> {
        Some(self.project_root.as_ref()?.join(".routecode").join("settings.json"))
    }

    fn project_trust_path(&self) -> Option<PathBuf> {
        Some(self.project_root.as_ref()?.join(".routecode").join("trusted_hooks.json"))
    }
}

/// Resolve the project root: the current working directory.
fn project_root() -> Option<PathBuf> {
    std::env::current_dir().ok()
}

/// Load and parse a settings file. Missing file is treated as
/// empty config (not an error).
fn load_settings_file(path: &Path) -> std::io::Result<SettingsFile> {
    let s = std::fs::read_to_string(path)?;
    serde_json::from_str(&s).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid settings.json: {}", e),
        )
    })
}

/// A stable signature for a project's settings file. Uses the
/// absolute path + mtime so editing the file invalidates trust.
fn project_signature(path: &Path) -> String {
    let abs = std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    let mtime = std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}@{}", abs, mtime)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn write_settings(dir: &Path, hooks: &HooksConfig) {
        let settings_dir = dir.join(".routecode");
        std::fs::create_dir_all(&settings_dir).unwrap();
        let file = SettingsFile { hooks: hooks.clone() };
        std::fs::write(
            settings_dir.join("settings.json"),
            serde_json::to_string(&file).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn trust_flow_lifecycle() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().canonicalize().unwrap();
        write_settings(
            &root,
            &HooksConfig {
                hooks: HashMap::from([(
                    HookEvent::PreToolUse,
                    vec![super::super::types::HookMatcherConfig {
                        matcher: Some("Bash".into()),
                        hooks: vec![],
                    }],
                )]),
            },
        );
        let mut reg = HookRegistry::load_at(Some(root.clone()));
        // Project has hooks but isn't trusted: registry should have
        // empty project_config
        assert!(reg.project_config.hooks.is_empty());
        assert!(reg.needs_trust_approval());
        // Approve trust
        reg.trust_project();
        assert!(!reg.needs_trust_approval());
        assert!(!reg.project_config.hooks.is_empty());
    }

    #[test]
    fn no_settings_file_means_no_hooks() {
        let dir = TempDir::new().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let reg = HookRegistry::load_at(Some(root));
        assert!(!reg.needs_trust_approval());
        assert!(reg.merged.hooks.is_empty());
    }
}
