use crate::core::config::Config;

use super::allowlist;
use super::decision::Decision;
use super::destructive;
use super::readonly;

/// Evaluate a bash command against the user's config and return a
/// `Decision` describing what the bash tool should do.
///
/// Flow (in order; first match wins):
/// 1. Denylist        → always Deny
/// 2. ReadOnly mode   → hard Deny on write/destructive/unknown commands
/// 3. AcceptEdits     → auto-Allow filesystem-mutating commands
/// 4. Allowlist       → Allow (skips prompt)
/// 5. Yolo            → Allow everything
/// 6. Read-only cmd   → Allow
/// 7. Destructive cmd → Ask with warning
/// 8. Default         → Ask
pub fn evaluate(command: &str, config: &Config) -> Decision {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Decision::deny("Empty command");
    }

    // 1. Denylist — overrides everything
    for pattern in &config.denylist {
        if allowlist::matches(pattern, trimmed) {
            return Decision::deny(format!(
                "Command matches denylist pattern '{}'",
                pattern
            ));
        }
    }

    // 2. ReadOnly mode — hard deny on anything non-read-only
    if config.bash_mode.is_read_only() {
        if readonly::is_read_only(trimmed) {
            return Decision::allow("Read-only command in read-only mode");
        }
        return Decision::deny(format!(
            "Bash mode is read-only; '{}' is not a read-only command",
            first_token(trimmed)
        ))
        .with_suggestion("Switch bash_mode to Default or AcceptEdits to allow writes");
    }

    // 3. AcceptEdits — auto-allow filesystem-mutating commands
    if config.bash_mode.is_accept_edits() && readonly::is_filesystem_command(trimmed) {
        return Decision::allow("Filesystem-mutating command allowed in accept-edits mode");
    }

    // 4. Allowlist — explicit allow
    for pattern in &config.allowlist {
        if allowlist::matches(pattern, trimmed) {
            return Decision::allow(format!("Matches allowlist pattern '{}'", pattern));
        }
    }

    // 5. Yolo — auto-allow everything
    if config.approval_mode.is_yolo() {
        return Decision::allow("Approval mode is Yolo");
    }

    // 6. Read-only command — no prompt
    if readonly::is_read_only(trimmed) {
        return Decision::allow("Read-only command");
    }

    // 7. Destructive command — ask with warning
    if let Some(warning) = destructive::get_warning(trimmed) {
        return Decision::ask_with_warning(
            "Command is potentially destructive",
            warning,
        );
    }

    // 8. Default — ask
    Decision::ask(format!("Execute: {}", trimmed))
}

fn first_token(command: &str) -> &str {
    command
        .split_whitespace()
        .next()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{ApprovalMode, BashMode, Config};
    use crate::tools::bash::decision::Behavior;

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn denylist_overrides_allowlist() {
        let mut c = cfg();
        c.allowlist = vec!["git".to_string()];
        c.denylist = vec!["git push".to_string()];
        let d = evaluate("git push", &c);
        assert_eq!(d.behavior, Behavior::Deny);
        assert!(d.reason.contains("denylist"));
    }

    #[test]
    fn readonly_mode_denies_writes() {
        let mut c = cfg();
        c.bash_mode = BashMode::ReadOnly;
        let d = evaluate("rm file.txt", &c);
        assert_eq!(d.behavior, Behavior::Deny);
    }

    #[test]
    fn readonly_mode_allows_reads() {
        let mut c = cfg();
        c.bash_mode = BashMode::ReadOnly;
        let d = evaluate("ls -la", &c);
        assert_eq!(d.behavior, Behavior::Allow);
    }

    #[test]
    fn readonly_mode_allows_git_readonly() {
        let mut c = cfg();
        c.bash_mode = BashMode::ReadOnly;
        assert_eq!(
            evaluate("git status", &c).behavior,
            Behavior::Allow
        );
        assert_eq!(
            evaluate("git log", &c).behavior,
            Behavior::Allow
        );
    }

    #[test]
    fn readonly_mode_denies_git_commit() {
        let mut c = cfg();
        c.bash_mode = BashMode::ReadOnly;
        let d = evaluate("git commit -m 'msg'", &c);
        assert_eq!(d.behavior, Behavior::Deny);
    }

    #[test]
    fn accept_edits_allows_mkdir() {
        let mut c = cfg();
        c.bash_mode = BashMode::AcceptEdits;
        let d = evaluate("mkdir -p foo/bar", &c);
        assert_eq!(d.behavior, Behavior::Allow);
    }

    #[test]
    fn accept_edits_does_not_allow_unrelated() {
        let mut c = cfg();
        c.bash_mode = BashMode::AcceptEdits;
        let d = evaluate("npm install", &c);
        assert_eq!(d.behavior, Behavior::Ask);
    }

    #[test]
    fn allowlist_skips_prompt() {
        let mut c = cfg();
        c.allowlist = vec!["git:*".to_string()];
        let d = evaluate("git push", &c);
        assert_eq!(d.behavior, Behavior::Allow);
    }

    #[test]
    fn yolo_allows_everything() {
        let mut c = cfg();
        c.approval_mode = ApprovalMode::Yolo;
        let d = evaluate("rm -rf /", &c);
        assert_eq!(d.behavior, Behavior::Allow);
    }

    #[test]
    fn read_only_command_allowed() {
        let c = cfg();
        let d = evaluate("cat file.txt", &c);
        assert_eq!(d.behavior, Behavior::Allow);
    }

    #[test]
    fn destructive_asks_with_warning() {
        let c = cfg();
        let d = evaluate("git push --force origin main", &c);
        assert_eq!(d.behavior, Behavior::Ask);
        assert!(d.warning.is_some());
        assert!(d.warning.unwrap().contains("remote"));
    }

    #[test]
    fn default_asks() {
        let c = cfg();
        let d = evaluate("npm install", &c);
        assert_eq!(d.behavior, Behavior::Ask);
        assert!(d.warning.is_none());
    }

    #[test]
    fn empty_command_is_denied() {
        let c = cfg();
        let d = evaluate("", &c);
        assert_eq!(d.behavior, Behavior::Deny);
    }
}
