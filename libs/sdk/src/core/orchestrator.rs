use crate::agents::types::StreamChunk;
use crate::agents::AIProvider;
use crate::core::{Config, Message};
use crate::hooks::{
    aggregate_results, run_hooks_for_event, HookEvent, HookInput,
    HookRegistry, HookTrustEntry,
};
use crate::tools::ToolRegistry;
use crate::utils::costs::Usage;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Pre-fire result for PreToolUse. Either the call is blocked
/// (with a reason) or it proceeds with possibly-mutated args +
/// additional context.
#[derive(Debug, Clone)]
enum PreFire {
    Ok(PreFireOk),
    Blocked(String),
}

#[derive(Debug, Clone)]
struct PreFireOk {
    args: serde_json::Value,
    additional_context: Option<String>,
}

#[derive(Debug, Clone)]
enum PostFire {
    Continue(PostFireOk),
    Stop(Option<String>),
}

#[derive(Debug, Clone)]
struct PostFireOk {
    tool_response: serde_json::Value,
    system_message: Option<String>,
}

/// Emit a `HookResult` chunk to the UI for the given aggregated
/// output. Used for `PreToolUse`, `PostToolUse`,
/// `PostToolUseFailure`, `Stop`, and `StopFailure`.
fn emit_hook_result_chunks(
    tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    event: HookEvent,
    name: &str,
    agg: &crate::hooks::Aggregated,
) {
    let Some(sender) = tx else { return };
    let _ = sender.send(StreamChunk::HookResult {
        event: event.as_str().to_string(),
        name: name.to_string(),
        decision: match agg.decision {
            crate::hooks::PreToolUseDecision::Approve => None,
            crate::hooks::PreToolUseDecision::Block => Some("block".to_string()),
        },
        reason: agg.reason.clone(),
        additional_context: agg.additional_context.clone(),
        system_message: agg.system_message.clone(),
    });
    if let Some(msg) = &agg.system_message {
        let _ = sender.send(StreamChunk::Status { content: msg.clone() });
    }
}

impl HookInput {
    /// Extract the `tool_input` from a `PreToolUse` variant. Used
    /// when the aggregated output didn't override the input; we
    /// want to pass the original input through unchanged.
    fn tool_input_snapshot(&self) -> serde_json::Value {
        match self {
            HookInput::PreToolUse(i) => i.tool_input.clone(),
            HookInput::PostToolUse(i) => i.tool_input.clone(),
            HookInput::PostToolUseFailure(i) => i.tool_input.clone(),
            _ => serde_json::Value::Null,
        }
    }
}

pub struct AgentOrchestrator {
    provider: Mutex<Arc<dyn AIProvider>>,
    tool_registry: Arc<ToolRegistry>,
    pub config: Arc<Mutex<Config>>,
    pub usage: Arc<Mutex<Usage>>,
    pub allow_session_commands: std::sync::atomic::AtomicBool,
    pub allow_session_outside_access: std::sync::atomic::AtomicBool,
    /// True when the AI is in plan mode: only read-only tools are exposed
    /// in the schema and bash is constrained to read-only commands. Set
    /// by `enter_plan_mode` tool or by the UI Shift+Tab Plan toggle.
    pub is_in_plan_mode: std::sync::atomic::AtomicBool,
    /// True when the user has approved a plan and unlocked write tools
    /// for the rest of this session. Once set, plan mode stays off until
    /// the user re-enters it explicitly.
    pub plan_mode_session_unlocked: std::sync::atomic::AtomicBool,
    /// Stable identifier for this orchestrator's session. Used to scope
    /// persisted plan files (`~/.routecode/plans/{session_id}/plan-N.md`)
    /// and as a key for any future per-session state. Auto-generated as
    /// a UUID-like string on creation; callers can override.
    pub session_id: String,
    /// Hook registry (user + project hooks). Shared so the CLI can
    /// observe / register runtime callbacks on the same instance.
    pub hook_registry: Arc<Mutex<HookRegistry>>,
}

impl AgentOrchestrator {
    pub fn new(
        provider: Arc<dyn AIProvider>,
        tool_registry: Arc<ToolRegistry>,
        config: Arc<Mutex<Config>>,
    ) -> Self {
        let project_root = std::env::current_dir().ok();
        Self {
            provider: Mutex::new(provider),
            tool_registry,
            config,
            usage: Arc::new(Mutex::new(Usage::default())),
            allow_session_commands: std::sync::atomic::AtomicBool::new(false),
            allow_session_outside_access: std::sync::atomic::AtomicBool::new(false),
            is_in_plan_mode: std::sync::atomic::AtomicBool::new(false),
            plan_mode_session_unlocked: std::sync::atomic::AtomicBool::new(false),
            session_id: generate_session_id(),
            hook_registry: Arc::new(Mutex::new(HookRegistry::load_at(project_root))),
        }
    }

    /// Enter plan mode. Idempotent. Called by the `enter_plan_mode` tool
    /// or by the UI's Shift+Tab toggle.
    pub fn enter_plan_mode(&self) {
        use std::sync::atomic::Ordering;
        self.is_in_plan_mode.store(true, Ordering::SeqCst);
    }

    /// Exit plan mode. If `unlock` is true, also unlocks write tools for
    /// the rest of the session.
    pub fn exit_plan_mode(&self, unlock: bool) {
        use std::sync::atomic::Ordering;
        self.is_in_plan_mode.store(false, Ordering::SeqCst);
        if unlock {
            self.plan_mode_session_unlocked
                .store(true, Ordering::SeqCst);
        }
    }

    /// If the current project has hooks that haven't been trusted,
    /// send a `RequestHookTrust` chunk to the UI and await the
    /// user's response. On trust, the registry is updated and
    /// future calls will see the project hooks. On deny, the
    /// registry stays empty for this session.
    ///
    /// Safe to call multiple times; the trust file is the source of
    /// truth so a second call after trust will be a no-op.
    pub async fn ensure_project_hooks_trusted(
        &self,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) {
        let needs_trust = {
            let reg = self.hook_registry.lock().await;
            reg.needs_trust_approval()
        };
        if !needs_trust {
            return;
        }
        let Some(sender) = tx else {
            // No UI: silently leave the project hooks untrusted. The
            // session will run without them.
            return;
        };
        let summary = {
            let reg = self.hook_registry.lock().await;
            reg.pending_trust_summary()
        };
        let signature = {
            let reg = self.hook_registry.lock().await;
            reg.pending_trust_signature()
                .unwrap_or_else(|| "unknown".to_string())
        };
        let project_path = {
            let reg = self.hook_registry.lock().await;
            reg.project_root()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        };
        let entries: Vec<HookTrustEntry> = summary
            .into_iter()
            .map(|(event, matcher, description)| HookTrustEntry {
                event: event.as_str().to_string(),
                matcher,
                description,
            })
            .collect();
        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
        let tx_wrapped = Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));
        if let Err(e) = sender.send(StreamChunk::RequestHookTrust {
            project_signature: signature,
            project_path,
            hooks: entries,
            tx: Some(tx_wrapped),
        }) {
            log::error!("Failed to send RequestHookTrust to UI: {}", e);
            return;
        }
        match oneshot_rx.await {
            Ok(crate::agents::types::HookTrustResponse::Trust) => {
                let mut reg = self.hook_registry.lock().await;
                reg.trust_project();
            }
            Ok(crate::agents::types::HookTrustResponse::Deny) => {
                // Leave the registry empty for this session.
            }
            Err(_) => {
                log::warn!(
                    "Hook trust channel closed without a response; \
                     project hooks will be skipped for this session."
                );
            }
        }
    }

    /// Build the `HookInput` for a `PreToolUse` hook from the
    /// current session + tool call.
    fn build_pre_input(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_use_id: &str,
    ) -> HookInput {
        HookInput::PreToolUse(crate::hooks::input::PreToolUseInput {
            base: self.build_base_input(),
            hook_event_name: HookEvent::PreToolUse,
            tool_name: tool_name.to_string(),
            tool_input,
            tool_use_id: tool_use_id.to_string(),
        })
    }

    /// Build the `HookInput` for a `PostToolUse` hook.
    fn build_post_input(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_response: serde_json::Value,
        tool_use_id: &str,
    ) -> HookInput {
        HookInput::PostToolUse(crate::hooks::input::PostToolUseInput {
            base: self.build_base_input(),
            hook_event_name: HookEvent::PostToolUse,
            tool_name: tool_name.to_string(),
            tool_input,
            tool_response,
            tool_use_id: tool_use_id.to_string(),
        })
    }

    /// Build the `HookInput` for a `PostToolUseFailure` hook.
    fn build_failure_input(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_use_id: &str,
        error: &str,
    ) -> HookInput {
        HookInput::PostToolUseFailure(
            crate::hooks::input::PostToolUseFailureInput {
                base: self.build_base_input(),
                hook_event_name: HookEvent::PostToolUseFailure,
                tool_name: tool_name.to_string(),
                tool_input,
                tool_use_id: tool_use_id.to_string(),
                error: error.to_string(),
                is_interrupt: None,
            },
        )
    }

    /// Build the `HookInput` for a `Stop` hook. Captures the last
    /// assistant message from the history snapshot.
    fn build_stop_input(&self, last_assistant: Option<String>) -> HookInput {
        HookInput::Stop(crate::hooks::input::StopInput {
            base: self.build_base_input(),
            hook_event_name: HookEvent::Stop,
            stop_hook_active: false,
            last_assistant_message: last_assistant,
        })
    }

    /// Build the `HookInput` for a `StopFailure` hook.
    fn build_stop_failure_input(
        &self,
        error: &str,
        last_assistant: Option<String>,
    ) -> HookInput {
        HookInput::StopFailure(crate::hooks::input::StopFailureInput {
            base: self.build_base_input(),
            hook_event_name: HookEvent::StopFailure,
            error: error.to_string(),
            error_details: None,
            last_assistant_message: last_assistant,
        })
    }

    fn build_base_input(&self) -> crate::hooks::input::BaseHookInput {
        crate::hooks::input::BaseHookInput {
            session_id: self.session_id.clone(),
            transcript_path: String::new(),
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default(),
            permission_mode: {
                use std::sync::atomic::Ordering;
                if self.is_in_plan_mode.load(Ordering::SeqCst) {
                    Some("plan".to_string())
                } else {
                    Some("default".to_string())
                }
            },
            agent_id: None,
            agent_type: None,
        }
    }

    /// Fire the PreToolUse hooks for a tool call. Returns:
    /// - `Ok((args, output))` where `output.additional_context` /
    ///   `output.updated_input` have been applied; `output.should_block()`
    ///   indicates the call was denied.
    /// - `Err(reason)` if the hook machinery itself failed.
    async fn fire_pre_tool_use(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_use_id: &str,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> PreFire {
        let input = self.build_pre_input(tool_name, tool_input, tool_use_id);
        let merged = {
            let reg = self.hook_registry.lock().await;
            run_hooks_for_event(HookEvent::PreToolUse, &input, &reg).await
        };
        let agg = aggregate_results(merged);
        emit_hook_result_chunks(
            tx,
            HookEvent::PreToolUse,
            tool_name,
            &agg,
        );
        if agg.should_block() {
            return PreFire::Blocked(agg.reason.unwrap_or_else(|| "denied".into()));
        }
        let new_args = agg
            .updated_input
            .clone()
            .unwrap_or_else(|| input.tool_input_snapshot());
        PreFire::Ok(PreFireOk {
            args: new_args,
            additional_context: agg.additional_context.clone(),
        })
    }

    /// Fire the PostToolUse hooks. If the aggregated output contains
    /// `additional_context`, prepend it to the tool response so the
    /// model sees it.
    async fn fire_post_tool_use(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_response: serde_json::Value,
        tool_use_id: &str,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> PostFire {
        let input = self.build_post_input(
            tool_name,
            tool_input,
            tool_response.clone(),
            tool_use_id,
        );
        let merged = {
            let reg = self.hook_registry.lock().await;
            run_hooks_for_event(HookEvent::PostToolUse, &input, &reg).await
        };
        let agg = aggregate_results(merged);
        emit_hook_result_chunks(
            tx,
            HookEvent::PostToolUse,
            tool_name,
            &agg,
        );
        if agg.should_stop() {
            return PostFire::Stop(agg.stop_reason.clone());
        }
        let combined = if let Some(ctx) = agg.additional_context.as_deref() {
            let mut s = String::new();
            s.push_str(ctx);
            s.push_str("\n\n");
            let resp_str = match &tool_response {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            s.push_str(&resp_str);
            serde_json::Value::String(s)
        } else {
            tool_response.clone()
        };
        let mcp_override = agg.updated_mcp_tool_output.clone();
        PostFire::Continue(PostFireOk {
            tool_response: mcp_override.unwrap_or(combined),
            system_message: agg.system_message.clone(),
        })
    }

    /// Fire the PostToolUseFailure hooks. Used when a tool returns
    /// an error or is denied.
    async fn fire_post_tool_use_failure(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        tool_use_id: &str,
        error: &str,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) {
        let input = self.build_failure_input(
            tool_name,
            tool_input,
            tool_use_id,
            error,
        );
        let merged = {
            let reg = self.hook_registry.lock().await;
            run_hooks_for_event(HookEvent::PostToolUseFailure, &input, &reg).await
        };
        let agg = aggregate_results(merged);
        emit_hook_result_chunks(
            tx,
            HookEvent::PostToolUseFailure,
            tool_name,
            &agg,
        );
    }

    /// Fire the Stop hook. Returns the stop reason if the
    /// aggregated output said `continue: false`. Currently we
    /// don't surface that as a hard stop — the turn has already
    /// finished — but the hook can still log / inject context
    /// for the next turn.
    async fn fire_stop(
        &self,
        last_assistant: Option<String>,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Option<String> {
        let input = self.build_stop_input(last_assistant);
        let merged = {
            let reg = self.hook_registry.lock().await;
            run_hooks_for_event(HookEvent::Stop, &input, &reg).await
        };
        let agg = aggregate_results(merged);
        emit_hook_result_chunks(tx, HookEvent::Stop, "session", &agg);
        if agg.should_stop() {
            agg.stop_reason.clone()
        } else {
            None
        }
    }

    /// Fire the StopFailure hook.
    async fn fire_stop_failure(
        &self,
        error: &str,
        last_assistant: Option<String>,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) {
        let input = self.build_stop_failure_input(error, last_assistant);
        let merged = {
            let reg = self.hook_registry.lock().await;
            run_hooks_for_event(HookEvent::StopFailure, &input, &reg).await
        };
        let agg = aggregate_results(merged);
        emit_hook_result_chunks(tx, HookEvent::StopFailure, "session", &agg);
    }

    /// Handle the AI's `exit_plan_mode` tool call.
    ///
    /// Reads the latest plan markdown from disk for this session, sends
    /// a `RequestPlanApproval` chunk to the UI, and awaits the user's
    /// response. Returns:
    /// - `Ok(true)`  — user approved AND unlocked the session
    /// - `Ok(false)` — user approved without unlocking (rare; treat as
    ///                 "go ahead for one step")
    /// - `Err(feedback)` — user denied; `feedback` is what to surface
    ///                      to the model for revision
    async fn handle_exit_plan_mode(
        &self,
        args: &serde_json::Value,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<bool, String> {
        let (plan_path, plan_content) =
            match crate::tools::plan::read_latest_plan(&self.session_id) {
                Ok(Some(pair)) => pair,
                Ok(None) => {
                    let note = args
                        .get("plan")
                        .and_then(|v| v.as_str())
                        .unwrap_or(
                            "(No plan file found. The AI did not persist a \
                             plan markdown during plan mode.)",
                        );
                    (std::path::PathBuf::from("(none)"), note.to_string())
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to read plan file: {}",
                        e
                    ));
                }
            };

        let allowed_prompts: Vec<crate::agents::types::AllowedPrompt> = args
            .get("allowedPrompts")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        let tool = item
                            .get("tool")
                            .and_then(|v| v.as_str())?;
                        let prompt = item
                            .get("prompt")
                            .and_then(|v| v.as_str())?;
                        Some(crate::agents::types::AllowedPrompt {
                            tool: tool.to_string(),
                            prompt: prompt.to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let Some(sender) = tx else {
            return Err(
                "Cannot exit plan mode without a UI to approve the plan."
                    .to_string(),
            );
        };

        let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
        let tx_wrapped = Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));
        if let Err(e) = sender.send(StreamChunk::RequestPlanApproval {
            plan: plan_content,
            plan_path: plan_path.to_string_lossy().to_string(),
            allowed_prompts,
            tx: Some(tx_wrapped),
        }) {
            log::error!("Failed to send RequestPlanApproval to UI: {}", e);
            return Err("Failed to send plan approval prompt to UI.".to_string());
        }

        match oneshot_rx.await {
            Ok(crate::agents::types::PlanApprovalResponse::ApproveAndUnlock) => {
                Ok(true)
            }
            Ok(crate::agents::types::PlanApprovalResponse::ApproveOnce) => {
                Ok(false)
            }
            Ok(crate::agents::types::PlanApprovalResponse::Deny) => Err(
                "Plan denied by user. Stay in plan mode and revise."
                    .to_string(),
            ),
            Ok(crate::agents::types::PlanApprovalResponse::Feedback(msg)) => {
                Err(format!("Plan denied: {}", msg))
            }
            Err(_) => Err(
                "Plan approval cancelled (confirmation channel closed)."
                    .to_string(),
            ),
        }
    }

    pub async fn get_provider_name(&self) -> String {
        let p = self.provider.lock().await;
        p.name().to_string()
    }

    pub async fn change_provider(&self, new_provider: Arc<dyn AIProvider>) {
        let mut p = self.provider.lock().await;
        *p = new_provider;
    }

    async fn prepare_system_prompt(&self) -> Message {
        let mut system_content = String::from(
            "You are RouteCode, a senior software engineer AI coding assistant. \
            You help users work with their codebase through a terminal interface.\n\
            \n\
            # Tool use\n\
            - You have access to tools for file operations, navigation, and bash commands.\n\
            - Read files before modifying them. Use the smallest tool that solves the task.\n\
            - When a tool fails, diagnose the root cause before retrying; do not blindly re-run.\n\
            - Chain tools deliberately: if one tool's output determines the next call, do not speculatively parallelize.\n\
            \n\
            # Plan mode\n\
            - For non-trivial implementation tasks (new features, multi-file changes, architectural decisions), \
            consider using `enter_plan_mode` first. In plan mode you can explore with read-only tools and \
            design an approach, then call `exit_plan_mode` to present the plan for user approval. Once the user \
            approves, write tools are unlocked for the rest of the session.\n\
            - Skip plan mode for simple tasks: single-line fixes, clear bug fixes, tasks with specific user instructions.\n\
            \n\
            # Response style\n\
            - Be concise. Prefer short, direct answers over long preambles.\n\
            - When you create or modify a file, briefly explain why in the chat.\n\
            - Show final code in the chat (not only via tool calls) so the user can review without expanding the file.\n\
            - Use markdown sparingly: fenced code blocks for code, bullet lists for 3+ items, plain text otherwise.\n\
            \n\
            # Language\n\
            - Reply in the same language the user uses. If the user writes in Spanish, respond in Spanish; \
            if in Japanese, respond in Japanese. Default to English only when the user writes in English.\n\
            - Keep code, identifiers, file paths, and error messages in their original language (usually English) \
            regardless of your response language.\n\
            \n\
            # Anticipate edge cases\n\
            - Before answering, consider what the user may not have thought of: empty inputs, null values, \
            error states, boundary conditions, concurrency, large inputs, encoding, locale, permissions.\n\
            - If an edge case could break the user's code or change behavior, mention it briefly with a one-line fix.\n\
            - Do not pad answers with exhaustive edge-case lists when the question is simple. Mention only what matters.\n\
            \n\
            # Safety\n\
            - Never run destructive commands (rm -rf, force push, dropping databases, etc.) without explicit user confirmation.\n\
            - Never exfiltrate secrets, API keys, or PII from the workspace.\n\
            - If a request is ambiguous or potentially harmful, ask one short clarifying question before acting.\n\
            - The user's README.md and ROUTECODE.md are appended below. Follow any project-specific rules in them \
            as if they were part of this prompt.\n\
            - Project conventions (libraries, code style, file layout) take precedence over generic best practices.\n\
            \n\
            # Hooks\n\
            - Projects can register hooks in `.routecode/settings.json` to run shell commands at lifecycle events \
            (PreToolUse, PostToolUse, Stop, etc.). On first encounter the user is prompted to trust them; once \
            trusted, hooks run automatically. Hook output may include `additionalContext` that is injected into \
            your context window, and a `decision: \"block\"` PreToolUse hook will deny the corresponding tool call.\n",
        );

        let project_root = crate::utils::storage::find_project_root();

        // Inject Project Context
        if let Ok(readme) = std::fs::read_to_string(project_root.join("README.md")) {
            system_content.push_str("\n--- PROJECT README ---\n");
            system_content.push_str(&readme);
        }
        if let Ok(routecode_md) = std::fs::read_to_string(project_root.join("ROUTECODE.md")) {
            system_content.push_str("\n--- PROJECT INSTRUCTIONS (ROUTECODE.md) ---\n");
            system_content.push_str(&routecode_md);
        }

        Message::system(system_content)
    }

    async fn prepare_messages_no_trim(&self, history: &[Message]) -> Vec<Message> {
        let mut messages = vec![self.prepare_system_prompt().await];
        messages.extend(history.iter().cloned());
        messages
    }

    async fn prepare_messages(&self, history: &[Message]) -> Vec<Message> {
        let mut messages = self.prepare_messages_no_trim(history).await;

        // 3. Truncate if necessary (Sliding Window) using safe split to keep pairs intact
        let max_tokens = 100_000;
        while crate::utils::tokens::count_tokens(&messages) > max_tokens && messages.len() > 2 {
            let safe_idx = crate::core::compact::find_safe_split_index(&messages, messages.len() / 2);
            if safe_idx > 1 && safe_idx < messages.len() {
                messages.drain(1..safe_idx);
            } else {
                messages.remove(1);
            }
        }

        messages
    }

    pub async fn handle_auto_compact(
        &self,
        history: &mut Vec<Message>,
        model: &str,
        provider: Arc<dyn AIProvider>,
        tx: Option<&tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
    ) -> Result<bool, anyhow::Error> {
        // 1. Run micro-compaction first to prune large old tool results (no LLM cost)
        crate::core::compact::micro_compact(history, 5);

        // 2. Check thresholds
        let (auto_compact_enabled, window_override) = {
            let cfg = self.config.lock().await;
            (cfg.auto_compact_enabled, cfg.context_window_override)
        };

        if !auto_compact_enabled {
            return Ok(false);
        }

        let thresholds = crate::core::compact::calculate_thresholds(model, window_override);
        let prepared = self.prepare_messages_no_trim(history).await;
        let token_count = crate::utils::tokens::count_tokens(&prepared);

        if token_count > thresholds.auto_compact_threshold {
            // We need to perform full LLM summarization!
            if let Some(sender) = tx {
                let _ = sender.send(StreamChunk::CompactProgress {
                    status: "Summarizing conversation history to reclaim context space...".to_string(),
                });
            }

            // Summarize everything except the last 3 messages
            let split_idx = crate::core::compact::find_safe_split_index(history, 3);
            if split_idx > 2 {
                let to_summarize = &history[0..split_idx];
                let to_preserve = &history[split_idx..];

                match crate::core::compact::compact_conversation(
                    Arc::clone(&provider),
                    model,
                    to_summarize,
                ).await {
                    Ok(summary) => {
                        let compacted_list = crate::core::compact::build_post_compact_messages(&summary, to_preserve);
                        
                        let pre_tokens = token_count as u32;
                        *history = compacted_list;

                        // Recalculate post tokens
                        let post_prepared = self.prepare_messages_no_trim(history).await;
                        let post_tokens = crate::utils::tokens::count_tokens(&post_prepared) as u32;

                        if let Some(sender) = tx {
                            let _ = sender.send(StreamChunk::CompactResult {
                                pre_tokens,
                                post_tokens,
                            });
                        }
                        return Ok(true);
                    }
                    Err(e) => {
                        log::error!("Auto-compaction failed: {}", e);
                    }
                }
            }
        } else if token_count > thresholds.warning_threshold {
            if let Some(sender) = tx {
                let _ = sender.send(StreamChunk::ContextWarning {
                    message: format!(
                        "Context window is filling up ({}/{} tokens). Auto-compaction will trigger soon.",
                        token_count, thresholds.context_window
                    ),
                });
            }
        }

        Ok(false)
    }

    pub async fn run(
        &self,
        history: &mut Vec<Message>,
        model: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
        cancel: Option<CancellationToken>,
    ) -> Result<(), anyhow::Error> {
        // First-run hook trust: if the project defines hooks, prompt
        // the user before any tool calls fire. No-op if already
        // trusted.
        self.ensure_project_hooks_trusted(tx.as_ref()).await;

        match self
            .run_with_depth(history, model, tx.clone(), 0, cancel.clone())
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let was_cancelled = cancel.as_ref().is_some_and(|c| c.is_cancelled());
                // Fire StopFailure hook so log/cleanup hooks can run
                // even on error.
                let last_assistant = history
                    .iter()
                    .rev()
                    .find(|m| matches!(m.role, crate::core::Role::Assistant))
                    .and_then(|m| m.content.as_ref().map(|c| c.to_string()));
                self.fire_stop_failure(&e.to_string(), last_assistant, tx.as_ref())
                    .await;
                if let Some(ref tx) = tx {
                    if was_cancelled {
                        let _ = tx.send(StreamChunk::Status {
                            content: "Request cancelled by user".to_string(),
                        });
                    } else {
                        let _ = tx.send(StreamChunk::Error {
                            content: e.to_string(),
                        });
                    }
                    let _ = tx.send(StreamChunk::Done);
                }
                Err(e)
            }
        }
    }

    async fn run_with_depth(
        &self,
        history: &mut Vec<Message>,
        model: &str,
        tx: Option<tokio::sync::mpsc::UnboundedSender<StreamChunk>>,
        depth: usize,
        cancel: Option<CancellationToken>,
    ) -> Result<(), anyhow::Error> {
        if depth >= 25 {
            return Err(anyhow::anyhow!(
                "Maximum tool recursion depth (25) reached. Aborting to prevent infinite loop."
            ));
        }
        if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
            return Err(anyhow::anyhow!("Request cancelled by user"));
        }

        // Wrap messages and tools in Arc once. The retry wrapper reuses the
        // same allocation across attempts (Arc::clone is just a refcount bump)
        // so we don't pay the cost of cloning a 100k-token history on every
        // retry.
        let tools: Arc<Option<Vec<serde_json::Value>>> = {
            use std::sync::atomic::Ordering;
            let in_plan = self.is_in_plan_mode.load(Ordering::SeqCst);
            if in_plan {
                let overrides = {
                    let cfg = self.config.lock().await;
                    cfg.plan_mode_tool_overrides.clone()
                };
                let schemas = self.tool_registry.get_all_schemas();
                Arc::new(Some(
                    crate::tools::plan::filter_for_plan_mode(schemas, &overrides),
                ))
            } else {
                Arc::new(Some(self.tool_registry.get_all_schemas()))
            }
        };
        let provider_arc_for_compact = {
            let p = self.provider.lock().await;
            Arc::clone(&p)
        };

        let _ = self.handle_auto_compact(history, model, provider_arc_for_compact, tx.as_ref()).await;

        let messages: Arc<Vec<Message>> = Arc::new(self.prepare_messages(history).await);

        log::debug!(
            "Sending AI request to model: {} (messages: {})",
            model,
            messages.len()
        );

        // Snapshot the retry policy, thinking-level, and provider at the start
        // of the run. A mid-request policy change is intentionally NOT honored
        // — an in-flight retry loop keeps its current policy for the rest of
        // the request to avoid half-applied policy and to make the run's retry
        // behavior deterministic for the user. Users who want to stop a
        // runaway loop should use cancellation, not toggle.
        let (thinking_level, retry_policy, provider_arc) = {
            let config = self.config.lock().await;
            let p = self.provider.lock().await;
            (
                config.thinking_level.clone(),
                config.retry_policy.clone(),
                Arc::clone(&p),
            )
        };

        // Wrap the provider in a RetryingProvider for this request. This is
        // where the retry logic lives now — the orchestrator just consumes
        // the returned stream.
        let retrying = crate::agents::RetryingProvider::new(Arc::clone(&provider_arc), retry_policy);

        let (assistant_content, assistant_thought, tool_calls) = {
            let s_res = retrying
                .ask_with_retry(
                    Arc::clone(&messages),
                    model,
                    Arc::clone(&tools),
                    Some(&thinking_level),
                    cancel.clone(),
                )
                .await;

            let mut s = match s_res {
                Ok(stream) => stream,
                Err(err) => {
                    if crate::core::compact::is_prompt_too_long_error(&err) {
                        log::warn!("Prompt too long error detected: {}. Attempting reactive compaction...", err);
                        if let Some(ref sender) = tx {
                            let _ = sender.send(StreamChunk::Status {
                                content: "Context limit exceeded. Reactive compaction triggering to recover...".to_string(),
                            });
                        }

                        // Summarize aggressively: keep only the last 3 messages
                        let split_idx = crate::core::compact::find_safe_split_index(history, 3);
                        if split_idx > 2 {
                            let to_summarize = &history[0..split_idx];
                            let to_preserve = &history[split_idx..];
                            if let Ok(summary) = crate::core::compact::compact_conversation(
                                Arc::clone(&provider_arc),
                                model,
                                to_summarize,
                            ).await {
                                let compacted_list = crate::core::compact::build_post_compact_messages(&summary, to_preserve);
                                *history = compacted_list;

                                // Retry the recursive call immediately with compacted history!
                                return Box::pin(self.run_with_depth(history, model, tx, depth, cancel)).await;
                            }
                        }

                        // If summarization was not possible or failed, do an aggressive safe truncation
                        let safe_idx = crate::core::compact::find_safe_split_index(history, history.len() / 2);
                        if safe_idx > 1 && safe_idx < history.len() {
                            history.drain(0..safe_idx); // remove the oldest half of history
                            history.insert(0, Message::system("Conversation truncated due to context limit."));

                            // Retry immediately with the truncated history
                            return Box::pin(self.run_with_depth(history, model, tx, depth, cancel)).await;
                        }
                    }
                    return Err(err);
                }
            };

            let mut local_content = String::new();
            let mut local_thought = String::new();
            let mut local_tool_calls: Vec<crate::core::ToolCall> = Vec::new();
            let mut chunk_buffer: Vec<StreamChunk> = Vec::new();

            while let Some(chunk_res) = s.next().await {
                // Check for user cancellation between stream chunks.
                if cancel.as_ref().is_some_and(|c| c.is_cancelled()) {
                    return Err(anyhow::anyhow!("Request cancelled by user"));
                }

                let chunk = match chunk_res {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(e);
                    }
                };

                // Detect retry events emitted by the wrapper and bump the
                // session-aggregate UI state in real time. The status text is
                // produced by the wrapper, so the strings are stable.
                if let StreamChunk::Status { content } = &chunk {
                    if content.starts_with("QIR retrying")
                        || content.starts_with("QIR stream interrupted")
                    {
                        let mut u = self.usage.lock().await;
                        u.record_qir_attempt();
                        if let Some(ref tx) = tx {
                            let _ = tx.send(StreamChunk::SessionStats {
                                total_tokens: u.total_tokens,
                                total_cost: u.total_cost,
                                qir_attempts: u.qir_attempts,
                            });
                        }
                    }
                }

                // Capture usage for the session aggregate as we go (success
                // path will have flushed chunks; this catches them in
                // non-buffered flows too).
                if let StreamChunk::Usage { usage } = &chunk {
                    let mut u = self.usage.lock().await;
                    u.add(usage.prompt_tokens, usage.completion_tokens, model)
                        .await;
                }

                // Accumulate the final assistant message.
                match &chunk {
                    StreamChunk::Text { content } => {
                        local_content.push_str(content);
                    }
                    StreamChunk::Thought { content } => {
                        local_thought.push_str(content);
                    }
                    StreamChunk::ToolCall { tool_call } => {
                        if let Some(idx) = tool_call.index {
                            if let Some(existing) =
                                local_tool_calls.iter_mut().find(|tc| tc.index == Some(idx))
                            {
                                *existing = tool_call.clone();
                            } else {
                                local_tool_calls.push(tool_call.clone());
                            }
                        } else {
                            local_tool_calls.push(tool_call.clone());
                        }
                    }
                    // A `StreamChunk::Error` reaching the orchestrator means
                    // either (a) the retry wrapper was bypassed (e.g. policy
                    // is `Disabled` and the provider emitted one mid-stream),
                    // or (b) the wrapper is being misused as a passthrough.
                    // In both cases the spec is "surface to the UI and abort
                    // cleanly". `run()` above wraps the returned Err with a
                    // `StreamChunk::Error` + `Done` so the consumer can
                    // finalize without hanging.
                    StreamChunk::Error { content } => {
                        return Err(anyhow::anyhow!("Provider error: {}", content));
                    }
                    _ => {}
                }

                // Buffer Done / Status / etc. for the final flush below.
                chunk_buffer.push(chunk);
            }

            // Final SessionStats snapshot so the UI sees the up-to-date
            // aggregate after a successful (possibly retried) attempt.
            if let Some(ref tx) = tx {
                let u = self.usage.lock().await;
                let _ = tx.send(StreamChunk::SessionStats {
                    total_tokens: u.total_tokens,
                    total_cost: u.total_cost,
                    qir_attempts: u.qir_attempts,
                });
            }

            // Flush all buffered chunks to the UI (this is the chunks from the
            // successful attempt; the wrapper buffered failed attempts'
            // chunks internally and only flushed the final one).
            for chunk in chunk_buffer {
                if let Some(ref tx) = tx {
                    if let Err(e) = tx.send(chunk) {
                        log::error!("Failed to send chunk to UI: {}", e);
                    }
                }
            }

            (local_content, local_thought, local_tool_calls)
        };

        let assistant_msg = Message::assistant(
            if assistant_content.is_empty() {
                if !assistant_thought.is_empty() || !tool_calls.is_empty() {
                    Some(std::sync::Arc::from(""))
                } else {
                    None
                }
            } else {
                Some(std::sync::Arc::from(assistant_content))
            },
            if assistant_thought.is_empty() {
                None
            } else {
                Some(std::sync::Arc::from(assistant_thought))
            },
            if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls.clone())
            },
        );

        history.push(assistant_msg);

        if !tool_calls.is_empty() {
            for tc in tool_calls {
                if let Some(tool) = self.tool_registry.get(&tc.function.name) {
                    let args: serde_json::Value = match serde_json::from_str(&tc.function.arguments)
                    {
                        Ok(a) => a,
                        Err(e) => {
                            return Err(anyhow::anyhow!(
                                "Failed to parse tool arguments: {}. \
                                This usually means the AI's response was truncated because it reached its output token limit. \
                                Try asking for a smaller part of the task or increasing the limit.",
                                e
                            ));
                        }
                    };
                    let mut execute_allowed = true;
                    let mut custom_error_msg = None;
                    let mut plan_tool_result: Option<String> = None;
                    let mut pending_pre_context: Option<String> = None;

                    use std::sync::atomic::Ordering;

                    // === Plan mode tool dispatch ===
                    // Intercepted BEFORE the generic `tool.execute()` path so
                    // we can mutate orchestrator state (is_in_plan_mode) and
                    // present the plan UI before any tool runs.
                    if tc.function.name
                        == crate::tools::plan::ENTER_PLAN_MODE_TOOL_NAME
                    {
                        // Entering plan mode:
                        // 1. Set the orchestrator flag
                        // 2. Force bash_mode = ReadOnly so bash writes are
                        //    denied even if the user toggles it back
                        // 3. Reset session-unlock flag (a fresh plan must be
                        //    re-approved even if the previous plan was approved)
                        self.enter_plan_mode();
                        {
                            let mut cfg = self.config.lock().await;
                            cfg.bash_mode =
                                crate::core::config::BashMode::ReadOnly;
                        }
                        plan_tool_result = Some(format!(
                            "Entered plan mode. Read-only tools and bash are \
                             available; write tools are hidden. Use \
                             `exit_plan_mode` when ready to present your plan \
                             for approval. Plan file: ~/.routecode/plans/{}/",
                            self.session_id
                        ));
                    } else if tc.function.name
                        == crate::tools::plan::EXIT_PLAN_MODE_TOOL_NAME
                    {
                        // Reading the plan file and presenting it for
                        // approval happens here. The user must approve
                        // (with unlock) or deny. On deny we stay in plan
                        // mode and return the feedback to the model.
                        let plan_result = self
                            .handle_exit_plan_mode(&args, tx.as_ref())
                            .await;
                        match plan_result {
                            Ok(unlock) => {
                                self.exit_plan_mode(unlock);
                                plan_tool_result = Some(if unlock {
                                    "Plan approved. Write tools are unlocked \
                                     for the rest of this session."
                                        .to_string()
                                } else {
                                    "Plan approved (single step). Stay in \
                                     plan mode for subsequent operations."
                                        .to_string()
                                });
                            }
                            Err(feedback) => {
                                execute_allowed = false;
                                custom_error_msg = Some(format!(
                                    "Plan denied: {}. Revise the plan and \
                                     call exit_plan_mode again.",
                                    feedback
                                ));
                            }
                        }
                    } else if self.is_in_plan_mode.load(Ordering::SeqCst)
                        && tc.function.name == "file_write"
                    {
                        // Defense-in-depth: the schema filter should have
                        // hidden this tool, but if the model calls it
                        // anyway (e.g. cached schema), deny it.
                        execute_allowed = false;
                        custom_error_msg = Some(
                            "file_write is not available in plan mode. Call \
                             `exit_plan_mode` to unlock write tools."
                                .to_string(),
                        );
                    } else if self.is_in_plan_mode.load(Ordering::SeqCst)
                        && tc.function.name == "file_edit"
                    {
                        execute_allowed = false;
                        custom_error_msg = Some(
                            "file_edit is not available in plan mode. Call \
                             `exit_plan_mode` to unlock write tools."
                                .to_string(),
                        );
                    } else if self.is_in_plan_mode.load(Ordering::SeqCst)
                        && tc.function.name == "apply_patch"
                    {
                        execute_allowed = false;
                        custom_error_msg = Some(
                            "apply_patch is not available in plan mode. \
                             Call `exit_plan_mode` to unlock write tools."
                                .to_string(),
                        );
                    } else if tc.function.name == "bash" {
                        let command_str =
                            args["command"].as_str().unwrap_or("").to_string();

                        // Run the permission engine. The result dictates
                        // whether we prompt, allow, or hard-deny.
                        let config_guard = self.config.lock().await;
                        let decision =
                            crate::tools::bash::BashTool::evaluate(&command_str, &config_guard);
                        drop(config_guard);

                        match decision.behavior {
                            crate::tools::bash::decision::Behavior::Allow => {
                                // Config allows the command outright
                            }
                            crate::tools::bash::decision::Behavior::Deny => {
                                execute_allowed = false;
                                custom_error_msg = Some(format!(
                                    "Bash command denied: {}",
                                    decision.reason
                                ));
                                if let Some(s) = decision.suggestions.first() {
                                    custom_error_msg = Some(format!(
                                        "{} (suggestion: {})",
                                        custom_error_msg.unwrap(),
                                        s
                                    ));
                                }
                            }
                            crate::tools::bash::decision::Behavior::Ask => {
                                if !self.allow_session_commands.load(Ordering::SeqCst) {
                                    if let Some(ref sender) = tx {
                                        let (oneshot_tx, oneshot_rx) =
                                            tokio::sync::oneshot::channel();
                                        let tx_wrapped = Arc::new(
                                            tokio::sync::Mutex::new(Some(oneshot_tx)),
                                        );

                                        let warning_str =
                                            decision.warning.clone().unwrap_or_default();

                                        if let Err(e) =
                                            sender.send(StreamChunk::RequestConfirmation {
                                                message: format!(
                                                    "The AI agent wants to execute: {}",
                                                    decision.reason
                                                ),
                                                target: command_str,
                                                warning: warning_str,
                                                tx: Some(tx_wrapped),
                                            })
                                        {
                                            log::error!(
                                                "Failed to send RequestConfirmation to UI: {}",
                                                e
                                            );
                                        }

                                        match oneshot_rx.await {
                                            Ok(
                                                crate::agents::types::ConfirmationResponse::AllowOnce,
                                            ) => {}
                                            Ok(
                                                crate::agents::types::ConfirmationResponse::AllowSession,
                                            )
                                            | Ok(
                                                crate::agents::types::ConfirmationResponse::AllowWorkspace,
                                            ) => {
                                                self.allow_session_commands
                                                    .store(true, Ordering::SeqCst);
                                            }
                                            Ok(
                                                crate::agents::types::ConfirmationResponse::Deny,
                                            ) => {
                                                execute_allowed = false;
                                                custom_error_msg = Some(
                                                    "Command execution denied by user."
                                                        .to_string(),
                                                );
                                            }
                                            Ok(
                                                crate::agents::types::ConfirmationResponse::Feedback(
                                                    msg,
                                                ),
                                            ) => {
                                                execute_allowed = false;
                                                custom_error_msg = Some(format!(
                                                    "Command execution denied by user with feedback: {}",
                                                    msg
                                                ));
                                            }
                                            Err(_) => {
                                                execute_allowed = false;
                                                custom_error_msg = Some(
                                                    "Command execution cancelled (confirmation channel closed)."
                                                        .to_string(),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else if ["file_read", "file_write", "file_edit", "ls", "tree", "grep"]
                        .contains(&tc.function.name.as_str())
                        && !self.allow_session_outside_access.load(Ordering::SeqCst)
                    {
                        let path_str = args["path"].as_str().unwrap_or(".");
                        if crate::utils::storage::is_path_outside_workspace(path_str) {
                            if let Some(ref sender) = tx {
                                let (oneshot_tx, oneshot_rx) = tokio::sync::oneshot::channel();
                                let tx_wrapped =
                                    Arc::new(tokio::sync::Mutex::new(Some(oneshot_tx)));

                                if let Err(e) = sender.send(StreamChunk::RequestConfirmation {
                                    message: "The AI agent wants to access a path OUTSIDE the current workspace:".to_string(),
                                    target: path_str.to_string(),
                                    warning: String::new(),
                                    tx: Some(tx_wrapped),
                                }) {
                                    log::error!("Failed to send RequestConfirmation to UI: {}", e);
                                }

                                match oneshot_rx.await {
                                    Ok(crate::agents::types::ConfirmationResponse::AllowOnce) => {}
                                    Ok(
                                        crate::agents::types::ConfirmationResponse::AllowSession,
                                    )
                                    | Ok(
                                        crate::agents::types::ConfirmationResponse::AllowWorkspace,
                                    ) => {
                                        self.allow_session_outside_access
                                            .store(true, Ordering::SeqCst);
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Deny) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!(
                                            "Access to outside path '{}' denied by user.",
                                            path_str
                                        ));
                                    }
                                    Ok(crate::agents::types::ConfirmationResponse::Feedback(
                                        msg,
                                    )) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(format!("Access to outside path '{}' denied by user with feedback: {}", path_str, msg));
                                    }
                                    Err(_) => {
                                        execute_allowed = false;
                                        custom_error_msg = Some(
                                            "Access cancelled (confirmation channel closed)."
                                                .to_string(),
                                        );
                                    }
                                }
                            } else {
                                // If there's no UI (headless), just block it by default.
                                execute_allowed = false;
                                custom_error_msg = Some(format!("Access to outside path '{}' denied (no UI confirmation available).", path_str));
                            }
                        }
                    }

                    let result = if let Some(synthetic) = plan_tool_result {
                        // Plan-mode tool: use the synthetic result we built
                        // above; do NOT call `tool.execute()` (it has no
                        // access to orchestrator state).
                        crate::core::ToolResult::success(synthetic)
                    } else if execute_allowed {
                        // Fire PreToolUse hooks. The aggregated
                        // output may: (a) block the call, (b)
                        // override the tool input (updatedInput),
                        // or (c) inject additional context.
                        let effective_args = match self
                            .fire_pre_tool_use(
                                &tc.function.name,
                                args.clone(),
                                &tc.id,
                                tx.as_ref(),
                            )
                            .await
                        {
                            PreFire::Ok(ok) => {
                                // If the hook injected additional
                                // context and the call isn't blocked,
                                // record it on the tool result below
                                // by appending to the response.
                                if ok.additional_context.is_some() {
                                    pending_pre_context = ok.additional_context;
                                }
                                ok.args
                            }
                            PreFire::Blocked(reason) => {
                                execute_allowed = false;
                                custom_error_msg = Some(format!(
                                    "Tool call blocked by hook: {}",
                                    reason
                                ));
                                args.clone()
                            }
                        };
                        if execute_allowed {
                            match tool.execute(effective_args).await {
                                Ok(res) => res,
                                Err(e) => crate::core::ToolResult::error(format!(
                                    "Tool execution failed: {}",
                                    e
                                )),
                            }
                        } else {
                            crate::core::ToolResult::error(
                                custom_error_msg.unwrap_or_default(),
                            )
                        }
                    } else {
                        crate::core::ToolResult::error(custom_error_msg.unwrap_or_default())
                    };

                    // Fire PostToolUse / PostToolUseFailure hooks
                    // and (if appropriate) merge their additional
                    // context into the tool response.
                    let result = match &result {
                        crate::core::ToolResult {
                            success: true,
                            content: _,
                            ..
                        } => {
                            let response_value = serde_json::to_value(&result)
                                .unwrap_or(serde_json::Value::Null);
                            match self
                                .fire_post_tool_use(
                                    &tc.function.name,
                                    args.clone(),
                                    response_value.clone(),
                                    &tc.id,
                                    tx.as_ref(),
                                )
                                .await
                            {
                                PostFire::Continue(ok) => {
                                    let mut out = result.clone();
                                    if let Some(ctx) = pending_pre_context.as_deref() {
                                        out.content = Some(format!(
                                            "{}\n\n{}",
                                            ctx,
                                            out.content.as_deref().unwrap_or("")
                                        ));
                                    }
                                    if !matches!(
                                        ok.tool_response,
                                        serde_json::Value::Null
                                    ) {
                                        out.content = Some(match ok.tool_response {
                                            serde_json::Value::String(s) => s,
                                            other => other.to_string(),
                                        });
                                    }
                                    if let Some(msg) = ok.system_message {
                                        out.content = Some(format!(
                                            "{}\n\n{}",
                                            out.content.as_deref().unwrap_or(""),
                                            msg
                                        ));
                                    }
                                    out
                                }
                                PostFire::Stop(reason) => {
                                    let mut out = result.clone();
                                    if let Some(r) = reason {
                                        out.content = Some(format!(
                                            "{}\n\n(stop: {})",
                                            out.content.as_deref().unwrap_or(""),
                                            r
                                        ));
                                    }
                                    out
                                }
                            }
                        }
                        other => {
                            // Failure path: fire PostToolUseFailure.
                            let err_str = other
                                .error
                                .as_deref()
                                .or(other.content.as_deref())
                                .unwrap_or("(unknown error)")
                                .to_string();
                            self.fire_post_tool_use_failure(
                                &tc.function.name,
                                args.clone(),
                                &tc.id,
                                &err_str,
                                tx.as_ref(),
                            )
                            .await;
                            other.clone()
                        }
                    };
                    let content = serde_json::to_string(&result)?;

                    let tool_msg =
                        Message::tool(tc.id.clone(), tc.function.name.clone(), content.clone());
                    history.push(tool_msg);

                    if let Some(ref tx) = tx {
                        if let Err(e) = tx.send(StreamChunk::ToolResult {
                            tool_call_id: tc.id.clone(),
                            name: tc.function.name.clone(),
                            content: content.clone(),
                        }) {
                            log::error!("Failed to send tool result to UI: {}", e);
                        }
                    }
                }
            }
            // Recurse after tool execution
            return Box::pin(self.run_with_depth(history, model, tx, depth + 1, cancel)).await;
        }

        if let Some(ref tx) = tx {
            let _ = tx.send(StreamChunk::FinalHistory {
                history: history.clone(),
            });
        }

        // Fire the Stop hook. The turn has finished (no more tool
        // calls). The hook can log / emit system messages. We
        // currently don't use the aggregated stop_reason to abort the
        // run — by the time Stop fires, the turn is already over.
        let last_assistant = history
            .iter()
            .rev()
            .find(|m| matches!(m.role, crate::core::Role::Assistant))
            .and_then(|m| m.content.as_ref().map(|c| c.to_string()));
        let _ = self.fire_stop(last_assistant, tx.as_ref()).await;

        if let Some(ref tx) = tx {
            if let Err(e) = tx.send(StreamChunk::Done) {
                log::error!("Failed to send Done chunk to UI: {}", e);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::StreamChunk;
    use crate::core::{FunctionCall, Message, Role, ToolCall, ToolResult};
    use crate::tools::traits::Tool;
    use async_trait::async_trait;
    use futures::stream;
    use serde_json::json;

    struct MockProvider {
        responses: Mutex<Vec<Vec<StreamChunk>>>,
    }

    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec!["mock".to_string()])
        }
        async fn ask(
            &self,
            _msgs: Arc<Vec<Message>>,
            _model: &str,
            _tools: Arc<Option<Vec<serde_json::Value>>>,
            _thinking_level: Option<&str>,
        ) -> Result<crate::agents::traits::StreamResponse, anyhow::Error> {
            let mut resps = self.responses.lock().await;
            if resps.is_empty() {
                return Err(anyhow::anyhow!("No more mock responses"));
            }
            let chunks = resps.remove(0);
            let s = stream::iter(chunks.into_iter().map(Ok));
            Ok(Box::pin(s))
        }
    }

    struct MockTool;
    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }
        fn description(&self) -> &str {
            "A mock tool"
        }
        fn parameters(&self) -> serde_json::Value {
            json!({})
        }
        async fn execute(&self, _args: serde_json::Value) -> Result<ToolResult, anyhow::Error> {
            Ok(ToolResult::success("success"))
        }
    }

    #[tokio::test]
    async fn test_orchestrator_simple_chat() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(vec![vec![
                StreamChunk::Text {
                    content: "Hello!".to_string(),
                },
                StreamChunk::Done,
            ]]),
        });
        let tool_registry = ToolRegistry::new();
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Hi")];
        orchestrator
            .run(&mut history, "mock", None, None)
            .await
            .unwrap();

        assert_eq!(history.len(), 2);
        assert_eq!(history[1].role, Role::Assistant);
        assert_eq!(history[1].content.as_deref(), Some("Hello!"));
    }

    #[tokio::test]
    async fn test_orchestrator_tool_use() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(vec![
                // First response: call tool
                vec![
                    StreamChunk::ToolCall {
                        tool_call: ToolCall {
                            id: "call_1".to_string(),
                            r#type: "function".to_string(),
                            index: Some(0),
                            function: FunctionCall {
                                name: "mock_tool".to_string(),
                                arguments: "{}".to_string(),
                            },
                        },
                    },
                    StreamChunk::Done,
                ],
                // Second response: finalize
                vec![
                    StreamChunk::Text {
                        content: "Tool executed!".to_string(),
                    },
                    StreamChunk::Done,
                ],
            ]),
        });

        let mut tool_registry = ToolRegistry::new();
        tool_registry.register(Arc::new(MockTool));
        let config = Arc::new(Mutex::new(crate::core::Config::default()));
        let orchestrator = AgentOrchestrator::new(provider, Arc::new(tool_registry), config);

        let mut history = vec![Message::user("Run tool")];
        orchestrator
            .run(&mut history, "mock", None, None)
            .await
            .unwrap();

        // History: User -> Assistant (ToolCall) -> ToolResult -> Assistant (Final)
        assert_eq!(history.len(), 4);
        assert_eq!(history[1].role, Role::Assistant);
        assert!(history[1].tool_calls.is_some());
        assert_eq!(history[2].role, Role::Tool);
        assert_eq!(history[3].role, Role::Assistant);
        assert_eq!(history[3].content.as_deref(), Some("Tool executed!"));
    }
}

/// Generate a stable-ish session id: a UUID v4 if the `uuid` crate is
/// available, otherwise a timestamp + pid fallback. Used as a key for
/// per-session state (e.g. plan files).
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("session-{}-{:x}", pid, nanos)
}
