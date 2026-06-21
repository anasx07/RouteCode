//! Hook matcher patterns.
//!
//! A matcher string controls which tool calls (for tool-related
//! events) a hook fires on. Syntax (mirrors Claude Code):
//!
//! - `*` — match all
//! - `Write|Edit|Bash` — match any of the listed tool names
//! - `Bash(git *)` — match Bash tool calls where the command starts
//!   with `git `
//! - `Read(.*\.rs)` — match Read tool calls with a regex against the
//!   path
//!
//! For non-tool events (Stop, StopFailure) the matcher is ignored;
//! the matcher string can be omitted.

use regex::Regex;

/// A parsed matcher pattern. Holds either a list of literal tool
/// names, a regex against a specific tool's input field, or a
/// wildcard.
#[derive(Debug, Clone)]
pub enum MatcherPattern {
    /// Match all
    Wildcard,
    /// Match any of these literal tool names (or regex / tool-rule
    /// patterns). The strings are matched against the tool name; the
    /// `Bash(git *)` style is encoded as `Rule { tool, input_regex }`.
    Tools(Vec<ToolMatcher>),
}

#[derive(Debug, Clone)]
pub enum ToolMatcher {
    /// Literal tool name (e.g. "Write", "Edit", "Bash").
    Name(String),
    /// `Bash(git *)` style: tool + input pattern.
    Rule {
        tool: String,
        input_pattern: InputPattern,
    },
}

#[derive(Debug, Clone)]
pub enum InputPattern {
    /// Bash-style glob (`git *`, `npm run *`). Uses the same matching
    /// as the bash allowlist (bare, `prefix:*`, multi-word).
    Glob(String),
    /// A regex (anything starting with `/` and ending with `/`, or
    /// containing regex metachars).
    Regex(Regex),
}

impl PartialEq for MatcherPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MatcherPattern::Wildcard, MatcherPattern::Wildcard) => true,
            (MatcherPattern::Tools(a), MatcherPattern::Tools(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for MatcherPattern {}

impl PartialEq for ToolMatcher {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ToolMatcher::Name(a), ToolMatcher::Name(b)) => a == b,
            (
                ToolMatcher::Rule { tool: t1, input_pattern: p1 },
                ToolMatcher::Rule { tool: t2, input_pattern: p2 },
            ) => t1 == t2 && p1 == p2,
            _ => false,
        }
    }
}

impl Eq for ToolMatcher {}

impl PartialEq for InputPattern {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputPattern::Glob(a), InputPattern::Glob(b)) => a == b,
            (InputPattern::Regex(a), InputPattern::Regex(b)) => {
                a.as_str() == b.as_str()
            }
            _ => false,
        }
    }
}

impl Eq for InputPattern {}

impl MatcherPattern {
    /// Parse a matcher string. Returns the parsed pattern or an
    /// error if the regex inside is invalid.
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let pattern = pattern.trim();
        if pattern.is_empty() || pattern == "*" {
            return Ok(MatcherPattern::Wildcard);
        }

        // Find the first `(` and matching `)`. If present, treat
        // as a tool-rule pattern.
        if let Some((tool, inner)) = split_tool_rule(pattern) {
            let input_pattern = parse_input_pattern(&inner)?;
            return Ok(MatcherPattern::Tools(vec![ToolMatcher::Rule {
                tool,
                input_pattern,
            }]));
        }

        // Otherwise: pipe-delimited list of tool names. Each name
        // can also be a literal `Bash(git *)` style which we already
        // handled above.
        let names: Vec<ToolMatcher> = pattern
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| ToolMatcher::Name(s.to_string()))
            .collect();
        if names.is_empty() {
            return Ok(MatcherPattern::Wildcard);
        }
        Ok(MatcherPattern::Tools(names))
    }

    /// Test whether the pattern matches a given tool name + tool
    /// input. `input_str` is a textual representation of the tool
    /// input (for Bash, the command string; for Read/Edit/etc, the
    /// file path).
    pub fn matches(&self, tool_name: &str, input_str: &str) -> bool {
        match self {
            MatcherPattern::Wildcard => true,
            MatcherPattern::Tools(tools) => tools.iter().any(|t| match t {
                ToolMatcher::Name(n) => n == tool_name,
                ToolMatcher::Rule { tool, input_pattern } => {
                    if tool != tool_name {
                        return false;
                    }
                    match input_pattern {
                        InputPattern::Glob(g) => glob_match(g, input_str),
                        InputPattern::Regex(r) => r.is_match(input_str),
                    }
                }
            }),
        }
    }
}

fn split_tool_rule(pattern: &str) -> Option<(String, String)> {
    let open = pattern.find('(')?;
    if !pattern.ends_with(')') {
        return None;
    }
            let tool = pattern[..open].trim().to_string();
            let inner = pattern[open + 1..pattern.len() - 1].to_string();
            Some((tool, inner))
        }

fn parse_input_pattern(inner: &str) -> Result<InputPattern, String> {
    // Slash-delimited regex: /foo/i or /foo/
    if let Some(rest) = inner.strip_prefix('/') {
        if let Some(end) = rest.rfind('/') {
            let body = &rest[..end];
            let flags = &rest[end + 1..];
            let pattern = if flags.is_empty() {
                body.to_string()
            } else {
                format!("(?{}){}", flags, body)
            };
            return Regex::new(&pattern)
                .map(InputPattern::Regex)
                .map_err(|e| format!("Invalid regex: {}", e));
        }
    }
    // Otherwise treat as a glob
    Ok(InputPattern::Glob(inner.to_string()))
}

fn glob_match(pattern: &str, s: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    // Use the `glob` crate's pattern matcher for `*`-style wildcards.
    match glob::Pattern::new(pattern) {
        Ok(p) => p.matches(s),
        Err(_) => {
            // Fall back to literal-prefix match
            if let Some(rest) = s.strip_prefix(pattern) {
                rest.is_empty() || rest.starts_with(char::is_whitespace)
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_wildcard() {
        let p = MatcherPattern::parse("*").unwrap();
        assert_eq!(p, MatcherPattern::Wildcard);
    }

    #[test]
    fn parse_pipe_list() {
        let p = MatcherPattern::parse("Write|Edit|Bash").unwrap();
        assert!(p.matches("Write", ""));
        assert!(p.matches("Edit", ""));
        assert!(p.matches("Bash", ""));
        assert!(!p.matches("Read", ""));
    }

    #[test]
    fn parse_tool_rule_glob() {
        let p = MatcherPattern::parse("Bash(git *)").unwrap();
        assert!(p.matches("Bash", "git status"));
        assert!(p.matches("Bash", "git push origin main"));
        assert!(!p.matches("Bash", "npm install"));
        assert!(!p.matches("Read", "git status"));
    }

    #[test]
    fn parse_tool_rule_multi_token_glob() {
        let p = MatcherPattern::parse("Bash(npm run *)").unwrap();
        assert!(p.matches("Bash", "npm run build"));
        assert!(p.matches("Bash", "npm run build:dev"));
        assert!(!p.matches("Bash", "npm install"));
    }

    #[test]
    fn parse_tool_rule_regex() {
        let p = MatcherPattern::parse("Read(/.*\\.rs/)").unwrap();
        assert!(p.matches("Read", "src/main.rs"));
        assert!(p.matches("Read", "/abs/path/foo.rs"));
        assert!(!p.matches("Read", "main.py"));
    }

    #[test]
    fn wildcard_matches_everything() {
        let p = MatcherPattern::parse("*").unwrap();
        assert!(p.matches("Anything", "with anything"));
    }
}
