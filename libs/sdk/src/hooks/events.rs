use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// The 5 hook events supported in v1. Mirrors Claude Code's event
/// names for settings-file compatibility (a user can copy a hook
/// block from a Claude Code project and it'll parse the same way).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    Stop,
    StopFailure,
}

impl HookEvent {
    pub const ALL: &'static [HookEvent] = &[
        HookEvent::PreToolUse,
        HookEvent::PostToolUse,
        HookEvent::PostToolUseFailure,
        HookEvent::Stop,
        HookEvent::StopFailure,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::PostToolUseFailure => "PostToolUseFailure",
            HookEvent::Stop => "Stop",
            HookEvent::StopFailure => "StopFailure",
        }
    }

    /// True if this event fires BEFORE the action it intercepts.
    /// Used to decide whether `decision:block` actually prevents
    /// execution.
    pub fn is_pre_event(&self) -> bool {
        matches!(self, HookEvent::PreToolUse)
    }
}

impl fmt::Display for HookEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HookEvent {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PreToolUse" => Ok(HookEvent::PreToolUse),
            "PostToolUse" => Ok(HookEvent::PostToolUse),
            "PostToolUseFailure" => Ok(HookEvent::PostToolUseFailure),
            "Stop" => Ok(HookEvent::Stop),
            "StopFailure" => Ok(HookEvent::StopFailure),
            other => Err(format!("Unknown hook event: {}", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_strings() {
        for e in HookEvent::ALL {
            assert_eq!(HookEvent::from_str(e.as_str()).unwrap(), *e);
        }
    }

    #[test]
    fn unknown_event_errors() {
        assert!(HookEvent::from_str("PreCompact").is_err());
    }

    #[test]
    fn pre_event_classification() {
        assert!(HookEvent::PreToolUse.is_pre_event());
        assert!(!HookEvent::PostToolUse.is_pre_event());
        assert!(!HookEvent::Stop.is_pre_event());
    }
}
