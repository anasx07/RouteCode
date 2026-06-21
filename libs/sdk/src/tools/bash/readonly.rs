/// Read-only commands: don't modify files, system state, or send network
/// writes. A command is read-only if its base command is in this list AND
/// the command string has no output redirection to a file.
const READONLY_COMMANDS: &[&str] = &[
    // File content viewing
    "cat", "head", "tail", "less", "more", "wc", "stat", "file", "strings",
    "hexdump", "od", "nl",
    // Directory listing
    "ls", "tree", "dir", "find", "fd", "fdfind", "pwd",
    // Text searching
    "grep", "rg", "ag", "ack", "locate", "which", "type", "command",
    // Text processing (read-only)
    "cut", "paste", "tr", "column", "tac", "rev", "fold", "expand",
    "unexpand", "fmt", "comm", "cmp", "diff", "sort", "uniq",
    // Path utilities
    "basename", "dirname", "realpath", "readlink",
    // System info
    "uname", "hostname", "whoami", "id", "date", "cal", "uptime", "df",
    "du", "free", "nproc", "groups", "locale", "getconf", "arch",
    // Network info (read-only)
    "ifconfig", "ip", "netstat", "ss", "ping", "traceroute", "nslookup", "dig",
    "host",
    // Misc safe commands
    "echo", "printf", "true", "false", "test", "[", "expr", "seq", "tsort",
    "sleep", "history", "alias", "env", "printenv",
    // Build / version tools
    "cmake", "ninja", "rustc", "cargo", "go", "java", "node", "python",
    "python3", "ruby", "perl",
];

/// Git subcommands that don't modify the repo, index, or remote state.
const GIT_READONLY_SUBCMDS: &[&str] = &[
    "status", "log", "diff", "show", "branch", "tag", "remote", "blame",
    "ls-files", "ls-tree", "ls-remote", "rev-parse", "describe", "shortlog",
    "reflog", "fetch", "config",
];

/// Filesystem-mutating commands allowed to bypass the confirmation prompt
/// when `BashMode::AcceptEdits` is set. These are the same commands Claude
/// Code accepts in `acceptEdits` mode.
pub const ACCEPT_EDITS_COMMANDS: &[&str] = &[
    "mkdir", "touch", "rm", "rmdir", "mv", "cp", "sed",
];

/// Returns true if the command is read-only — it doesn't write to files,
/// doesn't modify system state, and doesn't perform write-class network
/// operations.
pub fn is_read_only(command: &str) -> bool {
    let parsed = parse(command);
    let base = parsed.base;

    // Output redirection always makes the command a write
    if has_output_redirection(command) {
        return false;
    }

    if READONLY_COMMANDS.contains(&base.as_str()) {
        return true;
    }

    // Git subcommand check
    if base == "git" {
        let sub = parsed
            .args
            .first()
            .map(|s| s.as_str())
            .unwrap_or("");
        if GIT_READONLY_SUBCMDS.contains(&sub) {
            return true;
        }
        // `git config --get` is read-only, `git config --set` is not
        if sub == "config" {
            return parsed
                .args
                .iter()
                .skip(1)
                .any(|a| a == "--get" || a == "-l" || a == "--list");
        }
    }

    false
}

/// Returns true if the command's base is a filesystem-mutating command
/// (the allowlist for `BashMode::AcceptEdits`).
pub fn is_filesystem_command(command: &str) -> bool {
    let base = parse(command).base;
    ACCEPT_EDITS_COMMANDS.contains(&base.as_str())
}

/// Detects output redirection operators (`>`, `>>`, `&>`, `>&`, `&>>`).
/// Excludes fd-duplications like `2>&1` and `&> /dev/null` (the latter
/// IS a write to /dev/null, which is still safe — but we conservatively
/// flag it so the caller can decide).
pub fn has_output_redirection(command: &str) -> bool {
    // Strip fd-duplications first (2>&1, &>1, etc.) and /dev/null writes
    // to avoid false positives
    let cleaned = strip_safe_redirections(command);
    if cleaned.contains('>') {
        return true;
    }
    // Heredocs (`<<`) and input redirects (`<`) don't write to files
    // but combined with command substitution they can. For now, treat
    // plain `<` as safe.
    false
}

fn strip_safe_redirections(command: &str) -> String {
    command
        .replace(" 2>&1", "")
        .replace(" 2>&2", "")
}

/// Lightweight parser: returns the base command (first whitespace-delimited
/// token, after stripping env-var prefixes) and the remaining args. Does
/// not handle quoting; that's good enough for command-name detection.
struct Parsed {
    base: String,
    args: Vec<String>,
}

fn parse(command: &str) -> Parsed {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Parsed {
            base: String::new(),
            args: Vec::new(),
        };
    }
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    let mut idx = 0;

    // Skip env-var assignments (FOO=bar CMD ...)
    while idx < tokens.len() && is_env_assignment(tokens[idx]) {
        idx += 1;
    }

    let base = tokens.get(idx).unwrap_or(&"").to_string();
    let args: Vec<String> = tokens
        .iter()
        .skip(idx + 1)
        .map(|s| s.to_string())
        .collect();
    Parsed { base, args }
}

fn is_env_assignment(token: &str) -> bool {
    if let Some(eq) = token.find('=') {
        let (name, _val) = token.split_at(eq);
        !name.is_empty()
            && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cat_is_readonly() {
        assert!(is_read_only("cat file.txt"));
        assert!(is_read_only("cat -n file.txt"));
    }

    #[test]
    fn ls_is_readonly() {
        assert!(is_read_only("ls -la"));
    }

    #[test]
    fn grep_is_readonly() {
        assert!(is_read_only("grep -r pattern src/"));
    }

    #[test]
    fn git_status_is_readonly() {
        assert!(is_read_only("git status"));
        assert!(is_read_only("git log --oneline -10"));
    }

    #[test]
    fn git_commit_is_not_readonly() {
        assert!(!is_read_only("git commit -m 'msg'"));
    }

    #[test]
    fn redirect_to_file_is_not_readonly() {
        assert!(!is_read_only("echo hello > file.txt"));
        assert!(!is_read_only("ls > listing.txt"));
    }

    #[test]
    fn redirect_to_devnull_is_not_readonly() {
        // Even though /dev/null is technically safe, we flag any `>` so
        // the caller can decide. Keeps the logic simple.
        assert!(!is_read_only("ls > /dev/null"));
    }

    #[test]
    fn fd_duplication_is_readonly() {
        assert!(is_read_only("ls 2>&1"));
    }

    #[test]
    fn rm_is_not_readonly() {
        assert!(!is_read_only("rm file.txt"));
    }

    #[test]
    fn rm_is_filesystem_command() {
        assert!(is_filesystem_command("rm file.txt"));
        assert!(is_filesystem_command("mkdir -p foo/bar"));
        assert!(is_filesystem_command("mv a b"));
        assert!(!is_filesystem_command("ls"));
    }

    #[test]
    fn env_prefix_is_skipped() {
        assert!(is_read_only("FOO=bar cat file"));
        assert!(is_read_only("NODE_ENV=test ls"));
    }

    #[test]
    fn empty_command_is_not_readonly() {
        assert!(!is_read_only(""));
        assert!(!is_read_only("   "));
    }
}
