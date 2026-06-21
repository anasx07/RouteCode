/// What the bash tool should do with a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Behavior {
    /// Execute the command without asking the user.
    Allow,
    /// Ask the user for permission before executing.
    Ask,
    /// Refuse to execute; return the reason to the model as a tool error.
    Deny,
}

/// The decision returned by `BashTool::evaluate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub behavior: Behavior,
    /// Why this decision was made. Returned to the user in the prompt and
    /// (for `Deny`) to the model in the tool error.
    pub reason: String,
    /// Optional human-readable warning shown alongside an `Ask` prompt.
    /// Typically set for destructive commands.
    pub warning: Option<String>,
    /// Optional suggested safer alternative shown in the prompt.
    pub suggestions: Vec<String>,
}

impl Decision {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            behavior: Behavior::Allow,
            reason: reason.into(),
            warning: None,
            suggestions: Vec::new(),
        }
    }

    pub fn ask(reason: impl Into<String>) -> Self {
        Self {
            behavior: Behavior::Ask,
            reason: reason.into(),
            warning: None,
            suggestions: Vec::new(),
        }
    }

    pub fn ask_with_warning(reason: impl Into<String>, warning: impl Into<String>) -> Self {
        Self {
            behavior: Behavior::Ask,
            reason: reason.into(),
            warning: Some(warning.into()),
            suggestions: Vec::new(),
        }
    }

    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            behavior: Behavior::Deny,
            reason: reason.into(),
            warning: None,
            suggestions: Vec::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_set_fields() {
        let d = Decision::allow("ok");
        assert_eq!(d.behavior, Behavior::Allow);
        assert_eq!(d.reason, "ok");
        assert!(d.warning.is_none());

        let d = Decision::ask_with_warning("git push", "may overwrite remote");
        assert_eq!(d.behavior, Behavior::Ask);
        assert_eq!(d.warning.as_deref(), Some("may overwrite remote"));

        let d = Decision::deny("read-only mode").with_suggestion("git status");
        assert_eq!(d.behavior, Behavior::Deny);
        assert_eq!(d.suggestions, vec!["git status".to_string()]);
    }
}
