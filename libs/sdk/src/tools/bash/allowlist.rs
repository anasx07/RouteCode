/// Returns true if `command` matches `pattern`.
///
/// Pattern syntax (matches Claude Code's bashPermissions):
/// - `*`                 → match all commands
/// - `git:*`             → wildcard prefix; matches any command starting with `git`
/// - `git`               → bare command; matches the command `git` and any
///                         `git <subcommand>...` invocation (prefix match
///                         at whitespace or end-of-string boundary)
/// - `npm run build`     → multi-word exact prefix; matches `npm run build`
///                         and any subcommand like `npm run build:dev`
pub fn matches(pattern: &str, command: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }
    if let Some(stripped) = pattern.strip_suffix(":*") {
        let base = stripped.trim();
        return first_token(command) == base;
    }

    // Direct prefix match: command must start with pattern. The next char
    // after the matched prefix must be whitespace, end-of-string, or a
    // subcommand separator (`:` or `-` glued to the last pattern token,
    // e.g. `npm run build:dev`).
    if let Some(rest) = command.strip_prefix(pattern) {
        return is_valid_boundary(rest);
    }
    false
}

fn is_valid_boundary(rest: &str) -> bool {
    if rest.is_empty() {
        return true;
    }
    let first = rest.chars().next().unwrap();
    if first.is_whitespace() {
        return true;
    }
    // Subcommand separator glued to last pattern token
    first == ':' || first == '-'
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

    #[test]
    fn star_matches_anything() {
        assert!(matches("*", "ls"));
        assert!(matches("*", "rm -rf /"));
        assert!(matches("*", ""));
    }

    #[test]
    fn wildcard_prefix_matches() {
        assert!(matches("git:*", "git status"));
        assert!(matches("git:*", "git log --oneline"));
        assert!(matches("git:*", "git"));
        assert!(!matches("git:*", "github-cli status"));
    }

    #[test]
    fn bare_command_matches_itself() {
        assert!(matches("git", "git"));
        assert!(matches("git", "git status"));
        assert!(matches("git", "git log -1"));
    }

    #[test]
    fn bare_command_does_not_match_substring() {
        assert!(!matches("git", "github-cli status"));
        assert!(!matches("git", "gitfoo"));
        assert!(!matches("git", "gits status"));
    }

    #[test]
    fn multi_word_prefix() {
        assert!(matches("npm run build", "npm run build"));
        assert!(matches("npm run build", "npm run build:dev"));
        assert!(matches("npm run build", "npm run build --watch"));
        assert!(matches("npm run build", "npm run build-prod"));
        // `npm run dev` is NOT a prefix of `npm run build`
        assert!(!matches("npm run dev", "npm run build"));
        // But `npm run build` is a prefix of `npm run buildX` (with the `X`
        // glued on without separator)
        assert!(!matches("npm run build", "npm run buildX"));
    }

    #[test]
    fn empty_pattern_matches_nothing() {
        assert!(!matches("", "ls"));
        assert!(!matches("", ""));
    }
}
