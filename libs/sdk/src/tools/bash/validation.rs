use anyhow::Result;
use serde_json::Value;

pub fn parse_command(args: &Value) -> Result<&str> {
    args["command"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing command"))
}

pub fn is_valid(command: &str) -> bool {
    !command.trim().is_empty()
}

pub fn is_destructive(command: &str) -> bool {
    let lowered = command.to_ascii_lowercase();
    let destructive_patterns = [
        "rm -rf",
        "rm -fr",
        "del /f",
        "del /s",
        "format ",
        "mkfs",
        "dd if=",
        ":(){:|:&};:",
        "drop database",
        "drop table",
        "truncate table",
        "git push --force",
        "git push -f",
        "> /dev/sd",
        "chmod -r 777 /",
    ];
    destructive_patterns
        .iter()
        .any(|p| lowered.contains(p))
}
