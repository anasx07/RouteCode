use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use routecode_sdk::agents::types::{ConfirmationResponse, PlanApprovalResponse, StreamChunk};
use routecode_sdk::core::{AgentOrchestrator, Config, Message};
use routecode_sdk::tools::{
    bash::BashTool,
    file_ops::{ApplyPatchTool, FileEditTool, FileReadTool, FileWriteTool},
    lsp::LspTool,
    mcp::manager::McpManager,
    navigation::{GrepTool, LsTool, TreeTool},
    subagent::SubAgentTool,
    web::{fetch::WebFetchTool, search::WebSearchTool},
    ToolRegistry,
};
use routecode_sdk::utils::storage::{
    find_project_root, get_base_dir, list_sessions, load_session, load_session_config,
    load_workspace_config, sanitize_session_name, save_session, save_session_config,
    save_workspace_config, Session, SessionConfig, WorkspaceConfig,
};

type PendingConfirmation =
    Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>>;

type PendingPlanApproval = Arc<
    tokio::sync::Mutex<
        Option<tokio::sync::oneshot::Sender<PlanApprovalResponse>>,
    >,
>;

// Define the Shared Application State
pub struct AppState {
    pub orchestrator: Mutex<Option<Arc<AgentOrchestrator>>>,
    pub pending_confirmation: Mutex<Option<PendingConfirmation>>,
    pub pending_plan_approval: Mutex<Option<PendingPlanApproval>>,
    pub cancel_token: Mutex<Option<CancellationToken>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            orchestrator: Mutex::new(None),
            pending_confirmation: Mutex::new(None),
            pending_plan_approval: Mutex::new(None),
            cancel_token: Mutex::new(None),
        }
    }
}

// 1. Get Persistent Config Command
#[tauri::command]
async fn get_config() -> Result<Config, String> {
    println!("Loading persistent RouteCode configuration...");
    let config = routecode_sdk::utils::storage::load_config().unwrap_or_default();
    Ok(config)
}

// 2. Save Persistent Config Command
#[tauri::command]
async fn save_config(config: Config) -> Result<String, String> {
    println!(
        "Saving persistent RouteCode configuration: provider={}, model={}",
        config.provider, config.model
    );
    routecode_sdk::utils::storage::save_config(&config)
        .map_err(|e| format!("Failed to save configuration: {}", e))?;
    Ok("Configuration saved successfully".to_string())
}

// 3. List Saved Sessions Command
#[tauri::command]
async fn list_saved_sessions() -> Result<Vec<String>, String> {
    println!("Listing saved sessions...");
    let sessions = list_sessions().map_err(|e| format!("Failed to list sessions: {}", e))?;
    Ok(sessions)
}

// 4. Load Saved Session Command
#[tauri::command]
async fn load_saved_session(name: String) -> Result<Session, String> {
    println!("Loading saved session: {}", name);
    let session = load_session(&name).map_err(|e| format!("Failed to load session: {}", e))?;
    Ok(session)
}

// 5. Save/Update Session Command
#[tauri::command]
async fn save_saved_session(
    name: String,
    messages: Vec<Message>,
    model: String,
) -> Result<String, String> {
    println!(
        "Saving session: {} (message count={})",
        name,
        messages.len()
    );
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let session = Session {
        messages,
        usage: routecode_sdk::utils::costs::Usage::default(),
        model,
        timestamp,
    };
    save_session(&name, &session).map_err(|e| format!("Failed to save session: {}", e))?;
    Ok("Session saved successfully".to_string())
}

// 6. Delete Session Command
#[tauri::command]
async fn delete_session(name: String) -> Result<String, String> {
    println!("Deleting session: {}", name);
    let safe_name = sanitize_session_name(&name);
    if safe_name.is_empty() {
        return Err("Invalid session name".to_string());
    }

    let project_root = find_project_root();
    let workspace_dir = project_root.join(".routecode");
    let session_dir = workspace_dir.join("sessions").join(&safe_name);
    if session_dir.exists() {
        std::fs::remove_dir_all(&session_dir)
            .map_err(|e| format!("Failed to delete workspace session directory: {}", e))?;
    }

    let old_path = get_base_dir()
        .join("sessions")
        .join(format!("{}.json", safe_name));
    if old_path.exists() {
        std::fs::remove_file(&old_path)
            .map_err(|e| format!("Failed to delete legacy session file: {}", e))?;
    }

    Ok("Session deleted successfully".to_string())
}

// 7. Initialize RouteCode SDK Engine Command
#[tauri::command]
async fn init_engine(
    state: State<'_, AppState>,
    provider_name: String,
    model_name: String,
) -> Result<String, String> {
    println!(
        "Initializing RouteCode Engine: provider={}, model={}",
        provider_name, model_name
    );

    // Load persistent configuration
    let mut config = routecode_sdk::utils::storage::load_config().unwrap_or_default();
    config.provider = provider_name.clone();
    config.model = model_name.clone();

    // Discover API Key for the selected provider
    let api_key = std::env::var(format!("{}_API_KEY", provider_name.to_uppercase()))
        .ok()
        .or_else(|| config.api_keys.get(&provider_name).cloned());

    let api_key = match api_key {
        Some(key) => key,
        None => {
            // Default placeholder if none exists, allowing fallback / testing
            "your-api-key-here".to_string()
        }
    };

    // Resolve Provider Agent interface
    let provider = if provider_name == "vertex" {
        routecode_sdk::agents::resolve_provider_with_config(
            &provider_name,
            api_key,
            &config.vertex_project,
            &config.vertex_location,
        )
    } else {
        routecode_sdk::agents::resolve_provider(&provider_name, api_key)
    };

    // Register Secure Tools into Registry
    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(FileReadTool));
    tool_registry.register(Arc::new(FileWriteTool));
    tool_registry.register(Arc::new(FileEditTool));
    tool_registry.register(Arc::new(BashTool));
    tool_registry.register(Arc::new(LsTool));
    tool_registry.register(Arc::new(TreeTool));
    tool_registry.register(Arc::new(GrepTool));
    tool_registry.register(Arc::new(LspTool::new()));
    tool_registry.register(Arc::new(ApplyPatchTool));
    tool_registry.register(Arc::new(WebFetchTool));
    tool_registry.register(Arc::new(WebSearchTool));

    // Initialize MCP Manager and load dynamic tools
    let mcp_manager = McpManager::new();
    if let Err(e) = mcp_manager
        .load_and_register_tools(&mut tool_registry)
        .await
    {
        println!("Warning: Failed to load MCP tools: {}", e);
    }

    if config.sub_agents_enabled {
        let registry_clone = Arc::new(tool_registry.clone());
        tool_registry.register(Arc::new(SubAgentTool::new(
            provider.clone(),
            registry_clone,
            Arc::new(Mutex::new(config.clone())),
        )));
    }

    let tool_registry = Arc::new(tool_registry);

    // Build the Mutex Config and Orchestrator
    let config_mutex = Arc::new(Mutex::new(config));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        provider,
        tool_registry,
        config_mutex,
    ));

    // Store in AppState
    let mut orch_guard = state.orchestrator.lock().await;
    *orch_guard = Some(orchestrator);

    Ok("RouteCode SDK Engine Initialized Successfully".to_string())
}

// 7b. Fetch available models for a given provider using its API key. This
// mirrors the CLI's `/model` flow: resolve the provider trait, call
// `list_models()` (which hits the OpenAI-compatible `/models` endpoint for
// most providers, the Anthropic/Cloudflare fallbacks for those).
#[tauri::command]
async fn fetch_provider_models(
    provider_id: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    println!("Fetching models for provider={}", provider_id);

    if provider_id.trim().is_empty() {
        return Err("provider_id is empty".to_string());
    }

    let provider = routecode_sdk::agents::resolve_provider(&provider_id, api_key);
    let mut models = provider
        .list_models()
        .await
        .map_err(|e| format!("Failed to list models for {}: {}", provider_id, e))?;

    models.sort();
    models.dedup();
    Ok(models)
}

// 8. Stream Agent Response Command
#[tauri::command]
async fn send_message(
    app: AppHandle,
    state: State<'_, AppState>,
    history: Vec<Message>,
    model: String,
) -> Result<String, String> {
    println!(
        "Received prompt from frontend. Message history length: {}",
        history.len()
    );

    // Resolve the active orchestrator from state
    let orchestrator = {
        let guard = state.orchestrator.lock().await;
        match &*guard {
            Some(orch) => orch.clone(),
            None => return Err("SDK Engine not initialized. Call init_engine first.".to_string()),
        }
    };

    // Mint a fresh cancellation token for this request. Cancel any prior
    // in-flight request first to keep the invariant that at most one
    // request is active at a time.
    let cancel_token = CancellationToken::new();
    {
        let mut guard = state.cancel_token.lock().await;
        if let Some(prev) = guard.take() {
            prev.cancel();
        }
        *guard = Some(cancel_token.clone());
    }

    // Run the orchestrator in a spawned background thread
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

    let mut history_mut = history.clone();
    let cancel_for_task = cancel_token.clone();
    tokio::spawn(async move {
        let _ = orchestrator
            .run(&mut history_mut, &model, Some(tx), Some(cancel_for_task))
            .await;
    });

    // Listen to the unbounded channel and stream to the frontend
    let app_clone = app.clone();

    tokio::spawn(async move {
        let state_clone = app_clone.state::<AppState>();
        while let Some(chunk) = rx.recv().await {
            match chunk.clone() {
                StreamChunk::RequestConfirmation {
                    message: _,
                    target: _,
                    warning: _,
                    tx: oneshot_tx,
                } => {
                    // Stash the oneshot channel sender in the global AppState for allow/deny confirmation
                    if let Some(oneshot) = oneshot_tx {
                        let mut pending_guard = state_clone.pending_confirmation.lock().await;
                        *pending_guard = Some(oneshot);
                    }

                    // Emit RequestConfirmation event to trigger frontend modal dialog
                    let _ = app_clone.emit("agent-chunk", chunk);
                }
                StreamChunk::RequestPlanApproval { tx: oneshot_tx, .. } => {
                    // Stash the plan-approval sender in the global
                    // AppState for the frontend modal. Emit the
                    // event for the frontend to render the dialog.
                    if let Some(oneshot) = oneshot_tx {
                        let mut pending_guard = state_clone.pending_plan_approval.lock().await;
                        *pending_guard = Some(oneshot);
                    }
                    let _ = app_clone.emit("agent-chunk", chunk);
                }
                StreamChunk::Done => {
                    let _ = app_clone.emit("agent-chunk", chunk);
                    break;
                }
                _ => {
                    // Standard text, thought, and tool status chunks
                    let _ = app_clone.emit("agent-chunk", chunk);
                }
            }
        }
    });

    Ok("Streaming started".to_string())
}

// 8b. Cancel an in-flight agent run
#[tauri::command]
async fn cancel_message(state: State<'_, AppState>) -> Result<String, String> {
    println!("Cancellation requested for in-flight agent run");

    let token = {
        let guard = state.cancel_token.lock().await;
        guard.clone()
    };

    match token {
        Some(t) => {
            t.cancel();
            Ok("Cancellation signal sent to agent".to_string())
        }
        None => Err("No in-flight agent run to cancel".to_string()),
    }
}

// 9. User confirmation Response Command. The `response` string maps to a
// `ConfirmationResponse` variant the orchestrator awaits on:
//
//   "allow_once"      -> AllowOnce
//   "allow_session"   -> AllowSession (sets the in-memory atomic and
//                        persists `allow_all_commands` /
//                        `allow_all_outside_access` to the active session)
//   "allow_workspace" -> AllowWorkspace (sets the atomic, persists to the
//                        active session, AND to the workspace config so
//                        every session in this workspace inherits it)
//   "deny"            -> Deny
//   "feedback:<text>" -> Feedback(String) — the LLM sees the feedback as a
//                        denial reason
#[tauri::command]
async fn respond_confirmation(
    state: State<'_, AppState>,
    app: AppHandle,
    response: String,
    session_name: Option<String>,
) -> Result<String, String> {
    println!("User responded to confirmation: {}", response);

    let parsed = parse_confirmation_response(&response);

    // For session/workspace allows we also flip the orchestrator's atomic
    // eagerly and persist the change. The orchestrator's own branch in the
    // await arm does the same flip; doing it here lets the modal close
    // immediately and the next tool call in this session skips the prompt.
    if matches!(
        parsed,
        ConfirmationResponse::AllowSession | ConfirmationResponse::AllowWorkspace
    ) {
        let orch_guard = state.orchestrator.lock().await;
        if let Some(orch) = orch_guard.as_ref() {
            use std::sync::atomic::Ordering;
            orch.allow_session_commands.store(true, Ordering::SeqCst);
            orch.allow_session_outside_access
                .store(true, Ordering::SeqCst);
        }

        if let Some(name) = session_name.as_deref() {
            // Persist session-level permission.
            if let Ok(mut sc) = load_session_config(name) {
                sc.allow_all_commands = true;
                sc.allow_all_outside_access = true;
                if let Err(e) = save_session_config(name, &sc) {
                    eprintln!("Failed to save session config: {}", e);
                }
            }
            // For AllowWorkspace also persist to workspace config.
            if matches!(parsed, ConfirmationResponse::AllowWorkspace) {
                let wc = WorkspaceConfig {
                    allow_all_outside_access: true,
                    allowed_outside_paths: vec![],
                };
                if let Err(e) = save_workspace_config(&wc) {
                    eprintln!("Failed to save workspace config: {}", e);
                }
            }
        }
        let _ = app;
    }

    let sender_opt = {
        let mut pending_guard = state.pending_confirmation.lock().await;
        pending_guard.take()
    };

    if let Some(tx_mutex) = sender_opt {
        let mut tx_guard = tx_mutex.lock().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(parsed);
            return Ok("Permission response sent to agent".to_string());
        }
    }

    Err("No pending confirmation request found".to_string())
}

fn parse_confirmation_response(s: &str) -> ConfirmationResponse {
    if let Some(rest) = s.strip_prefix("feedback:") {
        ConfirmationResponse::Feedback(rest.to_string())
    } else {
        match s {
            "allow_once" => ConfirmationResponse::AllowOnce,
            "allow_session" => ConfirmationResponse::AllowSession,
            "allow_workspace" => ConfirmationResponse::AllowWorkspace,
            "deny" => ConfirmationResponse::Deny,
            // Back-compat: a bare "true"/"false" still maps to allow-once/deny
            "true" => ConfirmationResponse::AllowOnce,
            "false" => ConfirmationResponse::Deny,
            other => ConfirmationResponse::Feedback(format!(
                "Unknown response '{}' treated as deny",
                other
            )),
        }
    }
}

// 9b. Load per-session sandbox config. Defaults to `SessionConfig::default()`
// (all denies) if no file exists on disk.
#[tauri::command]
async fn load_session_config_cmd(name: String) -> Result<SessionConfig, String> {
    load_session_config(&name).map_err(|e| format!("Failed to load session config: {}", e))
}

// 9c. Persist per-session sandbox config. Used by the React side when the
// user clicks "Allow for this session" in the confirmation modal.
#[tauri::command]
async fn save_session_config_cmd(name: String, config: SessionConfig) -> Result<String, String> {
    save_session_config(&name, &config)
        .map_err(|e| format!("Failed to save session config: {}", e))?;
    Ok("Session config saved".to_string())
}

// 9d. Apply a session's persisted sandbox flags to the running orchestrator's
// atomics. Called on session load / tab switch so the user's previous
// "Allow for this session" choice is honored on the next run.
#[tauri::command]
async fn set_session_permissions(
    state: State<'_, AppState>,
    name: String,
) -> Result<SessionConfig, String> {
    let sc =
        load_session_config(&name).map_err(|e| format!("Failed to load session config: {}", e))?;
    let wc = load_workspace_config().unwrap_or_default();

    use std::sync::atomic::Ordering;
    let orch_guard = state.orchestrator.lock().await;
    if let Some(orch) = orch_guard.as_ref() {
        let allow_commands = sc.allow_all_commands || wc.allow_all_outside_access;
        orch.allow_session_commands
            .store(allow_commands, Ordering::SeqCst);
        orch.allow_session_outside_access.store(
            sc.allow_all_outside_access || wc.allow_all_outside_access,
            Ordering::SeqCst,
        );
    }
    Ok(sc)
}

// 9e. Load workspace config (per-folder sandbox rules).
#[tauri::command]
async fn load_workspace_config_cmd() -> Result<WorkspaceConfig, String> {
    Ok(load_workspace_config().unwrap_or_default())
}

// 10. Check for updates
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    let current_version = env!("CARGO_PKG_VERSION");
    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            let update_version = update.version.clone();
            let body = update.body.clone().unwrap_or_default();
            let date = update.date.map(|d| d.to_string()).unwrap_or_default();

            let info = routecode_sdk::update::types::UpdateInfo {
                version: update_version.clone(),
                current_version: current_version.to_string(),
                changelog: body,
                download_url: String::new(),
                checksum_url: String::new(),
                published_at: date,
                is_update_available: true,
            };

            serde_json::to_string(&info)
                .map_err(|e| format!("Failed to serialize update info: {}", e))
        }
        Ok(None) => {
            let info = routecode_sdk::update::types::UpdateInfo {
                version: current_version.to_string(),
                current_version: current_version.to_string(),
                changelog: String::new(),
                download_url: String::new(),
                checksum_url: String::new(),
                published_at: String::new(),
                is_update_available: false,
            };
            serde_json::to_string(&info)
                .map_err(|e| format!("Failed to serialize update info: {}", e))
        }
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}

// 11. Download and install update
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<String, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => match update.download_and_install(|_, _| {}, || {}).await {
            Ok(()) => Ok("Update installed. Please restart the application.".to_string()),
            Err(e) => Err(format!("Update installation failed: {}", e)),
        },
        Ok(None) => Err("No update available".to_string()),
        Err(e) => Err(format!("Update check failed: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            init_engine,
            fetch_provider_models,
            send_message,
            cancel_message,
            respond_confirmation,
            load_session_config_cmd,
            save_session_config_cmd,
            set_session_permissions,
            load_workspace_config_cmd,
            get_config,
            save_config,
            list_saved_sessions,
            load_saved_session,
            save_saved_session,
            delete_session,
            check_update,
            install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
