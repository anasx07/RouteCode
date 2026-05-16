use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use routecode_sdk::agents::StreamChunk;
use routecode_sdk::core::{AgentOrchestrator, Message, Role, DynamicModelInfo};
use routecode_sdk::utils::costs::Usage;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tui_textarea::TextArea;

// --- Theme ---
const COLOR_PRIMARY: Color = Color::Rgb(0, 150, 255); // Ocean Blue
const COLOR_BG: Color = Color::Rgb(25, 25, 25);      // Midnight Charcoal
const COLOR_INPUT_BG: Color = Color::Rgb(35, 35, 35);// Soft Obsidian
const COLOR_SECONDARY: Color = Color::DarkGray;      // Slate Gray
const COLOR_SYSTEM: Color = Color::Yellow;           // Amber Yellow
const COLOR_SUCCESS: Color = Color::Green;           // Emerald Green
const COLOR_TEXT: Color = Color::White;              // Primary Text
const COLOR_DIM: Color = Color::Rgb(50, 50, 50);      // Very Dim Text/Lines

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
}

const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo { id: "openrouter", name: "OpenRouter" },
    ProviderInfo { id: "nvidia", name: "NVIDIA" },
    ProviderInfo { id: "opencode-zen", name: "OpenCode Zen" },
    ProviderInfo { id: "opencode-go", name: "OpenCode Go" },
    ProviderInfo { id: "openai", name: "OpenAI" },
    ProviderInfo { id: "anthropic", name: "Anthropic" },
    ProviderInfo { id: "gemini", name: "Google Gemini" },
    ProviderInfo { id: "deepseek", name: "DeepSeek" },
    ProviderInfo { id: "cloudflare-workers", name: "Cloudflare Workers AI" },
    ProviderInfo { id: "cloudflare-gateway", name: "Cloudflare AI Gateway" },
];

#[derive(Clone, Debug)]
pub enum ModelMenuItem {
    Header(String),
    Model(DynamicModelInfo),
}

pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
}

const COMMANDS: &[Command] = &[
    Command { name: "/model", description: "Switch model" },
    Command { name: "/resume", description: "Resume a session" },
    Command { name: "/sessions", description: "List saved sessions" },
    Command { name: "/clear", description: "Clear history" },
    Command { name: "/help", description: "Show help" },
    Command { name: "/stop", description: "Stop AI generation" },
    Command { name: "/provider", description: "Manage providers" },
    Command { name: "/exit", description: "Exit application" },
];

#[derive(Debug, PartialEq)]
pub enum Screen {
    Welcome,
    Session,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ApiKeyInputStage {
    None,
    ApiKey,
    CloudflareAccountId,
    CloudflareGatewayId,
    CloudflareApiKey,
}

pub struct App {
    pub screen: Screen,
    pub input: TextArea<'static>,
    pub history: Vec<Message>,
    pub orchestrator: Arc<AgentOrchestrator>,
    pub current_model: String,
    pub current_provider_id: String,
    pub provider_name: String,
    pub show_menu: bool,
    pub show_provider_menu: bool,
    pub show_model_menu: bool,
    pub menu_state: ListState,
    pub filtered_commands: Vec<&'static Command>,
    pub filtered_models: Vec<ModelMenuItem>,
    pub all_available_models: Vec<DynamicModelInfo>,
    pub history_scroll: u16,
    pub is_generating: bool,
    pub tick_count: u64,
    pub active_tool: Option<String>,
    pub current_task: Option<tokio::task::JoinHandle<()>>,
    pub prompt_history: Vec<String>,
    pub prompt_history_index: Option<usize>,
    pub api_key_input: TextArea<'static>,
    pub model_search_input: TextArea<'static>,
    pub is_inputting_api_key: bool,
    pub pending_provider_id: Option<String>,
    pub api_key_input_stage: ApiKeyInputStage,
    pub pending_account_id: Option<String>,
    pub pending_gateway_id: Option<String>,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<StreamChunk>,
    pub tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>,
}

impl App {
    pub fn new(orchestrator: Arc<AgentOrchestrator>, provider_name: String) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_placeholder_style(Style::default().fg(COLOR_SECONDARY));
        input.set_placeholder_text(" Ask anything... \"How do I use this?\"");

        let mut api_key_input = TextArea::default();
        api_key_input.set_cursor_line_style(Style::default());
        api_key_input.set_placeholder_text(" Paste your API key here...");

        let mut model_search_input = TextArea::default();
        model_search_input.set_cursor_line_style(Style::default());
        model_search_input.set_placeholder_text(" Search models...");
        model_search_input.set_placeholder_style(Style::default().fg(COLOR_SECONDARY));

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            screen: Screen::Welcome,
            input,
            history: Vec::new(),
            orchestrator,
            current_model: "gpt-4o".to_string(),
            current_provider_id: provider_name.clone(),
            provider_name,
            show_menu: false,
            show_provider_menu: false,
            show_model_menu: false,
            menu_state: ListState::default(),
            filtered_commands: Vec::new(),
            filtered_models: Vec::new(),
            all_available_models: Vec::new(),
            history_scroll: 0,
            is_generating: false,
            tick_count: 0,
            active_tool: None,
            current_task: None,
            prompt_history: Vec::new(),
            prompt_history_index: None,
            api_key_input,
            model_search_input,
            is_inputting_api_key: false,
            pending_provider_id: None,
            api_key_input_stage: ApiKeyInputStage::None,
            pending_account_id: None,
            pending_gateway_id: None,
            rx,
            tx,
        }
    }

    pub fn update_filtered_commands(&mut self) {
        let input_line = self.input.lines()[0].to_lowercase();
        if input_line.starts_with('/') {
            self.filtered_commands = COMMANDS
                .iter()
                .filter(|c| c.name.to_lowercase().starts_with(&input_line))
                .collect();
            self.show_menu = !self.filtered_commands.is_empty();
            if self.show_menu {
                self.menu_state.select(Some(0));
            }
        } else {
            self.show_menu = false;
        }
    }
}

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(100);

    loop {
        let usage = app.orchestrator.usage.lock().await.clone();
        terminal.draw(|f| ui(f, &mut app, &usage))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| std::time::Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.show_menu = true;
                                app.menu_state.select(Some(0));
                                app.update_filtered_commands();
                            }
                            KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                if app.show_model_menu { app.show_model_menu = false; }
                                app.show_provider_menu = true;
                                app.menu_state.select(Some(0));
                            }
                            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                if app.is_generating {
                                    if let Some(handle) = app.current_task.take() { handle.abort(); }
                                    app.is_generating = false;
                                    app.active_tool = None;
                                }
                            }
                            KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                                app.history.clear();
                                app.screen = Screen::Welcome;
                                app.history_scroll = 0;
                            }
                            KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::SHIFT) => {
                                app.input.insert_newline();
                            }
                            KeyCode::Enter => {
                                if app.show_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(cmd) = app.filtered_commands.get(selected) {
                                            let name = cmd.name.to_string();
                                            app.show_menu = false;
                                            app.input = TextArea::default();
                                            handle_command(&mut app, &name).await;
                                        }
                                    }
                                } else if app.show_provider_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(p) = PROVIDERS.get(selected) {
                                            app.pending_provider_id = Some(p.id.to_string());
                                            app.is_inputting_api_key = true;
                                            app.api_key_input = TextArea::default();
                                            app.show_provider_menu = false;
                                            if p.id == "cloudflare-workers" || p.id == "cloudflare-gateway" {
                                                app.api_key_input_stage = ApiKeyInputStage::CloudflareAccountId;
                                            } else {
                                                app.api_key_input_stage = ApiKeyInputStage::ApiKey;
                                            }
                                        }
                                    }
                                } else if app.show_model_menu {
                                    if let Some(selected) = app.menu_state.selected() {
                                        if let Some(ModelMenuItem::Model(model_info)) = app.filtered_models.get(selected).cloned() {
                                            let provider_id = &model_info.provider_id;
                                            let model_name = &model_info.name;
                                            let mut config = app.orchestrator.config.lock().await;
                                            let env_key = format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                                            let api_key = std::env::var(env_key).ok().or_else(|| config.api_keys.get(provider_id).cloned());
                                            if let Some(key) = api_key {
                                                config.model = model_name.clone();
                                                config.provider = provider_id.clone();
                                                config.recent_models.retain(|m| m.name != *model_name || m.provider_id != *provider_id);
                                                config.recent_models.insert(0, model_info.clone());
                                                config.recent_models.truncate(3);
                                                let _ = routecode_sdk::utils::storage::save_config(&config);
                                                if app.provider_name.to_lowercase() != *provider_id {
                                                    let provider = routecode_sdk::agents::resolve_provider(provider_id, key);
                                                    app.provider_name = provider.name().to_string();
                                                    app.current_provider_id = provider_id.clone();
                                                    drop(config);
                                                    app.orchestrator.change_provider(provider).await;
                                                } else { drop(config); }
                                                app.current_model = model_name.clone();
                                                app.history.push(Message::system(format!("Switched to {} on {}", model_name, app.provider_name)));
                                                app.show_model_menu = false;
                                            } else {
                                                app.history.push(Message::system(format!("Error: No API key for {}", provider_id)));
                                            }
                                        }
                                    }
                                } else if app.is_inputting_api_key {
                                    let input_value = app.api_key_input.lines().join("\n").trim().to_string();
                                    if !input_value.is_empty() {
                                        match app.api_key_input_stage {
                                            ApiKeyInputStage::ApiKey => {
                                                if let Some(provider_id) = app.pending_provider_id.take() {
                                                    let mut config = app.orchestrator.config.lock().await;
                                                    config.api_keys.insert(provider_id.clone(), input_value);
                                                    let _ = routecode_sdk::utils::storage::save_config(&config);
                                                    app.history.push(Message::system(format!("API Key saved for {}", provider_id)));
                                                }
                                                app.is_inputting_api_key = false;
                                                app.api_key_input_stage = ApiKeyInputStage::None;
                                            }
                                            ApiKeyInputStage::CloudflareAccountId => {
                                                app.pending_account_id = Some(input_value);
                                                app.api_key_input = TextArea::default();
                                                if app.pending_provider_id.as_deref() == Some("cloudflare-gateway") {
                                                    app.api_key_input_stage = ApiKeyInputStage::CloudflareGatewayId;
                                                } else { app.api_key_input_stage = ApiKeyInputStage::CloudflareApiKey; }
                                            }
                                            ApiKeyInputStage::CloudflareGatewayId => {
                                                app.pending_gateway_id = Some(input_value);
                                                app.api_key_input = TextArea::default();
                                                app.api_key_input_stage = ApiKeyInputStage::CloudflareApiKey;
                                            }
                                            ApiKeyInputStage::CloudflareApiKey => {
                                                if let Some(provider_id) = app.pending_provider_id.take() {
                                                    let account_id = app.pending_account_id.take().unwrap_or_default();
                                                    let final_key = if provider_id == "cloudflare-gateway" {
                                                        let gateway_id = app.pending_gateway_id.take().unwrap_or_default();
                                                        format!("{}:{}:{}", account_id, gateway_id, input_value)
                                                    } else { format!("{}:{}", account_id, input_value) };
                                                    let mut config = app.orchestrator.config.lock().await;
                                                    config.api_keys.insert(provider_id.clone(), final_key);
                                                    let _ = routecode_sdk::utils::storage::save_config(&config);
                                                    app.history.push(Message::system(format!("Credentials saved for {}", provider_id)));
                                                }
                                                app.is_inputting_api_key = false;
                                                app.api_key_input_stage = ApiKeyInputStage::None;
                                            }
                                            _ => { app.is_inputting_api_key = false; }
                                        }
                                    } else {
                                        app.is_inputting_api_key = false;
                                        app.api_key_input_stage = ApiKeyInputStage::None;
                                    }
                                } else {
                                    let input_text = app.input.lines().join("\n");
                                    if !input_text.trim().is_empty() {
                                        if input_text.starts_with('/') {
                                            handle_command(&mut app, &input_text).await;
                                        } else {
                                            app.history.push(Message::user(input_text.clone()));
                                            app.prompt_history.push(input_text.clone());
                                            app.prompt_history_index = None;
                                            app.input = TextArea::default();
                                            app.screen = Screen::Session;
                                            app.is_generating = true;
                                            let orchestrator = app.orchestrator.clone();
                                            let mut history = app.history.clone();
                                            let model = app.current_model.clone();
                                            let tx = app.tx.clone();
                                            let task = tokio::spawn(async move {
                                                let _ = orchestrator.run(&mut history, &model, Some(tx)).await;
                                            });
                                            app.current_task = Some(task);
                                        }
                                        app.input = TextArea::default();
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                if app.show_menu { app.show_menu = false; }
                                else if app.show_provider_menu { app.show_provider_menu = false; }
                                else if app.show_model_menu { app.show_model_menu = false; }
                                else if app.is_inputting_api_key {
                                    app.is_inputting_api_key = false;
                                    app.api_key_input_stage = ApiKeyInputStage::None;
                                    app.pending_account_id = None;
                                    app.pending_gateway_id = None;
                                } else if app.is_generating {
                                    if let Some(handle) = app.current_task.take() { handle.abort(); }
                                    app.is_generating = false;
                                    app.active_tool = None;
                                } else {
                                    if !app.history.is_empty() {
                                        let session = routecode_sdk::utils::storage::Session {
                                            messages: app.history.clone(),
                                            model: app.current_model.clone(),
                                            usage: app.orchestrator.usage.lock().await.clone(),
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
                                        let mut new_selected = if selected == 0 { items_len - 1 } else { selected - 1 };
                                        if app.show_model_menu {
                                            while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(new_selected) {
                                                new_selected = if new_selected == 0 { items_len - 1 } else { new_selected - 1 };
                                                if new_selected == selected { break; }
                                            }
                                        }
                                        app.menu_state.select(Some(new_selected));
                                    }
                                } else if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                    app.history_scroll = app.history_scroll.saturating_sub(1);
                                } else {
                                    let (row, _) = app.input.cursor();
                                    if row == 0 && !app.prompt_history.is_empty() {
                                        let idx = match app.prompt_history_index {
                                            Some(i) => if i == 0 { 0 } else { i - 1 },
                                            None => app.prompt_history.len() - 1,
                                        };
                                        app.prompt_history_index = Some(idx);
                                        let prev = app.prompt_history[idx].clone();
                                        app.input = TextArea::from(prev.lines().map(|s| s.to_string()));
                                        app.input.move_cursor(tui_textarea::CursorMove::End);
                                    } else { app.input.input(key); }
                                }
                            }
                            KeyCode::Down => {
                                if app.show_menu || app.show_provider_menu || app.show_model_menu {
                                    let items_len = if app.show_menu { app.filtered_commands.len() }
                                                   else if app.show_provider_menu { PROVIDERS.len() }
                                                   else { app.filtered_models.len() };
                                    if items_len > 0 {
                                        let selected = app.menu_state.selected().unwrap_or(0);
                                        let mut new_selected = if selected >= items_len - 1 { 0 } else { selected + 1 };
                                        if app.show_model_menu {
                                            while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(new_selected) {
                                                new_selected = if new_selected >= items_len - 1 { 0 } else { new_selected + 1 };
                                                if new_selected == selected { break; }
                                            }
                                        }
                                        app.menu_state.select(Some(new_selected));
                                    }
                                } else if key.modifiers.contains(event::KeyModifiers::SHIFT) {
                                    app.history_scroll = app.history_scroll.saturating_add(1);
                                } else {
                                    let (row, _) = app.input.cursor();
                                    let lines_len = app.input.lines().len();
                                    if row >= lines_len - 1 && app.prompt_history_index.is_some() {
                                        let idx = app.prompt_history_index.unwrap();
                                        if idx >= app.prompt_history.len() - 1 {
                                            app.prompt_history_index = None;
                                            app.input = TextArea::default();
                                        } else {
                                            let new_idx = idx + 1;
                                            app.prompt_history_index = Some(new_idx);
                                            let next = app.prompt_history[new_idx].clone();
                                            app.input = TextArea::from(next.lines().map(|s| s.to_string()));
                                            app.input.move_cursor(tui_textarea::CursorMove::End);
                                        }
                                    } else { app.input.input(key); }
                                }
                            }
                            KeyCode::Right if app.show_model_menu => {
                                let len = app.filtered_models.len();
                                if len > 0 {
                                    let current = app.menu_state.selected().unwrap_or(0);
                                    let mut next_header_idx = None;
                                    for i in (current + 1)..len {
                                        if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) {
                                            next_header_idx = Some(i); break;
                                        }
                                    }
                                    if next_header_idx.is_none() {
                                        for i in 0..current {
                                            if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) {
                                                next_header_idx = Some(i); break;
                                            }
                                        }
                                    }
                                    if let Some(h_idx) = next_header_idx {
                                        let mut target = (h_idx + 1) % len;
                                        while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) {
                                            target = (target + 1) % len;
                                            if target == h_idx { break; }
                                        }
                                        app.menu_state.select(Some(target));
                                    }
                                }
                            }
                            KeyCode::Left if app.show_model_menu => {
                                let len = app.filtered_models.len();
                                if len > 0 {
                                    let current = app.menu_state.selected().unwrap_or(0);
                                    let mut headers = Vec::new();
                                    for (i, item) in app.filtered_models.iter().enumerate() {
                                        if let ModelMenuItem::Header(_) = item { headers.push(i); }
                                    }
                                    if !headers.is_empty() {
                                        let current_header_idx_in_headers = headers.iter().enumerate().rev().find(|(_, &h_idx)| h_idx < current).map(|(i, _)| i);
                                        let target_header_idx = match current_header_idx_in_headers {
                                            Some(i) => if i == 0 { *headers.last().unwrap() } else { headers[i - 1] },
                                            None => *headers.last().unwrap()
                                        };
                                        let mut target = (target_header_idx + 1) % len;
                                        while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) {
                                            target = (target + 1) % len;
                                            if target == target_header_idx { break; }
                                        }
                                        app.menu_state.select(Some(target));
                                    }
                                }
                            }
                            KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) && app.show_model_menu => {
                                if let Some(selected) = app.menu_state.selected() {
                                    if let Some(ModelMenuItem::Model(model_info)) = app.filtered_models.get(selected).cloned() {
                                        let mut config = app.orchestrator.config.lock().await;
                                        if config.favorites.iter().any(|m| m.name == model_info.name && m.provider_id == model_info.provider_id) {
                                            config.favorites.retain(|m| m.name != model_info.name || m.provider_id != model_info.provider_id);
                                            app.history.push(Message::system(format!("Removed {} from favorites", model_info.name)));
                                        } else {
                                            config.favorites.push(model_info.clone());
                                            app.history.push(Message::system(format!("Added {} to favorites", model_info.name)));
                                        }
                                        let _ = routecode_sdk::utils::storage::save_config(&config);
                                    }
                                }
                            }
                            _ => {
                                let event = event::Event::Key(key);
                                if app.is_inputting_api_key {
                                    app.api_key_input.input(event);
                                } else if app.show_model_menu {
                                    if app.model_search_input.input(event) {
                                        let search = app.model_search_input.lines()[0].to_lowercase().trim().to_string();
                                        handle_model_search(&mut app, &search, true).await;
                                    }
                                } else {
                                    app.input.input(event);
                                    app.update_filtered_commands();
                                }
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

        if last_tick.elapsed() >= tick_rate {
            app.tick_count += 1;
            last_tick = std::time::Instant::now();
        }

        while let Ok(chunk) = app.rx.try_recv() {
            match chunk {
                StreamChunk::Text { content } => {
                    if let Some(last) = app.history.last_mut() {
                        if last.role == Role::Assistant {
                            let mut current = last.content.clone().unwrap_or_default();
                            current.push_str(&content);
                            last.content = Some(current);
                        } else {
                            app.history.push(Message::assistant(Some(content), None, None));
                        }
                    } else {
                        app.history.push(Message::assistant(Some(content), None, None));
                    }
                }
                StreamChunk::Thought { content } => {
                    if let Some(last) = app.history.last_mut() {
                        if last.role == Role::Assistant {
                            let mut current = last.thought.clone().unwrap_or_default();
                            current.push_str(&content);
                            last.thought = Some(current);
                        } else {
                            app.history.push(Message::assistant(None, Some(content), None));
                        }
                    } else {
                        app.history.push(Message::assistant(None, Some(content), None));
                    }
                }
                StreamChunk::ToolCall { tool_call } => {
                    app.active_tool = Some(tool_call.function.name.clone());
                    if let Some(last) = app.history.last_mut() {
                        if last.role == Role::Assistant {
                            let mut calls = last.tool_calls.clone().unwrap_or_default();
                            calls.push(tool_call);
                            last.tool_calls = Some(calls);
                        } else {
                            app.history.push(Message::assistant(None, None, Some(vec![tool_call])));
                        }
                    } else {
                        app.history.push(Message::assistant(None, None, Some(vec![tool_call])));
                    }
                }
                StreamChunk::ToolResult { name, content, tool_call_id } => {
                    app.active_tool = None;
                    app.history.push(Message::tool(tool_call_id, name, content));
                }
                StreamChunk::Done => {
                    app.is_generating = false;
                    app.active_tool = None;
                }
                StreamChunk::Error { content } => {
                    app.history.push(Message::system(format!("Error: {}", content)));
                    app.is_generating = false;
                    app.active_tool = None;
                }
                _ => {}
            }
        }
    }
}

async fn handle_model_search(app: &mut App, search: &str, force_reset: bool) {
    let mut sections: Vec<ModelMenuItem> = Vec::new();
    let config = app.orchestrator.config.lock().await.clone();

    let recent: Vec<DynamicModelInfo> = config.recent_models.iter()
        .filter(|m| m.name.to_lowercase().contains(search) || m.provider_id.to_lowercase().contains(search))
        .cloned()
        .collect();
    if !recent.is_empty() {
        sections.push(ModelMenuItem::Header("Recently Used".to_string()));
        for m in recent { sections.push(ModelMenuItem::Model(m)); }
    }

    let favorites: Vec<DynamicModelInfo> = config.favorites.iter()
        .filter(|m| m.name.to_lowercase().contains(search) || m.provider_id.to_lowercase().contains(search))
        .cloned()
        .collect();
    if !favorites.is_empty() {
        sections.push(ModelMenuItem::Header("Favorite Models".to_string()));
        for m in favorites { sections.push(ModelMenuItem::Model(m)); }
    }

    let mut by_provider: std::collections::HashMap<String, Vec<DynamicModelInfo>> = std::collections::HashMap::new();
    for m in &app.all_available_models {
        if m.name.to_lowercase().contains(search) || m.provider_id.to_lowercase().contains(search) {
            by_provider.entry(m.provider_id.clone()).or_default().push(m.clone());
        }
    }

    let mut provider_ids: Vec<String> = by_provider.keys().cloned().collect();
    provider_ids.sort();

    for p_id in provider_ids {
        if let Some(models) = by_provider.get(&p_id) {
            let p_name = PROVIDERS.iter().find(|p| p.id == p_id).map(|p| p.name).unwrap_or(&p_id);
            sections.push(ModelMenuItem::Header(p_name.to_string()));
            for m in models { sections.push(ModelMenuItem::Model(m.clone())); }
        }
    }

    app.filtered_models = sections;
    
    if force_reset {
        if !app.filtered_models.is_empty() {
            let mut first_model = None;
            for (i, item) in app.filtered_models.iter().enumerate() {
                if let ModelMenuItem::Model(_) = item { first_model = Some(i); break; }
            }
            app.menu_state.select(first_model);
        } else { app.menu_state.select(None); }
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
            app.all_available_models.clear();
            app.model_search_input = TextArea::default();
            app.model_search_input.set_cursor_line_style(Style::default());
            app.model_search_input.set_placeholder_text(" Search models...");
            app.model_search_input.set_placeholder_style(Style::default().fg(COLOR_SECONDARY));

            let config = app.orchestrator.config.lock().await.clone();
            for p_info in PROVIDERS {
                let env_key = format!("{}_API_KEY", p_info.id.to_uppercase().replace("-", "_"));
                let mut api_key = std::env::var(env_key).ok().or_else(|| config.api_keys.get(p_info.id).cloned());
                if api_key.is_none() && p_info.id.starts_with("cloudflare") {
                    api_key = std::env::var("CLOUDFLARE_API_KEY").ok();
                }
                if let Some(key) = api_key {
                    let provider = routecode_sdk::agents::resolve_provider(p_info.id, key);
                    match provider.list_models().await {
                        Ok(models) => {
                            for m_name in models {
                                app.all_available_models.push(DynamicModelInfo { name: m_name, provider_id: p_info.id.to_string() });
                            }
                        }
                        Err(e) => { log::error!("Failed to list models for {}: {}", p_info.id, e); }
                    }
                }
            }
            handle_model_search(app, "", true).await;
            if app.filtered_models.is_empty() {
                app.history.push(Message::system("No models found. Ensure providers are connected."));
            } else { app.show_model_menu = true; }
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
        "/clear" => {
            app.history.clear();
            app.screen = Screen::Welcome;
        }
        "/stop" => {
            if app.is_generating {
                if let Some(handle) = app.current_task.take() { handle.abort(); }
                app.is_generating = false;
                app.active_tool = None;
                app.history.push(Message::system("Generation cancelled."));
            }
        }
        "/help" => {
            app.history.push(Message::system("Available commands:\n  /model           - Select model\n  /provider        - Manage connections\n  /resume <name>   - Resume session\n  /sessions        - List sessions\n  /clear           - Clear history\n  /help            - Show help\n  /exit            - Use Esc to exit"));
        }
        "/provider" => { app.show_provider_menu = true; app.menu_state.select(Some(0)); }
        _ => { app.history.push(Message::system(format!("Unknown command: {}", command))); }
    }
}

fn ui(f: &mut Frame, app: &mut App, usage: &Usage) {
    let area = f.size();
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), area);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Content
        ])
        .split(area);

    let current_dir = std::env::current_dir()
        .map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string())
        .unwrap_or_else(|_| "workspace".to_string());
    
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(25), // " RouteCode v0.1.1 "
        ])
        .split(main_layout[0]);

    let version = env!("CARGO_PKG_VERSION");
    let header_title = format!(" RouteCode v{} ", version);

    f.render_widget(Paragraph::new(Span::styled(format!(" {} ", current_dir), Style::default().fg(COLOR_SECONDARY))), header_layout[0]);
    f.render_widget(Paragraph::new(Span::styled(header_title, Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Right), header_layout[1]);

    let input_area = match app.screen {
        Screen::Welcome => ui_welcome(f, app, main_layout[1]),
        Screen::Session => ui_session(f, app, usage, main_layout[1]),
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

pub fn clean_model_name(name: &str, provider_id: &str) -> String {
    if provider_id.starts_with("cloudflare") && name.starts_with("@cf/") {
        name.split('/').last().unwrap_or(name).to_string()
    } else if (provider_id == "openrouter" || provider_id == "nvidia") && name.contains('/') {
        name.split('/').last().unwrap_or(name).to_string()
    } else {
        name.to_string()
    }
}

fn draw_modal(f: &mut Frame, title: &str, width: u16, height: u16, footer: Vec<Span>) -> Rect {
    let area = f.size();
    let modal_area = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );

    f.render_widget(Clear, modal_area);
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), modal_area);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer Spacer
            Constraint::Length(1), // Footer
        ])
        .margin(1)
        .split(modal_area);

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(main_layout[0]);
    
    f.render_widget(Paragraph::new(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))), header_layout[0]);
    f.render_widget(Paragraph::new(Span::styled("esc", Style::default().fg(COLOR_SECONDARY))), header_layout[1]);
    f.render_widget(Paragraph::new(Line::from(footer)), main_layout[3]);

    main_layout[1]
}

fn render_api_key_dialog(f: &mut Frame, app: &mut App) {
    let provider_id = app.pending_provider_id.as_deref().unwrap_or("provider");
    let p_info = PROVIDERS.iter().find(|p| p.id == provider_id);
    let provider_name = p_info.map(|p| p.name).unwrap_or(provider_id);
    
    let title = format!("Connect {}", provider_name);
    let body_area = draw_modal(f, &title, 60, 10, vec![
        Span::styled("Press Enter to save", Style::default().add_modifier(Modifier::BOLD)),
    ]);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Prompt
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Input
        ])
        .split(body_area);

    let (prompt, placeholder) = match app.api_key_input_stage {
        ApiKeyInputStage::CloudflareAccountId => (format!("Enter Cloudflare Account ID:"), " Account ID..."),
        ApiKeyInputStage::CloudflareGatewayId => (format!("Enter Cloudflare Gateway ID:"), " Gateway ID..."),
        ApiKeyInputStage::CloudflareApiKey => (format!("Enter Cloudflare API Token:"), " API Token..."),
        _ => (format!("Enter API key for {}:", provider_name), " Paste your API key here..."),
    };

    f.render_widget(Paragraph::new(prompt), layout[0]);

    app.api_key_input.set_placeholder_text(placeholder);
    app.api_key_input.set_block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(COLOR_SECONDARY)));
    f.render_widget(app.api_key_input.widget(), layout[2]);

    let (row, col) = app.api_key_input.cursor();
    f.set_cursor(layout[2].x + 1 + col as u16, layout[2].y + 1 + row as u16);
}

fn render_provider_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (PROVIDERS.len() + 6).min(15) as u16;
    let body_area = draw_modal(f, "AI Providers", 60, height, vec![
        Span::styled("Space", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" toggle connection"),
    ]);

    let config = futures::executor::block_on(app.orchestrator.config.lock());
    let items: Vec<ListItem> = PROVIDERS.iter().map(|p| {
        let env_key = format!("{}_API_KEY", p.id.to_uppercase().replace("-", "_"));
        let is_connected = config.api_keys.contains_key(p.id) || std::env::var(env_key).is_ok();
        let status = if is_connected { Span::styled(" ✔ connected", Style::default().fg(COLOR_SUCCESS)) }
                    else { Span::styled(" ✖ disconnected", Style::default().fg(COLOR_SECONDARY)) };
        
        let total_width = body_area.width.saturating_sub(4);
        let left = p.name.to_string();
        let status_str = if is_connected { "✔ connected" } else { "✖ disconnected" };
        let padding = total_width.saturating_sub(left.len() as u16).saturating_sub(status_str.len() as u16);
        let spaces = " ".repeat(padding as usize);

        ListItem::new(Line::from(vec![
            Span::raw(format!(" {}", left)),
            Span::raw(spaces),
            status,
            Span::raw(" "),
        ]))
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");
    
    f.render_stateful_widget(list, body_area, &mut app.menu_state);
}

fn render_model_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (app.filtered_models.len() + 7).min(18) as u16;
    let body_area = draw_modal(f, "Select model", 70, height, vec![
        Span::styled("Connect provider ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("ctrl+a", Style::default().fg(COLOR_SECONDARY)),
        Span::raw("  "),
        Span::styled("Favorite ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("ctrl+f", Style::default().fg(COLOR_SECONDARY)),
    ]);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Search area
            Constraint::Min(0),    // List area
        ])
        .split(body_area);

    let search_text = app.model_search_input.lines()[0].clone();
    let search_para = if search_text.is_empty() {
        Paragraph::new(Span::styled("search models...", Style::default().fg(COLOR_SECONDARY)))
    } else {
        Paragraph::new(Span::styled(&search_text, Style::default().fg(COLOR_TEXT)))
    };
    f.render_widget(search_para, layout[0]);
    
    if app.show_model_menu && !app.is_inputting_api_key {
        let (row, col) = app.model_search_input.cursor();
        f.set_cursor(layout[0].x + col as u16, layout[0].y + row as u16);
    }

    let config = futures::executor::block_on(app.orchestrator.config.lock());
    let items: Vec<ListItem> = app.filtered_models.iter().map(|item| {
        match item {
            ModelMenuItem::Header(title) => {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {}", title), Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::DIM))
                ]))
            }
            ModelMenuItem::Model(m) => {
                let is_fav = config.favorites.iter().any(|fav| fav.name == m.name && fav.provider_id == m.provider_id);
                let fav_star = if is_fav { " ★" } else { "" };
                let display_name = clean_model_name(&m.name, &m.provider_id).replace(":free", " Free");
                let p_name = PROVIDERS.iter().find(|p| p.id == m.provider_id).map(|p| p.name).unwrap_or(&m.provider_id);
                let left = format!("{}{}", display_name, fav_star);
                let right = p_name.to_string();
                let total_width = layout[1].width.saturating_sub(4);
                let padding = total_width.saturating_sub(left.len() as u16).saturating_sub(right.len() as u16);
                let spaces = " ".repeat(padding as usize);
                ListItem::new(Line::from(vec![Span::raw(format!(" {}", left)), Span::raw(spaces), Span::raw(right), Span::raw(" ")]))
            }
        }
    }).collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");
    
    f.render_stateful_widget(list, layout[1], &mut app.menu_state);
}

fn ui_welcome(f: &mut Frame, app: &mut App, area: Rect) -> Rect {
    let logo_height = if area.height < 20 { 0 } else { 6 };
    let spacer_height = if area.height < 15 { 0 } else { area.height / 3 };
    let input_lines = app.input.lines().len() as u16;
    let input_height = (input_lines + 2).min(12);
    
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(spacer_height),
            Constraint::Length(logo_height),
            Constraint::Length(input_height), // Dynamic Input
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // Info & Tips
            Constraint::Min(0),
            Constraint::Length(1), // Footer
        ])
        .split(area);

    if logo_height > 0 {
        let logo_text = if area.width < 60 {
            vec![
                Line::from(Span::styled("  __          _   ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" |__) _|_ _ _/  _  _| _ ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" |  \\(_|(_(- \\__(_)(_|(/_ ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
            ]
        } else {
            vec![
                Line::from(Span::styled("  ____             _        ____          _      ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" |  _ \\ ___  _   _| |_ ___ / ___|___   __| | ___ ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" | |_) / _ \\| | | | __/ _ \\ |   / _ \\ / _` |/ _ \\", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" |  _ < (_) | |_| | ||  __/ |__| (_) | (_| |  __/", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(" |_| \\_\\___/ \\__,_|\\__\\___|\\____\\___/ \\__,_|\\___|", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))),
            ]
        };
        f.render_widget(Paragraph::new(logo_text).alignment(ratatui::layout::Alignment::Center), chunks[1]);
    }

    let input_width_percent = if area.width < 50 { 0.95 } else if area.width < 100 { 0.8 } else { 0.6 };
    let input_width = (area.width as f32 * input_width_percent) as u16;
    let input_area = Rect::new((area.width - input_width) / 2, chunks[2].y, input_width, input_height);
    f.render_widget(Block::default().style(Style::default().bg(COLOR_INPUT_BG)), input_area);
    let inner_input_area = Rect::new(input_area.x + 1, input_area.y + 1, input_area.width.saturating_sub(2), input_area.height.saturating_sub(2));
    app.input.set_block(Block::default().borders(Borders::NONE));
    f.render_widget(app.input.widget(), inner_input_area);
    if !app.is_generating { f.set_cursor(inner_input_area.x + app.input.cursor().1 as u16, inner_input_area.y + app.input.cursor().0 as u16); }

    let cleaned_model = clean_model_name(&app.current_model, &app.current_provider_id);
    let provider_info = vec![
        Span::styled("Model ", Style::default().fg(COLOR_SECONDARY)),
        Span::styled(cleaned_model, Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(" • Provider ", Style::default().fg(COLOR_SECONDARY)),
        Span::styled(&app.provider_name, Style::default().fg(COLOR_TEXT)),
    ];
    f.render_widget(Paragraph::new(Line::from(provider_info)).alignment(ratatui::layout::Alignment::Center), chunks[4]);

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];
    let tip_text = if app.is_generating { format!(" {} AI is working... ", frame) } else { "ctrl+p help | esc exit".to_string() };
    f.render_widget(Paragraph::new(tip_text).alignment(ratatui::layout::Alignment::Center).style(Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::DIM)), chunks[6]);
    input_area
}

fn ui_session(f: &mut Frame, app: &mut App, usage: &Usage, area: Rect) -> Rect {
    let input_height = (app.input.lines().len() as u16 + 2).min(12);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(input_height), Constraint::Length(1)])
        .split(area);
    
    let history = render_history(&app.history);
    
    // 1. Auto-scroll logic
    let mut total_height = 0;
    let available_width = chunks[0].width.saturating_sub(4).max(1); // Account for some margin
    for line in &history.lines {
        let line_width: usize = line.spans.iter().map(|s| s.content.len()).sum();
        let wrapped_height = (line_width as u16 / available_width) + 1;
        total_height += wrapped_height;
    }
    
    // Pin to bottom if generating
    if app.is_generating {
        app.history_scroll = total_height.saturating_sub(chunks[0].height);
    }

    f.render_widget(Paragraph::new(history).wrap(Wrap { trim: false }).scroll((app.history_scroll, 0)), chunks[0]);
    f.render_widget(Block::default().style(Style::default().bg(COLOR_INPUT_BG)), chunks[1]);
    let inner_input_area = Rect::new(chunks[1].x + 1, chunks[1].y + 1, chunks[1].width.saturating_sub(2), chunks[1].height.saturating_sub(2));
    app.input.set_block(Block::default().borders(Borders::NONE));
    f.render_widget(app.input.widget(), inner_input_area);
    if !app.is_generating { f.set_cursor(inner_input_area.x + app.input.cursor().1 as u16, inner_input_area.y + app.input.cursor().0 as u16); }

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];
    let generating_text = if app.is_generating {
        if let Some(tool) = &app.active_tool { format!(" {} [Running {}...] ", frame, tool) }
        else { format!(" {} [Thinking...] ", frame) }
    } else { "".to_string() };

    let cleaned_model = clean_model_name(&app.current_model, &app.current_provider_id);
    let status_bar = Line::from(vec![
        Span::styled(format!(" {} ", cleaned_model), Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" • Tokens: {} • Cost: ${:.4} ", usage.total_tokens, usage.total_cost), Style::default().fg(COLOR_SECONDARY)),
        Span::styled(generating_text, Style::default().fg(COLOR_SYSTEM)),
        Span::styled(" • ctrl+p help ", Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::DIM)),
    ]);
    f.render_widget(Paragraph::new(status_bar), chunks[2]);
    chunks[1]
}

fn render_history(history: &[Message]) -> Text<'_> {
    let mut lines = Vec::new();
    for m in history {
        match m.role {
            Role::User => {
                lines.push(Line::from(vec![Span::styled(" ● User", Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD))]));
                if let Some(content) = &m.content { for line in content.lines() { lines.push(Line::from(vec![Span::raw("   "), Span::raw(line)])); } }
            }
            Role::Assistant => {
                lines.push(Line::from(vec![Span::styled(" ● RouteCode", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))]));
                if let Some(thought) = &m.thought {
                    for line in thought.lines() {
                        lines.push(Line::from(vec![Span::styled("   │ ", Style::default().fg(COLOR_DIM)), Span::styled(line, Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::ITALIC))]));
                    }
                }
                if let Some(tool_calls) = &m.tool_calls {
                    for tc in tool_calls {
                        let args: serde_json::Value = serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));
                        let arg_preview = if let Some(path) = args["path"].as_str() {
                            format!("({})", path)
                        } else {
                            format!("({})", tc.function.name)
                        };

                        lines.push(Line::from(vec![
                            Span::styled("   🛠 ", Style::default().fg(COLOR_PRIMARY)),
                            Span::styled(format!("Using {} ", tc.function.name), Style::default().fg(COLOR_TEXT)),
                            Span::styled(arg_preview, Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::DIM)),
                        ]));
                    }
                }
                if let Some(content) = &m.content {
                    for line in content.lines() {
                        if line.trim().starts_with("```") { lines.push(Line::from(vec![Span::raw("   "), Span::styled(line, Style::default().fg(COLOR_PRIMARY))])); }
                        else { lines.push(Line::from(vec![Span::raw("   "), Span::raw(line)])); }
                    }
                }
            }
            Role::Tool => {
                lines.push(Line::from(vec![Span::styled(format!("   ✓ Tool ({})", m.name.as_deref().unwrap_or("result")), Style::default().fg(COLOR_SECONDARY))]));
                if let Some(content) = &m.content {
                    if let Ok(res) = serde_json::from_str::<routecode_sdk::core::ToolResult>(content) {
                        if let Some(diff) = res.diff {
                            for line in diff.lines() {
                                let style = if line.starts_with('+') {
                                    Style::default().fg(COLOR_SUCCESS)
                                } else if line.starts_with('-') {
                                    Style::default().fg(Color::Red)
                                } else {
                                    Style::default().fg(COLOR_DIM)
                                };
                                lines.push(Line::from(vec![Span::raw("     "), Span::styled(line.to_string(), style)]));
                            }
                        } else if let Some(out) = res.content {
                            let preview = if out.len() > 100 { format!("{}...", &out[..100]) } else { out };
                            lines.push(Line::from(vec![Span::styled(format!("     {}", preview), Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM))]));
                        } else if let Some(err) = res.error {
                            lines.push(Line::from(vec![Span::styled(format!("     Error: {}", err), Style::default().fg(Color::Red))]));
                        }
                    } else {
                        let preview = if content.len() > 100 { format!("{}...", &content[..100]) } else { content.clone() };
                        lines.push(Line::from(vec![Span::styled(format!("     {}", preview), Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM))]));
                    }
                }
            }
            Role::System => {
                lines.push(Line::from(vec![Span::styled(" ● System", Style::default().fg(COLOR_SYSTEM).add_modifier(Modifier::DIM))]));
                if let Some(content) = &m.content { lines.push(Line::from(vec![Span::styled(format!("   {}", content), Style::default().fg(COLOR_SYSTEM).add_modifier(Modifier::DIM))])); }
            }
        }
        lines.push(Line::from(""));
    }
    Text::from(lines)
}

fn render_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (app.filtered_commands.len() + 6).min(15) as u16;
    let body_area = draw_modal(f, "Commands", 60, height, vec![Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)), Span::raw(" select command")]);
    let items: Vec<ListItem> = app.filtered_commands.iter().map(|cmd| {
        let total_width = body_area.width.saturating_sub(4);
        let left = cmd.name.to_string();
        let right = cmd.description.to_string();
        let padding = total_width.saturating_sub(left.len() as u16).saturating_sub(right.len() as u16);
        let spaces = " ".repeat(padding as usize);
        ListItem::new(Line::from(vec![Span::raw(format!(" {}", left)), Span::raw(spaces), Span::styled(right, Style::default().fg(COLOR_SECONDARY)), Span::raw(" ")]))
    }).collect();
    let list = List::new(items).highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black)).highlight_symbol("");
    f.render_stateful_widget(list, body_area, &mut app.menu_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use routecode_sdk::tools::ToolRegistry;
    use routecode_sdk::core::Config;
    use tokio::sync::Mutex;
    use async_trait::async_trait;
    use routecode_sdk::agents::AIProvider;

    struct MockProvider;
    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str { "Mock" }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> { Ok(vec![]) }
        async fn ask(&self, _: Vec<Message>, _: &str, _: Option<Vec<serde_json::Value>>) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
            Err(anyhow::anyhow!("Not implemented"))
        }
    }

    #[test]
    fn test_app_initialization() {
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(MockProvider),
            Arc::new(ToolRegistry::new()),
            Arc::new(Mutex::new(Config::default())),
        ));
        let app = App::new(orchestrator, "Mock".to_string());
        assert_eq!(app.screen, Screen::Welcome);
        assert!(app.history.is_empty());
        assert_eq!(app.current_model, "gpt-4o");
    }

    #[test]
    fn test_update_filtered_commands() {
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(MockProvider),
            Arc::new(ToolRegistry::new()),
            Arc::new(Mutex::new(Config::default())),
        ));
        let mut app = App::new(orchestrator, "Mock".to_string());
        
        app.input.insert_str("/hel");
        app.update_filtered_commands();
        
        assert!(app.show_menu);
        assert_eq!(app.filtered_commands.len(), 1);
        assert_eq!(app.filtered_commands[0].name, "/help");
    }

    #[test]
    fn test_update_filtered_commands_no_match() {
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(MockProvider),
            Arc::new(ToolRegistry::new()),
            Arc::new(Mutex::new(Config::default())),
        ));
        let mut app = App::new(orchestrator, "Mock".to_string());
        
        app.input.insert_str("/nonexistent");
        app.update_filtered_commands();
        
        assert!(!app.show_menu);
        assert!(app.filtered_commands.is_empty());
    }
}
