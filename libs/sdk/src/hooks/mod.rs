//! Hooks system for RouteCode.
//!
//! Mirrors Claude Code's hook model: shell commands and Rust
//! callbacks can intercept the agent loop at 5 lifecycle events
//! (PreToolUse, PostToolUse, PostToolUseFailure, Stop, StopFailure).
//!
//! ## Quick start
//!
//! ```no_run
//! use routecode_sdk::hooks::{HookRegistry, HookEvent, HookInput, CommandHook, ShellKind};
//!
//! let mut registry = HookRegistry::load();
//!
//! // Register a runtime callback (e.g. for logging)
//! struct MyLogger;
//! #[async_trait::async_trait]
//! impl routecode_sdk::hooks::HookCallback for MyLogger {
//!     fn name(&self) -> &str { "logger" }
//!     async fn run(&self, _input: &HookInput) -> anyhow::Result<routecode_sdk::hooks::HookOutput> {
//!         Ok(routecode_sdk::hooks::HookOutput::ok())
//!     }
//! }
//! registry.register_callback(std::sync::Arc::new(MyLogger));
//! ```
//!
//! Hooks are configured via `~/.routecode/settings.json` (user) and
//! `./.routecode/settings.json` (per-project). On first encounter
//! of project hooks, the user is prompted to trust them.

pub mod aggregate;
pub mod events;
pub mod input;
pub mod matcher;
pub mod output;
pub mod registry;
pub mod runner;
pub mod types;

pub use events::HookEvent;
pub use input::{
    BaseHookInput, HookInput, PostToolUseFailureInput, PostToolUseInput,
    PreToolUseInput, StopFailureInput, StopInput,
};
pub use matcher::MatcherPattern;
pub use output::{HookOutcome, HookOutput, HookSpecificOutput, PreToolUseDecision};
pub use registry::{HookRegistry, SettingsFile, TrustFile};
pub use aggregate::{aggregate_results, Aggregated};
pub use runner::{run_hook, run_hooks_for_event, HookExecutionError};

/// One entry in a `RequestHookTrust` summary. Re-exported as
/// `crate::hooks::HookTrustEntry` for use in the orchestrator's
/// `StreamChunk::RequestHookTrust` chunk.
pub use crate::agents::types::HookTrustEntry;
pub use types::{
    BoxedCallback, CommandHook, CommandHookKind, HookCallback, HookEntry,
    HookEntryCallback, HooksConfig, ShellKind,
};
