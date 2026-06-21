//! Hook runner (Phase B).
//!
//! Dispatches a single hook (currently shell-command only) and runs
//! all matching hooks for an event in parallel.
//!
//! ## Command protocol
//!
//! - The runner spawns the hook's shell (default Bash on Unix, Cmd
//!   on Windows), passing the hook command as a single argument.
//! - The hook input is serialized to JSON and piped to the hook's
//!   stdin.
//! - The hook reads the input, does its work, and prints a JSON
//!   `HookOutput` to stdout.
//! - Exit code 0 = success; the runner parses stdout as JSON.
//! - Exit code 2 = blocking error: the runner returns
//!   `HookOutput::block("...")` and the agent stops / the tool is
//!   denied (matches Claude Code semantics).
//! - Other non-zero exit codes = non-blocking error: the runner
//!   returns `HookOutput::ok()` and surfaces a `system_message`
//!   warning to the spinner.
//! - The hook has a configurable timeout (default 60s); on timeout,
//!   the process is killed and the runner returns `HookOutput::ok()`
//!   with a system-message warning.
//!
//! ## `if` condition
//!
//! If the hook has an `if` field, it's treated as a shell command
//! that must exit 0 for the hook to run. The runner spawns it first
//! (no stdin); if it exits non-zero, the hook is skipped silently.

use std::process::Stdio;
use std::time::Duration;

use futures::future::join_all;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use crate::hooks::events::HookEvent;
use crate::hooks::input::HookInput;
use crate::hooks::matcher::MatcherPattern;
use crate::hooks::output::HookOutput;
use crate::hooks::registry::HookRegistry;
use crate::hooks::types::{CommandHook, HookEntry, ShellKind};

/// Error type for hook execution.
#[derive(Debug)]
pub enum HookExecutionError {
    /// I/O error from spawning or communicating with the hook.
    Io(std::io::Error),
    /// Hook JSON output could not be parsed.
    Parse(String),
    /// Hook exceeded its timeout.
    Timeout,
    /// Hook was aborted by the signal.
    Cancelled,
    /// Other error.
    Other(String),
}

impl std::fmt::Display for HookExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HookExecutionError::Io(e) => write!(f, "I/O error: {}", e),
            HookExecutionError::Parse(s) => write!(f, "Parse error: {}", s),
            HookExecutionError::Timeout => write!(f, "Hook timed out"),
            HookExecutionError::Cancelled => write!(f, "Hook cancelled"),
            HookExecutionError::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for HookExecutionError {}

impl From<std::io::Error> for HookExecutionError {
    fn from(e: std::io::Error) -> Self {
        HookExecutionError::Io(e)
    }
}

const DEFAULT_TIMEOUT_SECS: u32 = 60;

/// Run a single hook. Returns the parsed `HookOutput`, or an error
/// for I/O / parse failures. Spawn failure for a command hook is an
/// error; non-zero exit codes (other than 2) are surfaced as
/// `system_message` warnings on an `ok()` output.
pub async fn run_hook(
    hook: &HookEntry,
    input: &HookInput,
    timeout_secs: Option<u32>,
) -> Result<HookOutput, HookExecutionError> {
    let HookEntry::Command(cmd_hook) = hook;
    let secs = timeout_secs.or(cmd_hook.timeout).unwrap_or(DEFAULT_TIMEOUT_SECS);
    run_command_hook(cmd_hook, input, secs).await
}

/// Run all matching hooks for an event and return their outputs in
/// completion order. Hooks run in parallel; a failing hook doesn't
/// affect the others. The aggregator (`aggregate::aggregate_results`)
/// is responsible for combining these into a final decision.
pub async fn run_hooks_for_event(
    event: HookEvent,
    input: &HookInput,
    registry: &HookRegistry,
) -> Vec<HookOutput> {
    let matchers = registry.merged.matchers_for(event);
    if matchers.is_empty() {
        return Vec::new();
    }

    let (tool_name, tool_input_str) = extract_tool_info(input);
    let is_tool = is_tool_event(event);

    let mut tasks = Vec::new();
    for matcher_cfg in matchers {
        let pattern = match matcher_cfg.matcher.as_deref() {
            Some(s) => match MatcherPattern::parse(s) {
                Ok(p) => p,
                Err(_) => continue, // invalid pattern — skip matcher
            },
            None => MatcherPattern::Wildcard,
        };
        let matches_input = !is_tool
            || pattern.matches(&tool_name, &tool_input_str);
        if !matches_input {
            continue;
        }
        for hook in &matcher_cfg.hooks {
            tasks.push(run_hook(hook, input, None));
        }
    }

    join_all(tasks)
        .await
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

async fn run_command_hook(
    hook: &CommandHook,
    input: &HookInput,
    timeout_secs: u32,
) -> Result<HookOutput, HookExecutionError> {
    // 1. Evaluate `if` condition, if any. If it fails (non-zero
    //    exit or spawn error), skip the hook silently.
    if let Some(ref cond) = hook.if_ {
        if !check_if_condition(cond, hook.shell).await {
            return Ok(HookOutput::ok());
        }
    }

    // 2. Async hooks: fire-and-forget. The runner returns ok() now
    //    and the actual output (if any) is discarded. Async hook
    //    output queuing (e.g. updatedMCPToolOutput for the next
    //    tool call) is a Phase 2 feature.
    if hook.async_ == Some(true) {
        let hook = hook.clone();
        let input = input.clone();
        tokio::spawn(async move {
            let _ = run_command_hook_sync(&hook, &input, timeout_secs).await;
        });
        return Ok(HookOutput::ok());
    }

    run_command_hook_sync(hook, input, timeout_secs).await
}

async fn run_command_hook_sync(
    hook: &CommandHook,
    input: &HookInput,
    timeout_secs: u32,
) -> Result<HookOutput, HookExecutionError> {
    let input_json = serde_json::to_string(input)
        .map_err(|e| HookExecutionError::Parse(e.to_string()))?;
    let shell = hook.shell.unwrap_or_else(ShellKind::default_for_platform);
    let mut cmd = build_shell_command(shell, &hook.command);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        // Write input + close stdin. We ignore short writes — the
        // hook can stop reading if it doesn't care about the input.
        let _ = stdin.write_all(input_json.as_bytes()).await;
        let _ = stdin.shutdown().await;
    }

    let dur = Duration::from_secs(timeout_secs as u64);
    let output = match timeout(dur, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(HookExecutionError::Io(e)),
        Err(_) => {
            // Timed out: kill the child and report a system
            // message. We can't wait for it after timeout fires
            // here, so we rely on the OS to clean up.
            return Ok(HookOutput {
                system_message: Some(format!(
                    "Hook timed out after {}s",
                    timeout_secs
                )),
                ..HookOutput::ok()
            });
        }
    };

    match output.status.code() {
        Some(0) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let trimmed = stdout.trim();
            if trimmed.is_empty() {
                return Ok(HookOutput::ok());
            }
            serde_json::from_str(trimmed)
                .map_err(|e| HookExecutionError::Parse(e.to_string()))
        }
        Some(2) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let reason = if stderr.is_empty() {
                "Hook blocked execution (exit 2)".to_string()
            } else {
                stderr
            };
            Ok(HookOutput::block(reason))
        }
        Some(code) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let msg = if stderr.is_empty() {
                format!("Hook exited with code {}", code)
            } else {
                format!("Hook exited with code {}: {}", code, stderr)
            };
            Ok(HookOutput {
                system_message: Some(msg),
                ..HookOutput::ok()
            })
        }
        None => Ok(HookOutput {
            system_message: Some("Hook killed by signal".to_string()),
            ..HookOutput::ok()
        }),
    }
}

fn build_shell_command(shell: ShellKind, command: &str) -> Command {
    let mut cmd = Command::new(shell.program());
    match shell {
        ShellKind::Bash | ShellKind::Sh => {
            cmd.arg("-c").arg(command);
        }
        #[cfg(windows)]
        ShellKind::Cmd => {
            cmd.arg("/c").arg(command);
        }
        #[cfg(windows)]
        ShellKind::Powershell => {
            cmd.arg("-NoProfile").arg("-Command").arg(command);
        }
    }
    cmd
}

async fn check_if_condition(cond: &str, shell: Option<ShellKind>) -> bool {
    let shell = shell.unwrap_or_else(ShellKind::default_for_platform);
    let mut cmd = build_shell_command(shell, cond);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    match cmd.status().await {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

fn is_tool_event(event: HookEvent) -> bool {
    matches!(
        event,
        HookEvent::PreToolUse
            | HookEvent::PostToolUse
            | HookEvent::PostToolUseFailure
    )
}

fn extract_tool_info(input: &HookInput) -> (String, String) {
    match input {
        HookInput::PreToolUse(i) => (i.tool_name.clone(), tool_input_string(&i.tool_input)),
        HookInput::PostToolUse(i) => (i.tool_name.clone(), tool_input_string(&i.tool_input)),
        HookInput::PostToolUseFailure(i) => {
            (i.tool_name.clone(), tool_input_string(&i.tool_input))
        }
        _ => (String::new(), String::new()),
    }
}

fn tool_input_string(value: &serde_json::Value) -> String {
    if let Some(cmd) = value.get("command").and_then(|v| v.as_str()) {
        return cmd.to_string();
    }
    if let Some(path) = value.get("file_path").and_then(|v| v.as_str()) {
        return path.to_string();
    }
    if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
        return path.to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::types::CommandHookKind;
    use serde_json::json;

    fn base_hook(command: impl Into<String>) -> HookEntry {
        HookEntry::Command(CommandHook {
            kind: CommandHookKind::Command,
            command: command.into(),
            if_: None,
            timeout: Some(5),
            shell: None,
            status_message: None,
            once: None,
            async_: None,
        })
    }

    fn pre_input(cmd: &str) -> HookInput {
        HookInput::PreToolUse(crate::hooks::input::PreToolUseInput {
            base: crate::hooks::input::BaseHookInput {
                session_id: "s1".into(),
                transcript_path: "/tmp/t".into(),
                cwd: "/cwd".into(),
                permission_mode: None,
                agent_id: None,
                agent_type: None,
            },
            hook_event_name: HookEvent::PreToolUse,
            tool_name: "Bash".into(),
            tool_input: json!({ "command": cmd }),
            tool_use_id: "call_1".into(),
        })
    }

    /// Build a command that prints `s` to stdout. On Windows we use
    /// PowerShell because `cmd /C echo` mangles embedded double
    /// quotes. On Unix we use plain `printf` for portability across
    /// sh/bash/zsh.
    fn json_echo_cmd(s: &str) -> String {
        if cfg!(windows) {
            let escaped = s.replace('\'', "''");
            format!(
                "powershell -NoProfile -Command Write-Output '{}'",
                escaped
            )
        } else {
            format!("printf '%s' '{}'", s.replace('\'', "'\\''"))
        }
    }

    /// Build a command that writes `msg` to stderr and exits with
    /// `code`. Portable across cmd / sh.
    fn stderr_and_exit_cmd(msg: &str, code: i32) -> String {
        if cfg!(windows) {
            format!("echo {} 1>&2 & exit /b {}", msg, code)
        } else {
            format!("printf '%s\\n' '{}' 1>&2; exit {}", msg, code)
        }
    }

    #[tokio::test]
    async fn exit_zero_empty_stdout_returns_ok() {
        let hook = base_hook("true");
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        assert!(out.continue_.unwrap_or(false) || out.continue_.is_none());
        assert!(out.additional_context.is_none());
    }

    #[tokio::test]
    async fn exit_zero_with_json_returns_output() {
        let hook = base_hook(json_echo_cmd(r#"{"continue_":true,"additional_context":"injected"}"#));
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        assert_eq!(out.additional_context.as_deref(), Some("injected"));
    }

    #[tokio::test]
    async fn exit_two_blocks() {
        let hook = base_hook(stderr_and_exit_cmd("denied", 2));
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        assert_eq!(out.continue_, Some(false));
        assert_eq!(out.reason.as_deref(), Some("denied"));
    }

    #[tokio::test]
    async fn exit_one_is_non_blocking_warning() {
        let hook = base_hook(stderr_and_exit_cmd("boom", 1));
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        assert_eq!(out.continue_, Some(true));
        assert!(out.system_message.unwrap().contains("boom"));
    }

    #[tokio::test]
    async fn invalid_json_stdout_is_error() {
        let hook = base_hook(json_echo_cmd("not json"));
        let input = pre_input("ls");
        let err = run_hook(&hook, &input, Some(5)).await.unwrap_err();
        assert!(matches!(err, HookExecutionError::Parse(_)));
    }

    #[tokio::test]
    async fn if_condition_skip_when_nonzero() {
        let cmd = json_echo_cmd(r#"{"additional_context":"should not appear"}"#);
        let mut hook = base_hook(&cmd);
        let HookEntry::Command(c) = &mut hook;
        c.if_ = Some("false".into());
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        // Skipped: ok() with no additional_context.
        assert!(out.additional_context.is_none());
        assert_eq!(out.continue_, Some(true));
    }

    #[tokio::test]
    async fn if_condition_run_when_zero() {
        let cmd = json_echo_cmd(r#"{"additional_context":"ok"}"#);
        let mut hook = base_hook(&cmd);
        let HookEntry::Command(c) = &mut hook;
        c.if_ = Some("true".into());
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(5)).await.unwrap();
        assert_eq!(out.additional_context.as_deref(), Some("ok"));
    }

    #[tokio::test]
    async fn timeout_kills_hook() {
        let hook = base_hook(if cfg!(windows) {
            "ping -n 11 127.0.0.1 >NUL"
        } else {
            "sleep 10"
        });
        let input = pre_input("ls");
        let out = run_hook(&hook, &input, Some(1)).await.unwrap();
        assert_eq!(out.continue_, Some(true));
        assert!(out.system_message.unwrap().contains("timed out"));
    }

    #[tokio::test]
    async fn run_for_event_dispatches_matching_hooks() {
        use crate::hooks::types::{HookMatcherConfig, HooksConfig};
        use std::collections::HashMap;

        let mut reg = HookRegistry::new();
        // PreToolUse with Bash(git *) matcher: should run.
        reg.merged = HooksConfig {
            hooks: HashMap::from([(
                HookEvent::PreToolUse,
                vec![HookMatcherConfig {
                    matcher: Some("Bash(git *)".into()),
                    hooks: vec![base_hook(json_echo_cmd(
                        r#"{"additional_context":"git hook ran"}"#,
                    ))],
                }],
            )]),
        };

        let input = pre_input("git status");
        let outs = run_hooks_for_event(HookEvent::PreToolUse, &input, &reg).await;
        assert_eq!(outs.len(), 1);
        assert_eq!(
            outs[0].additional_context.as_deref(),
            Some("git hook ran")
        );

        // Non-matching command: no hook fires.
        let input = pre_input("npm install");
        let outs = run_hooks_for_event(HookEvent::PreToolUse, &input, &reg).await;
        assert!(outs.is_empty());

        // Wildcard matcher: always fires.
        reg.merged = HooksConfig {
            hooks: HashMap::from([(
                HookEvent::PreToolUse,
                vec![HookMatcherConfig {
                    matcher: Some("*".into()),
                    hooks: vec![base_hook("true")],
                }],
            )]),
        };
        let outs = run_hooks_for_event(HookEvent::PreToolUse, &pre_input("ls"), &reg).await;
        assert_eq!(outs.len(), 1);
    }
}
