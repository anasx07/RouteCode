use super::validation;
use crate::core::{Config, ToolResult};
use crate::core::config::ApprovalMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Allow,
    AllowIfReadOnly,
    Deny,
}

pub fn check(command: &str, config: &Config) -> Permission {
    if validation::is_destructive(command) && !is_yolo(config) {
        return Permission::AllowIfReadOnly;
    }
    Permission::Allow
}

pub fn is_yolo(config: &Config) -> bool {
    config.approval_mode == ApprovalMode::Yolo
}

pub fn read_only_violation(command: &str) -> Option<String> {
    if !validation::is_destructive(command) {
        return None;
    }
    let trimmed = command.trim_start();
    Some(format!(
        "Command '{}' is classified as destructive and is not allowed in read-only mode. \
        Use a sandboxed session or enable session commands to run it.",
        trimmed.chars().take(80).collect::<String>()
    ))
}

pub fn build_denial_result(message: impl Into<String>) -> ToolResult {
    ToolResult::error(message)
}
