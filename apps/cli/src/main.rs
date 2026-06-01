use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "Model to use")]
    pub model: Option<String>,

    #[arg(
        long,
        help = "Provider (openrouter, openai, anthropic, google, deepseek, nvidia, cloudflare-workers, cloudflare-gateway, opencode-zen, opencode-go)"
    )]
    pub provider: Option<String>,

    #[arg(short, long, help = "Resume a saved session by name")]
    pub resume: Option<String>,

    #[arg(long, help = "Check for and install the latest version of RouteCode")]
    pub update: bool,

    #[arg(
        short,
        long,
        help = "Development mode: opens log window at DEBUG level"
    )]
    pub debug: bool,

    #[arg(long, help = "Export a session to a portable JSON file")]
    pub export: Option<String>,

    #[arg(long, help = "Import a session from a JSON file")]
    pub import: Option<String>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show version info
    Version,
}

mod ui;

use crossterm::{
    event::{EnableBracketedPaste, DisableBracketedPaste, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use routecode_sdk::core::AgentOrchestrator;
use routecode_sdk::tools::bash::BashTool;
use routecode_sdk::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
use routecode_sdk::tools::navigation::{GrepTool, LsTool, TreeTool};
use routecode_sdk::tools::ToolRegistry;
use std::io;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::{run_app, App};
use simplelog::{CombinedLogger, ConfigBuilder, LevelFilter, SharedLogger, WriteLogger};


fn restore_terminal() {
    use crossterm::terminal::disable_raw_mode;
    let _ = disable_raw_mode();
    let _ = execute!(
        std::io::stdout(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    );
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let base_dir = routecode_sdk::utils::storage::get_base_dir();
    if !base_dir.exists() {
        std::fs::create_dir_all(&base_dir)?;
    }
    let log_path = base_dir.join("routecode.log");

    let log_level = if cli.debug { LevelFilter::Debug } else { LevelFilter::Info };

    let loggers: Vec<Box<dyn SharedLogger>> = vec![
        WriteLogger::new(
            log_level,
            ConfigBuilder::default().set_time_format_rfc3339().build(),
            std::fs::OpenOptions::new().create(true).append(true).open(&log_path)?,
        ),
    ];

    CombinedLogger::init(loggers)?;

    log::info!("Starting RouteCode v{}", env!("CARGO_PKG_VERSION"));

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    if cli.debug {
        log::debug!("Debug mode active. Spawning log window...");
        let spawn_result = {
            #[cfg(target_os = "windows")]
            {
                Command::new("cmd")
                    .args(["/C", "start", "powershell", "-NoExit", "-Command", &format!("Get-Content -Path \"{}\" -Wait", log_path.display())])
                    .spawn()
                    .map(|_| ())
            }
            #[cfg(target_os = "macos")]
            {
                Command::new("osascript")
                    .args(["-e", &format!("tell application \"Terminal\" to do script \"tail -f '{}'\"", log_path.display())])
                    .spawn()
                    .map(|_| ())
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos")))]
            {
                Command::new("x-terminal-emulator")
                    .args(["-e", "tail", "-f", &log_path.display().to_string()])
                    .spawn()
                    .map(|_| ())
            }
        };
        if let Err(e) = spawn_result {
            let msg = format!("Warning: Failed to open debug log window: {}", e);
            log::warn!("{}", msg);
            eprintln!("{}", msg);
        }
    }

    if let Some(Commands::Version) = cli.command {
        println!("routecode {}", env!("CARGO_PKG_VERSION"));
        println!("Rust based");
        return Ok(());
    }

    if let Some(session_name) = &cli.export {
        let session = routecode_sdk::utils::storage::load_session(session_name)
            .map_err(|e| anyhow::anyhow!("Failed to load session '{}': {}", session_name, e))?;
        let path = std::env::current_dir()?.join(format!("{}.routecode-session", session_name));
        let json = serde_json::to_string_pretty(&session)?;
        std::fs::write(&path, json)?;
        println!("Session '{}' exported to {}", session_name, path.display());
        return Ok(());
    }

    if let Some(path_str) = &cli.import {
        let path = std::path::Path::new(path_str);
        let json = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read '{}': {}", path.display(), e))?;
        let session: routecode_sdk::utils::storage::Session = serde_json::from_str(&json)?;
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported");
        routecode_sdk::utils::storage::save_session(name, &session)?;
        if let Ok(mut config) = routecode_sdk::utils::storage::load_session_config(name) {
            config.allow_all_commands = false;
            config.allow_all_outside_access = false;
            let _ = routecode_sdk::utils::storage::save_session_config(name, &config);
        }
        println!("Session imported as '{}'", name);
        return Ok(());
    }

    // Initialize logic
    let mut config = routecode_sdk::utils::storage::load_config().unwrap_or_default();

    if let Some(m) = &cli.model {
        config.model = m.clone();
    }
    if let Some(p) = &cli.provider {
        config.provider = p.clone();
    }

    // API Key Discovery
    let provider_name = config.provider.clone();
    let api_key = std::env::var(format!("{}_API_KEY", provider_name.to_uppercase()))
        .ok()
        .or_else(|| config.api_keys.get(&provider_name).cloned());

    let api_key = match api_key {
        Some(key) => key,
        None => {
            if cli.debug {
                "your-api-key-here".to_string()
            } else {
                anyhow::bail!("API Key for {} not found. Set {}_API_KEY environment variable or configure it in ~/.routecode/config.json", 
                    provider_name, provider_name.to_uppercase());
            }
        }
    };

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

    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(FileReadTool));
    tool_registry.register(Arc::new(FileWriteTool));
    tool_registry.register(Arc::new(FileEditTool));
    tool_registry.register(Arc::new(BashTool));
    tool_registry.register(Arc::new(LsTool));
    tool_registry.register(Arc::new(TreeTool));
    tool_registry.register(Arc::new(GrepTool));
    let tool_registry = Arc::new(tool_registry);

    let config_mutex = Arc::new(Mutex::new(config.clone()));
    let orchestrator = Arc::new(AgentOrchestrator::new(
        provider,
        tool_registry,
        config_mutex,
    ));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(orchestrator, config.provider.clone(), config.model.clone());

    let tx = app.tx.clone();
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let mut config_clone = config.clone();
    let update_handle = tokio::spawn(async move {
        if routecode_sdk::update::should_check(config_clone.last_update_check, 24) {
            match routecode_sdk::update::check_for_update(&current_version, "anasx07/routecode").await {
                Ok(info) => {
                    config_clone.last_update_check = routecode_sdk::update::now_timestamp();
                    let _ = routecode_sdk::utils::storage::save_config(&config_clone);
                    if info.is_update_available {
                        let _ = tx.send(routecode_sdk::agents::types::StreamChunk::UpdateAvailable {
                            version: info.version,
                            changelog: info.changelog,
                            published_at: info.published_at,
                        });
                    }
                }
                Err(e) => {
                    log::warn!("Update check failed: {}", e);
                }
            }
        }
    });
    app.current_model = config.model;

    if let Some(resume_name) = cli.resume {
        match routecode_sdk::utils::storage::load_session(&resume_name) {
            Ok(session) => {
                app.history = session.messages;
                app.current_model = session.model;
                let mut u = app.orchestrator.usage.lock().await;
                *u = session.usage;
                app.session_id = resume_name.clone();
                if let Ok(config) = routecode_sdk::utils::storage::load_session_config(&resume_name) {
                    app.orchestrator.allow_session_commands.store(config.allow_all_commands, std::sync::atomic::Ordering::SeqCst);
                    app.orchestrator.allow_session_outside_access.store(config.allow_all_outside_access, std::sync::atomic::Ordering::SeqCst);
                }
            }
            Err(e) => app.history.push(routecode_sdk::core::Message::system(format!("Failed to resume session '{}': {}", resume_name, e))),
        }
    }

    if let Ok(workspace_config) = routecode_sdk::utils::storage::load_workspace_config() {
        if workspace_config.allow_all_outside_access {
            app.orchestrator.allow_session_outside_access.store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    // Signal handling: gracefully exit on SIGINT/SIGTERM
    let (sig_tx, mut sig_rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix;
            let mut term = unix::signal(unix::SignalKind::terminate()).ok();
            let mut int = unix::signal(unix::SignalKind::interrupt()).ok();
            tokio::select! {
                _ = async { if let Some(ref mut s) = term { s.recv().await; } } => {}
                _ = async { if let Some(ref mut s) = int { s.recv().await; } } => {}
            }
        }
        #[cfg(windows)]
        {
            let _ = tokio::signal::ctrl_c().await;
        }
        let _ = sig_tx.try_send(());
    });

    let res = tokio::select! {
        res = run_app(&mut terminal, app) => res,
        _ = sig_rx.recv() => {
            // Signal received, exit gracefully
            Ok(false)
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;

    match res {
        Ok(true) => {
            println!("Starting update process...");
            #[cfg(target_os = "windows")]
            {
                match std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", "irm https://raw.githubusercontent.com/anasx07/routecode/main/install.ps1 | iex"])
                    .status()
                {
                    Ok(status) => {
                        if !status.success() {
                            eprintln!("Update command failed with exit code: {:?}", status.code());
                        }
                    }
                    Err(e) => eprintln!("Failed to run update command: {}", e),
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                match std::process::Command::new("sh")
                    .args(["-c", "curl -fsSL https://raw.githubusercontent.com/anasx07/routecode/main/install.sh | sh"])
                    .status()
                {
                    Ok(status) => {
                        if !status.success() {
                            eprintln!("Update command failed with exit code: {:?}", status.code());
                        }
                    }
                    Err(e) => eprintln!("Failed to run update command: {}", e),
                }
            }
        }
        Err(err) => {
            eprintln!("{:?}", err);
        }
        _ => {}
    }

    // Don't block shutdown on slow update checks — timeout after 1 second
    tokio::time::timeout(std::time::Duration::from_secs(1), update_handle).await.ok();

    Ok(())
}
