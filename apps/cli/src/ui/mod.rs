use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use routecode_sdk::agents::StreamChunk;
use routecode_sdk::core::{AgentOrchestrator, Message, Role};
use routecode_sdk::utils::costs::Usage;
use std::io;
use std::sync::Arc;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
}

const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "openrouter",
        name: "OpenRouter",
    },
    ProviderInfo {
        id: "nvidia",
        name: "NVIDIA",
    },
    ProviderInfo {
        id: "opencode-zen",
        name: "OpenCode Zen",
    },
    ProviderInfo {
        id: "opencode-go",
        name: "OpenCode Go",
    },
    ProviderInfo {
        id: "openai",
        name: "OpenAI",
    },
];

#[derive(Clone)]
pub struct DynamicModelInfo {
    pub name: String,
    pub provider_id: String,
}

pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

const COMMANDS: &[Command] = &[
    Command {
        name: "/model",
        description: "Switch model",
    },
    Command {
        name: "/resume",
        description: "Resume a session",
    },
    Command {
        name: "/sessions",
        description: "List saved sessions",
    },
    Command {
        name: "/clear",
        description: "Clear history",
    },
    Command {
        name: "/help",
        description: "Show help",
    },
    Command {
        name: "/provider",
        description: "Switch provider",
    },
    Command {
        name: "/exit",
        description: "Exit application",
    },
];

#[derive(PartialEq)]
pub enum Screen {
    Welcome,
    Session,
}

pub struct App {
    pub screen: Screen,
    pub input: TextArea<'static>,
    pub history: Vec<Message>,
    pub orchestrator: Arc<AgentOrchestrator>,
    pub current_model: String,
    pub provider_name: String,
    pub show_menu: bool,
    pub show_provider_menu: bool,
    pub show_model_menu: bool,
    pub menu_state: ListState,
    pub filtered_commands: Vec<&'static Command>,
    pub filtered_models: Vec<DynamicModelInfo>,
    pub history_scroll: u16,
    pub is_generating: bool,
    pub tick_count: u64,
    pub active_tool: Option<String>,
    pub api_key_input: TextArea<'static>,
    pub is_inputting_api_key: bool,
    pub pending_provider_id: Option<String>,
}

impl App {
    pub fn new(orchestrator: Arc<AgentOrchestrator>, provider_name: String) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_placeholder_style(Style::default().fg(Color::DarkGray));
        input.set_placeholder_text(" Ask anything... \"How do I use this?\"");

        let mut api_key_input = TextArea::default();
        api_key_input.set_cursor_line_style(Style::default());
        api_key_input.set_placeholder_text(" Paste your API key here...");

        Self {
            screen: Screen::Welcome,
            input,
            history: Vec::new(),
            orchestrator,
            current_model: "gpt-4o".to_string(),
            provider_name,
            show_menu: false,
            show_provider_menu: false,
            show_model_menu: false,
            menu_state: ListState::default(),
            filtered_commands: Vec::new(),
            filtered_models: Vec::new(),
            history_scroll: 0,
            is_generating: false,
            tick_count: 0,
            active_tool: None,
            api_key_input,
            is_inputting_api_key: false,
            pending_provider_id: None,
        }
    }

    pub fn update_filtered_commands(&mut self) {
        let input_line = self.input.lines()[0].to_lowercase();
        if input_line.starts_with('/') {
            self.filtered_commands = COMMANDS
                .iter()
                .filter(|c| c.name.starts_with(&input_line))
                .collect();
            self.show_menu = !self.filtered_commands.is_empty();
            if self.show_menu && self.menu_state.selected().is_none() {
                self.menu_state.select(Some(0));
            }
        } else {
            self.show_menu = false;
        }
    }
}

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut ratatui::Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<StreamChunk>();
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));

    // Dedicated event polling task for maximum responsiveness
    tokio::task::spawn_blocking(move || {
        loop {
            if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                if let Ok(evt) = event::read() {
                    if event_tx.send(evt).is_err() {
                        break;
                    }
                }
            }
        }
    });

    loop {
        let usage = app.orchestrator.usage.lock().await.clone();
        terminal.draw(|f| ui(f, &mut app, &usage))?;

        tokio::select! {
            _ = interval.tick() => {
                if app.is_generating {
                    app.tick_count = app.tick_count.wrapping_add(1);
                } else {
                    continue;
                }
            }
            // Handle streaming chunks from the orchestrator
            Some(chunk) = rx.recv() => {
                match chunk {
                    StreamChunk::Text { content } => {
                        if let Some(last) = app.history.last_mut() {
                            if last.role == Role::Assistant {
                                let mut current = last.content.take().unwrap_or_default();
                                current.push_str(&content);
                                last.content = Some(current);
                            } else {
                                app.history.push(Message::assistant(Some(content), None, None));
                            }
                        } else {
                            app.history.push(Message::assistant(Some(content), None, None));
                        }
                        app.history_scroll = 0;
                    }
                    StreamChunk::Thought { content } => {
                         if let Some(last) = app.history.last_mut() {
                            if last.role == Role::Assistant {
                                let mut current = last.thought.take().unwrap_or_default();
                                current.push_str(&content);
                                last.thought = Some(current);
                            } else {
                                app.history.push(Message::assistant(None, Some(content), None));
                            }
                        } else {
                            app.history.push(Message::assistant(None, Some(content), None));
                        }
                        app.history_scroll = 0;
                    }
                    StreamChunk::Usage { usage: _ } => {}
                    StreamChunk::ToolCall { tool_call } => {
                         app.active_tool = Some(tool_call.function.name.clone());
                         if let Some(last) = app.history.last_mut() {
                            if last.role == Role::Assistant {
                                let mut current = last.tool_calls.take().unwrap_or_default();
                                if let Some(idx) = tool_call.index {
                                    if let Some(existing) = current.iter_mut().find(|tc| tc.index == Some(idx)) {
                                        *existing = tool_call;
                                    } else {
                                        current.push(tool_call);
                                    }
                                } else {
                                    current.push(tool_call);
                                }
                                last.tool_calls = Some(current);
                            } else {
                                app.history.push(Message::assistant(None, None, Some(vec![tool_call])));
                            }
                        } else {
                            app.history.push(Message::assistant(None, None, Some(vec![tool_call])));
                        }
                    }
                    StreamChunk::ToolResult { tool_call_id, name, content } => {
                        app.active_tool = None;
                        app.history.push(Message::tool(tool_call_id, name, content));
                    }
                    StreamChunk::Error { content } => {
                        app.history.push(Message::system(format!("Error: {}", content)));
                        app.is_generating = false;
                    }
                    StreamChunk::Done => {
                        app.is_generating = false;
                    }
                }
            }
            // Handle system events
            Some(event) = event_rx.recv() => {
                match event {
                    Event::Key(key) => {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        match key.code {
                            KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.show_menu = true;
                                app.menu_state.select(Some(0));
                                app.update_filtered_commands();
                            }
                            KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.history.clear();
                                app.screen = Screen::Welcome;
                                app.history_scroll = 0;
                            }
                            KeyCode::Enter => {
                                if app.show_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(cmd) = app.filtered_commands.get(selected) {
                                            app.input = TextArea::from(vec![format!("{} ", cmd.name)]);
                                            app.input.move_cursor(tui_textarea::CursorMove::End);
                                            app.show_menu = false;
                                        }
                                    }
                                } else if app.show_provider_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(p) = PROVIDERS.get(selected) {
                                            app.pending_provider_id = Some(p.id.to_string());
                                            app.is_inputting_api_key = true;
                                            app.api_key_input = TextArea::default();
                                            app.show_provider_menu = false;
                                        }
                                    }
                                } else if app.is_inputting_api_key {
                                    if let Some(provider_id) = app.pending_provider_id.take() {
                                        let api_key = app.api_key_input.lines()[0].to_string();
                                        if !api_key.is_empty() {
                                            let mut config = app.orchestrator.config.lock().await;
                                            config.api_keys.insert(provider_id.clone(), api_key);
                                            let _ = routecode_sdk::utils::storage::save_config(&config);
                                            app.history.push(Message::system(format!("API Key saved for {}", provider_id)));
                                        }
                                    }
                                    app.is_inputting_api_key = false;
                                } else if app.show_model_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(model_info) = app.filtered_models.get(selected).cloned() {
                                            let provider_id = &model_info.provider_id;
                                            let model_name = &model_info.name;
                                            
                                            let mut config = app.orchestrator.config.lock().await;
                                            let env_key = format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                                            let api_key = std::env::var(env_key).ok().or_else(|| config.api_keys.get(provider_id).cloned());

                                            if let Some(key) = api_key {
                                                config.model = model_name.clone();
                                                config.provider = provider_id.clone();
                                                let _ = routecode_sdk::utils::storage::save_config(&config);

                                                if app.provider_name.to_lowercase() != *provider_id {
                                                    let provider = routecode_sdk::agents::resolve_provider(provider_id, key);
                                                    app.provider_name = provider.name().to_string();
                                                    drop(config);
                                                    app.orchestrator.change_provider(provider).await;
                                                } else {
                                                    drop(config);
                                                }

                                                app.current_model = model_name.clone();
                                                app.history.push(Message::system(format!("Switched to {} on {}", model_name, app.provider_name)));
                                            } else {
                                                app.history.push(Message::system(format!("Error: No API key for {}", provider_id)));
                                            }
                                            app.show_model_menu = false;
                                        }
                                    }
                                } else if !app.is_generating {
                                    let user_input = app.input.lines()[0].to_string();
                                    if user_input.is_empty() { continue; }

                                    if user_input.starts_with('/') {
                                        handle_command(&mut app, &user_input).await;
                                        app.input = TextArea::default();
                                    } else {
                                        if app.screen == Screen::Welcome {
                                            app.screen = Screen::Session;
                                        }
                                        app.history.push(Message::user(user_input));
                                        app.input = TextArea::default();
                                        app.history_scroll = 0;

                                        let model = app.current_model.clone();
                                        let orchestrator = app.orchestrator.clone();
                                        let mut history = app.history.clone();
                                        let tx_clone = tx.clone();
                                        
                                        app.is_generating = true;
                                        tokio::spawn(async move {
                                            let _ = orchestrator.run(&mut history, &model, Some(tx_clone)).await;
                                        });
                                    }
                                }
                            }
                            KeyCode::Char(' ') if app.show_provider_menu => {
                                if let Some(selected) = app.menu_state.selected() {
                                    if let Some(p) = PROVIDERS.get(selected) {
                                        let mut config = app.orchestrator.config.lock().await;
                                        if config.api_keys.contains_key(p.id) {
                                            config.api_keys.remove(p.id);
                                            app.history.push(Message::system(format!("Disconnected {}", p.name)));
                                            let _ = routecode_sdk::utils::storage::save_config(&config);
                                        } else {
                                            app.history.push(Message::system(format!("To connect {}, set {}_API_KEY env var", p.name, p.id.to_uppercase().replace("-", "_"))));
                                        }
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                if app.show_menu { app.show_menu = false; }
                                else if app.show_provider_menu { app.show_provider_menu = false; }
                                else if app.show_model_menu { app.show_model_menu = false; }
                                else if app.is_inputting_api_key { app.is_inputting_api_key = false; }
                                else {
                                    if !app.history.is_empty() {
                                        let session = routecode_sdk::utils::storage::Session {
                                            messages: app.history.clone(),
                                            usage: app.orchestrator.usage.lock().await.clone(),
                                            model: app.current_model.clone(),
                                            timestamp: chrono::Utc::now().timestamp(),
                                        };
                                        let _ = routecode_sdk::utils::storage::save_session("last_session", &session);
                                    }
                                    return Ok(());
                                }
                            }
                            KeyCode::Up => {
                                if app.show_menu || app.show_provider_menu || app.show_model_menu {
                                    let items_len = if app.show_menu { app.filtered_commands.len() } 
                                                   else if app.show_provider_menu { PROVIDERS.len() }
                                                   else { app.filtered_models.len() };
                                    if items_len > 0 {
                                        let selected = app.menu_state.selected().unwrap_or(0);
                                        let new_selected = if selected == 0 { items_len - 1 } else { selected - 1 };
                                        app.menu_state.select(Some(new_selected));
                                    }
                                } else {
                                    app.history_scroll = app.history_scroll.saturating_sub(1);
                                }
                            }
                            KeyCode::Down => {
                                if app.show_menu || app.show_provider_menu || app.show_model_menu {
                                    let items_len = if app.show_menu { app.filtered_commands.len() } 
                                                   else if app.show_provider_menu { PROVIDERS.len() }
                                                   else { app.filtered_models.len() };
                                    if items_len > 0 {
                                        let selected = app.menu_state.selected().unwrap_or(0);
                                        let new_selected = if selected >= items_len - 1 { 0 } else { selected + 1 };
                                        app.menu_state.select(Some(new_selected));
                                    }
                                } else {
                                    app.history_scroll = app.history_scroll.saturating_add(1);
                                }
                            }
                            _ => {
                                if app.is_inputting_api_key {
                                    app.api_key_input.input(event);
                                } else {
                                    app.input.input(event);
                                    app.update_filtered_commands();
                                }
                            }
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::ScrollUp => { app.history_scroll = app.history_scroll.saturating_sub(2); }
                            MouseEventKind::ScrollDown => { app.history_scroll = app.history_scroll.saturating_add(2); }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn handle_command(app: &mut App, input: &str) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() { return; }
    let command = parts[0];
    let args = &parts[1..];

    match command {
        "/model" => {
            app.history.push(Message::system("Fetching available models..."));
            let mut all_models = Vec::new();
            let config = app.orchestrator.config.lock().await.clone();
            for p_info in PROVIDERS {
                let env_key = format!("{}_API_KEY", p_info.id.to_uppercase().replace("-", "_"));
                let api_key = std::env::var(env_key).ok().or_else(|| config.api_keys.get(p_info.id).cloned());
                if let Some(key) = api_key {
                    let provider = routecode_sdk::agents::resolve_provider(p_info.id, key);
                    if let Ok(models) = provider.list_models().await {
                        for m_name in models {
                            all_models.push(DynamicModelInfo { name: m_name, provider_id: p_info.id.to_string() });
                        }
                    }
                }
            }
            if all_models.is_empty() {
                app.history.push(Message::system("No models found. Ensure providers are connected."));
            } else {
                app.filtered_models = all_models;
                app.show_model_menu = true;
                app.menu_state.select(Some(0));
            }
        }
        "/resume" => {
            if let Some(name) = args.first() {
                if let Ok(session) = routecode_sdk::utils::storage::load_session(name) {
                    app.history = session.messages;
                    app.current_model = session.model;
                    let mut u = app.orchestrator.usage.lock().await;
                    *u = session.usage;
                    app.history.push(Message::system(format!("Session resumed: {}", name)));
                    app.screen = Screen::Session;
                }
            }
        }
        "/sessions" => {
            if let Ok(sessions) = routecode_sdk::utils::storage::list_sessions() {
                if sessions.is_empty() { app.history.push(Message::system("No saved sessions found.")); }
                else { app.history.push(Message::system(format!("Saved sessions:\n  {}", sessions.join("\n  ")))); }
            }
        }
        "/clear" => { app.history.clear(); app.screen = Screen::Welcome; }
        "/help" => {
            app.history.push(Message::system("Available commands:\n  /model           - Select model\n  /provider        - Manage connections\n  /resume <name>   - Resume session\n  /sessions        - List sessions\n  /clear           - Clear history\n  /help            - Show help\n  /exit            - Use Esc to exit"));
        }
        "/provider" => { app.show_provider_menu = true; app.menu_state.select(Some(0)); }
        _ => { app.history.push(Message::system(format!("Unknown command: {}", command))); }
    }
}

fn ui(f: &mut Frame, app: &mut App, usage: &Usage) {
    let input_area = match app.screen {
        Screen::Welcome => ui_welcome(f, app),
        Screen::Session => ui_session(f, app, usage),
    };

    if app.show_menu {
        render_menu(f, app, input_area);
    } else if app.show_provider_menu {
        render_provider_menu(f, app, input_area);
    } else if app.show_model_menu {
        render_model_menu(f, app, input_area);
    } else if app.is_inputting_api_key {
        render_api_key_dialog(f, app);
    }
}

fn render_api_key_dialog(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let width = 60;
    let height = 7;
    let dialog_area = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    f.render_widget(Clear, dialog_area);

    let provider_name = app.pending_provider_id.as_deref().unwrap_or("Provider");
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Connect {} ", provider_name))
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(inner_area);

    f.render_widget(
        Paragraph::new(format!("Enter API key for {}:", provider_name)),
        layout[0],
    );

    app.api_key_input.set_block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(app.api_key_input.widget(), layout[1]);

    let (row, col) = app.api_key_input.cursor();
    f.set_cursor(layout[1].x + 1 + col as u16, layout[1].y + 1 + row as u16);

    f.render_widget(
        Paragraph::new(" Press Enter to save | Esc to cancel ")
            .style(Style::default().fg(Color::DarkGray)),
        layout[2],
    );
}

fn render_provider_menu(f: &mut Frame, app: &App, input_area: Rect) {

    let area = f.size();
    let menu_width = 60;
    let menu_height = (PROVIDERS.len() + 2).min(10) as u16;
    let menu_area = Rect::new((area.width.saturating_sub(menu_width)) / 2, input_area.y.saturating_sub(menu_height), menu_width, menu_height);
    f.render_widget(Clear, menu_area);

    let config = futures::executor::block_on(app.orchestrator.config.lock());
    let items: Vec<ListItem> = PROVIDERS.iter().map(|p| {
        let env_key = format!("{}_API_KEY", p.id.to_uppercase().replace("-", "_"));
        let is_connected = config.api_keys.contains_key(p.id) || std::env::var(env_key).is_ok();
        let status = if is_connected { Span::styled(" ✔ connected", Style::default().fg(Color::Green)) }
                    else { Span::styled(" ✖ disconnected", Style::default().fg(Color::DarkGray)) };
        ListItem::new(Line::from(vec![Span::raw(format!("{:<15}", p.name)), status]))
    }).collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Providers (Space to Toggle)").border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black)).highlight_symbol(">> ");
    let mut state = app.menu_state.clone();
    f.render_stateful_widget(list, menu_area, &mut state);
}

fn render_model_menu(f: &mut Frame, app: &App, input_area: Rect) {
    let area = f.size();
    let menu_width = 60;
    let menu_height = (app.filtered_models.len() + 2).min(15) as u16;
    let menu_area = Rect::new((area.width.saturating_sub(menu_width)) / 2, input_area.y.saturating_sub(menu_height), menu_width, menu_height);
    f.render_widget(Clear, menu_area);

    let items: Vec<ListItem> = app.filtered_models.iter().map(|m| ListItem::new(format!("{:<30} ({})", m.name, m.provider_id))).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Models").border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black)).highlight_symbol(">> ");
    let mut state = app.menu_state.clone();
    f.render_stateful_widget(list, menu_area, &mut state);
}

fn ui_welcome(f: &mut Frame, app: &mut App) -> Rect {
    let area = f.size();
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(area.height / 3), Constraint::Length(6), Constraint::Length(3),
        Constraint::Length(2), Constraint::Min(0), Constraint::Length(1),
    ]).split(area);

    let logo_text = vec![
        Line::from(Span::styled("  ____             _        ____          _      ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" |  _ \\ ___  _   _| |_ ___ / ___|___   __| | ___ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" | |_) / _ \\| | | | __/ _ \\ |   / _ \\ / _` |/ _ \\", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" |  _ < (_) | |_| | ||  __/ |__| (_) | (_| |  __/", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(" |_| \\_\\___/ \\__,_|\\__\\___|\\____\\___/ \\__,_|\\___|", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
    ];
    f.render_widget(Paragraph::new(logo_text).alignment(ratatui::layout::Alignment::Center), chunks[1]);

    let input_width = (area.width as f32 * 0.6) as u16;
    let input_area = Rect::new((area.width - input_width) / 2, chunks[2].y, input_width, 3);
    
    app.input.set_block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(app.input.widget(), input_area);

    let (row, col) = app.input.cursor();
    if !app.is_generating {
        f.set_cursor(input_area.x + 1 + col as u16, input_area.y + 1 + row as u16);
    }

    let provider_info = format!(" Model: {} | Provider: {} ", app.current_model, app.provider_name);
    f.render_widget(Paragraph::new(provider_info).alignment(ratatui::layout::Alignment::Center).style(Style::default().fg(Color::DarkGray)), chunks[3]);

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];
    let tip_text = if app.is_generating { format!(" {} AI is working... ", frame) } else { " Tip: Use /help to see all commands | Esc to exit ".to_string() };
    f.render_widget(Paragraph::new(tip_text).alignment(ratatui::layout::Alignment::Center).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::DIM)), chunks[5]);

    input_area
}

fn ui_session(f: &mut Frame, app: &mut App, usage: &Usage) -> Rect {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(1), Constraint::Length(3), Constraint::Length(1)]).split(f.size());
    let history = render_history(app);
    f.render_widget(Paragraph::new(history).wrap(Wrap { trim: true }).scroll((app.history_scroll, 0)), chunks[0]);

    app.input.set_block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(app.input.widget(), chunks[1]);

    let (row, col) = app.input.cursor();
    if !app.is_generating {
        f.set_cursor(chunks[1].x + col as u16, chunks[1].y + 1 + row as u16);
    }

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];
    let generating_text = if app.is_generating {
        if let Some(tool) = &app.active_tool { format!(" {} [Running {}...] ", frame, tool) }
        else { format!(" {} [Thinking...] ", frame) }
    } else { "".to_string() };

    let status_text = format!(" {} | Tokens: {} | Cost: ${:.4}{} ", app.current_model, usage.total_tokens, usage.total_cost, generating_text);
    let status_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(0), Constraint::Length(20)]).split(chunks[2]);
    f.render_widget(Paragraph::new(status_text).style(Style::default().fg(Color::DarkGray)), status_layout[0]);
    f.render_widget(Paragraph::new(" ctrl+p commands ").alignment(ratatui::layout::Alignment::Right).style(Style::default().fg(Color::DarkGray)), status_layout[1]);

    chunks[1]
}

fn render_history(app: &App) -> Text<'_> {
    let mut lines = Vec::new();
    for m in &app.history {
        match m.role {
            Role::User => {
                lines.push(Line::from(vec![
                    Span::styled(" User ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(m.content.as_deref().unwrap_or(""), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                ]));
            }
            Role::Assistant => {
                lines.push(Line::from(vec![Span::styled(" AI   ", Style::default().bg(Color::Cyan).fg(Color::Black).add_modifier(Modifier::BOLD))]));
                if let Some(thought) = &m.thought {
                    lines.push(Line::from(vec![Span::styled("   Thinking: ", Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))]));
                    for line in thought.lines() { lines.push(Line::from(vec![Span::styled(format!("   {}", line), Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))])); }
                }
                if let Some(tool_calls) = &m.tool_calls {
                    for tc in tool_calls { lines.push(Line::from(vec![Span::styled(format!("   🛠️  {} ", tc.function.name), Style::default().fg(Color::Magenta)), Span::styled(format!("({})", tc.function.arguments), Style::default().fg(Color::DarkGray))])); }
                }
                if let Some(content) = &m.content {
                    for line in content.lines() {
                        if line.trim().starts_with("```") { lines.push(Line::from(vec![Span::raw("   "), Span::styled(line, Style::default().fg(Color::Cyan))])); }
                        else { lines.push(Line::from(vec![Span::raw("   "), Span::raw(line)])); }
                    }
                }
            }
            Role::Tool => {
                lines.push(Line::from(vec![
                    Span::styled(format!(" TOOL ({}) ", m.name.as_deref().unwrap_or("tool")), Style::default().bg(Color::Magenta).fg(Color::Black).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(m.content.as_deref().unwrap_or(""), Style::default().fg(Color::DarkGray)),
                ]));
            }
            Role::System => {
                lines.push(Line::from(vec![
                    Span::styled(" SYS  ", Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD)),
                    Span::raw(" "),
                    Span::styled(m.content.as_deref().unwrap_or(""), Style::default().fg(Color::Yellow)),
                ]));
            }
        }
        lines.push(Line::from(""));
    }
    Text::from(lines)
}

fn render_menu(f: &mut Frame, app: &App, input_area: Rect) {
    let area = f.size();
    let menu_width = 60;
    let menu_height = (app.filtered_commands.len() + 2).min(10) as u16;
    let menu_area = Rect::new((area.width.saturating_sub(menu_width)) / 2, input_area.y.saturating_sub(menu_height), menu_width, menu_height);
    f.render_widget(Clear, menu_area);

    let items: Vec<ListItem> = app.filtered_commands.iter().map(|cmd| ListItem::new(format!("{:<15} {}", cmd.name, cmd.description))).collect();
    let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Commands").border_style(Style::default().fg(Color::Cyan)))
        .highlight_style(Style::default().bg(Color::Cyan).fg(Color::Black)).highlight_symbol(">> ");
    let mut state = app.menu_state.clone();
    f.render_stateful_widget(list, menu_area, &mut state);
}
