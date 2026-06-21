struct DestructivePattern {
    pattern: &'static str,
    warning: &'static str,
}

const DESTRUCTIVE_PATTERNS: &[DestructivePattern] = &[
    // Git — data loss / hard to reverse
    DestructivePattern {
        pattern: r"\bgit\s+reset\s+--hard\b",
        warning: "Note: may discard uncommitted changes",
    },
    DestructivePattern {
        pattern: r"\bgit\s+push\b[^;&|\n]*[ \t](--force|--force-with-lease|-f)\b",
        warning: "Note: may overwrite remote history",
    },
    DestructivePattern {
        pattern: r"\bgit\s+clean\b(?![^;&|\n]*(?:-[a-zA-Z]*n|--dry-run))[^;&|\n]*-[a-zA-Z]*f",
        warning: "Note: may permanently delete untracked files",
    },
    DestructivePattern {
        pattern: r"\bgit\s+checkout\s+(--\s+)?\.[ \t]*($|[;&|\n])",
        warning: "Note: may discard all working tree changes",
    },
    DestructivePattern {
        pattern: r"\bgit\s+restore\s+(--\s+)?\.[ \t]*($|[;&|\n])",
        warning: "Note: may discard all working tree changes",
    },
    DestructivePattern {
        pattern: r"\bgit\s+stash[ \t]+(drop|clear)\b",
        warning: "Note: may permanently remove stashed changes",
    },
    DestructivePattern {
        pattern: r"\bgit\s+branch\s+(-D[ \t]|--delete\s+--force|--force\s+--delete)\b",
        warning: "Note: may force-delete a branch",
    },
    // Git — safety bypass
    DestructivePattern {
        pattern: r"\bgit\s+(commit|push|merge)\b[^;&|\n]*--no-verify\b",
        warning: "Note: may skip safety hooks",
    },
    DestructivePattern {
        pattern: r"\bgit\s+commit\b[^;&|\n]*--amend\b",
        warning: "Note: may rewrite the last commit",
    },
    // File deletion
    DestructivePattern {
        pattern: r"(^|[;&|\n]\s*)rm\s+-[a-zA-Z]*[rR][a-zA-Z]*f|(^|[;&|\n]\s*)rm\s+-[a-zA-Z]*f[a-zA-Z]*[rR]",
        warning: "Note: may recursively force-remove files",
    },
    DestructivePattern {
        pattern: r"(^|[;&|\n]\s*)rm\s+-[a-zA-Z]*[rR]",
        warning: "Note: may recursively remove files",
    },
    DestructivePattern {
        pattern: r"(^|[;&|\n]\s*)rm\s+-[a-zA-Z]*f",
        warning: "Note: may force-remove files",
    },
    // Database
    DestructivePattern {
        pattern: r"\b(DROP|TRUNCATE)\s+(TABLE|DATABASE|SCHEMA)\b",
        warning: "Note: may drop or truncate database objects",
    },
    DestructivePattern {
        pattern: r#"\bDELETE\s+FROM\s+\w+[ \t]*(;|"|'|\n|$)"#,
        warning: "Note: may delete all rows from a database table",
    },
    // Infrastructure
    DestructivePattern {
        pattern: r"\bkubectl\s+delete\b",
        warning: "Note: may delete Kubernetes resources",
    },
    DestructivePattern {
        pattern: r"\bterraform\s+destroy\b",
        warning: "Note: may destroy Terraform infrastructure",
    },
    // Filesystem-level destruction (substrings, not regex)
    DestructivePattern {
        pattern: r"mkfs\b",
        warning: "Note: may format a disk/partition",
    },
    DestructivePattern {
        pattern: r"\bdd\s+if=",
        warning: "Note: may overwrite a disk/partition",
    },
    DestructivePattern {
        pattern: r":\(\)\s*\{\s*:\s*\|\s*:\s*&\s*\}\s*;\s*:",
        warning: "Note: fork-bomb pattern",
    },
    DestructivePattern {
        pattern: r"chmod\s+-r\s+777\s+/",
        warning: "Note: may make filesystem world-writable",
    },
    DestructivePattern {
        pattern: r">\s*/dev/sd",
        warning: "Note: writing to a raw block device",
    },
    DestructivePattern {
        pattern: r"\bformat\s+[a-zA-Z]:",
        warning: "Note: may format a Windows drive",
    },
    DestructivePattern {
        pattern: r"\bdel\s+/[fsq]",
        warning: "Note: may recursively delete files (Windows)",
    },
];

/// Returns a human-readable warning if the command matches a known
/// destructive pattern, otherwise `None`. The warning is purely
/// informational and is surfaced in the permission dialog.
pub fn get_warning(command: &str) -> Option<String> {
    for pat in DESTRUCTIVE_PATTERNS {
        if matches_pattern(pat.pattern, command) {
            return Some(pat.warning.to_string());
        }
    }
    None
}

/// Returns true if the command matches ANY destructive pattern. Used by
/// the decision flow to bump write commands to "ask with warning".
pub fn is_destructive(command: &str) -> bool {
    DESTRUCTIVE_PATTERNS
        .iter()
        .any(|p| matches_pattern(p.pattern, command))
}

fn matches_pattern(pattern: &str, command: &str) -> bool {
    match regex::Regex::new(pattern) {
        Ok(re) => re.is_match(command),
        Err(e) => {
            log::warn!("destructive: invalid pattern '{}': {}", pattern, e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_git_force_push() {
        let w = get_warning("git push --force origin main");
        assert!(w.is_some());
        assert!(w.unwrap().contains("remote history"));
    }

    #[test]
    fn detects_git_reset_hard() {
        assert!(is_destructive("git reset --hard HEAD~1"));
    }

    #[test]
    fn detects_rm_rf() {
        assert!(is_destructive("rm -rf /tmp/foo"));
        assert!(is_destructive("rm -fr build/"));
    }

    #[test]
    fn detects_database_drop() {
        assert!(is_destructive("DROP TABLE users"));
        assert!(is_destructive("TRUNCATE TABLE logs"));
    }

    #[test]
    fn ignores_benign_git() {
        assert!(get_warning("git status").is_none());
        assert!(get_warning("git log --oneline -10").is_none());
        assert!(get_warning("git diff").is_none());
    }

    #[test]
    fn ignores_benign_rm() {
        // rm without -r or -f is not flagged
        assert!(get_warning("rm file.txt").is_none());
    }

    #[test]
    fn detects_fork_bomb() {
        assert!(is_destructive(":(){ :|:& };:"));
    }
}
