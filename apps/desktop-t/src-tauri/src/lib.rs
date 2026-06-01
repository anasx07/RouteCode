use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, State, Manager};

use routecode_sdk::core::{AgentOrchestrator, Message, Config};
use routecode_sdk::agents::types::{ConfirmationResponse, StreamChunk};
use routecode_sdk::tools::ToolRegistry;
use routecode_sdk::tools::bash::BashTool;
use routecode_sdk::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
use routecode_sdk::tools::navigation::{GrepTool, LsTool, TreeTool};
use routecode_sdk::utils::storage::{
    Session, save_session, load_session, list_sessions, sanitize_session_name,
    find_project_root, get_base_dir
};

type PendingConfirmation = Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<ConfirmationResponse>>>>;

// Define the Shared Application State
pub struct AppState {
    pub orchestrator: Mutex<Option<Arc<AgentOrchestrator>>>,
    pub pending_confirmation: Mutex<Option<PendingConfirmation>>,
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
    println!("Saving persistent RouteCode configuration: provider={}, model={}", config.provider, config.model);
    routecode_sdk::utils::storage::save_config(&config)
        .map_err(|e| format!("Failed to save configuration: {}", e))?;
    Ok("Configuration saved successfully".to_string())
}

// 3. List Saved Sessions Command
#[tauri::command]
async fn list_saved_sessions() -> Result<Vec<String>, String> {
    println!("Listing saved sessions...");
    let sessions = list_sessions()
        .map_err(|e| format!("Failed to list sessions: {}", e))?;
    Ok(sessions)
}

// 4. Load Saved Session Command
#[tauri::command]
async fn load_saved_session(name: String) -> Result<Session, String> {
    println!("Loading saved session: {}", name);
    let session = load_session(&name)
        .map_err(|e| format!("Failed to load session: {}", e))?;
    Ok(session)
}

// 5. Save/Update Session Command
#[tauri::command]
async fn save_saved_session(name: String, messages: Vec<Message>, model: String) -> Result<String, String> {
    println!("Saving session: {} (message count={})", name, messages.len());
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
    save_session(&name, &session)
        .map_err(|e| format!("Failed to save session: {}", e))?;
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

    let old_path = get_base_dir().join("sessions").join(format!("{}.json", safe_name));
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
    println!("Initializing RouteCode Engine: provider={}, model={}", provider_name, model_name);

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
            &provider_name, api_key,
            &config.vertex_project, &config.vertex_location,
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

// 8. Stream Agent Response Command
#[tauri::command]
async fn send_message(
    app: AppHandle,
    state: State<'_, AppState>,
    history: Vec<Message>,
    model: String,
) -> Result<String, String> {
    println!("Received prompt from frontend. Message history length: {}", history.len());

    // Resolve the active orchestrator from state
    let orchestrator = {
        let guard = state.orchestrator.lock().await;
        match &*guard {
            Some(orch) => orch.clone(),
            None => return Err("SDK Engine not initialized. Call init_engine first.".to_string()),
        }
    };

    // Run the orchestrator in a spawned background thread
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StreamChunk>();

    let mut history_mut = history.clone();
    tokio::spawn(async move {
        let _ = orchestrator.run(&mut history_mut, &model, Some(tx)).await;
    });

    // Listen to the unbounded channel and stream to the frontend
    let app_clone = app.clone();
    
    tokio::spawn(async move {
        let state_clone = app_clone.state::<AppState>();
        while let Some(chunk) = rx.recv().await {
            match chunk.clone() {
                StreamChunk::RequestConfirmation { message: _, target: _, tx: oneshot_tx } => {
                    // Stash the oneshot channel sender in the global AppState for allow/deny confirmation
                    if let Some(oneshot) = oneshot_tx {
                        let mut pending_guard = state_clone.pending_confirmation.lock().await;
                        *pending_guard = Some(oneshot);
                    }
                    
                    // Emit RequestConfirmation event to trigger frontend modal dialog
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

// 9. User confirmation Response Command
#[tauri::command]
async fn respond_confirmation(
    state: State<'_, AppState>,
    allowed: bool,
) -> Result<String, String> {
    println!("User responded to confirmation dialog: allowed={}", allowed);

    let sender_opt = {
        let mut pending_guard = state.pending_confirmation.lock().await;
        pending_guard.take()
    };

    if let Some(tx_mutex) = sender_opt {
        let mut tx_guard = tx_mutex.lock().await;
        if let Some(tx) = tx_guard.take() {
            let response = if allowed {
                ConfirmationResponse::AllowOnce
            } else {
                ConfirmationResponse::Deny
            };

            let _ = tx.send(response);
            return Ok("Permission response sent to agent".to_string());
        }
    }

    Err("No pending confirmation request found".to_string())
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
            let date = update.date
                .map(|d| d.to_string())
                .unwrap_or_default();

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
        Ok(Some(update)) => {
            match update.download_and_install(|_, _| {}, || {}).await {
                Ok(()) => Ok("Update installed. Please restart the application.".to_string()),
                Err(e) => Err(format!("Update installation failed: {}", e)),
            }
        }
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
            send_message,
            respond_confirmation,
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
