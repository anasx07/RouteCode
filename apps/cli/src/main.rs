use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, help = "Model to use")]
    pub model: Option<String>,

    #[arg(
        long,
        help = "Provider (openrouter, openai, anthropic, google, deepseek)"
    )]
    pub provider: Option<String>,

    #[arg(short, long, help = "Resume a saved session by name")]
    pub resume: Option<String>,

    #[arg(long, help = "Run a single query and print the result (headless)")]
    pub print: bool,

    #[arg(long, help = "Check for and install the latest version of RouteCode")]
    pub update: bool,

    #[arg(
        short,
        long,
        help = "Development mode: opens log window at DEBUG level"
    )]
    pub debug: bool,

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
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use routecode_sdk::core::AgentOrchestrator;
use routecode_sdk::tools::bash::BashTool;
use routecode_sdk::tools::file_ops::{FileEditTool, FileReadTool, FileWriteTool};
use routecode_sdk::tools::navigation::{GrepTool, LsTool};
use routecode_sdk::tools::ToolRegistry;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use ui::{run_app, App};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::Version) = cli.command {
        println!("routecode {}", env!("CARGO_PKG_VERSION"));
        println!("Rust based");
        return Ok(());
    }

    // Initialize logic
    let mut config = routecode_sdk::utils::storage::load_config().unwrap_or_default();

    // Override from CLI
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

    // Choose provider
    let provider = routecode_sdk::agents::resolve_provider(&provider_name, api_key);

    let mut tool_registry = ToolRegistry::new();
    tool_registry.register(Arc::new(FileReadTool));
    tool_registry.register(Arc::new(FileWriteTool));
    tool_registry.register(Arc::new(FileEditTool));
    tool_registry.register(Arc::new(BashTool));
    tool_registry.register(Arc::new(LsTool));
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new(orchestrator, config.provider.clone());
    app.current_model = config.model;

    if let Some(resume_name) = cli.resume {
        // Automatically handle resume if specified
        match routecode_sdk::utils::storage::load_session(&resume_name) {
            Ok(session) => {
                app.history = session.messages;
                app.current_model = session.model;
                let mut u = app.orchestrator.usage.lock().await;
                *u = session.usage;
            }
            Err(e) => eprintln!("Failed to resume session: {}", e),
        }
    }

    let res = run_app(&mut terminal, app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("{:?}", err)
    }

    Ok(())
}
