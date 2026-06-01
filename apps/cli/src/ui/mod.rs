use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind, MouseButton};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, ListState, Paragraph},
    Frame, Terminal,
};
use routecode_sdk::agents::types::ConfirmationResponse;
use routecode_sdk::agents::StreamChunk;
use routecode_sdk::core::{AgentOrchestrator, Message, Role, DynamicModelInfo};
use routecode_sdk::utils::costs::Usage;
use std::io;
use std::sync::Arc;
use tokio::task::JoinSet;
use tui_textarea::TextArea;

pub mod components;
pub mod welcome;
pub mod session;
pub mod menus;
pub mod logic;

pub use components::*;
pub use logic::*;
pub use menus::*;
pub use session::*;
pub use welcome::*;

type ConfirmationSender = std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<routecode_sdk::agents::types::ConfirmationResponse>>>>;

pub struct ProviderInfo {
    pub id: &'static str,
    pub name: &'static str,
}

pub const PROVIDERS: &[ProviderInfo] = &[
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
    ProviderInfo { id: "vertex", name: "Google Vertex AI" },
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

pub const COMMANDS: &[Command] = &[
    Command { name: "/model", description: "Switch model" },
    Command { name: "/resume", description: "Resume a session" },
    Command { name: "/sessions", description: "List saved sessions" },
    Command { name: "/clear", description: "Clear history" },
    Command { name: "/thinking", description: "Set thinking level (low/max)" },
    Command { name: "/help", description: "Show help" },
    Command { name: "/stop", description: "Stop AI generation" },
    Command { name: "/provider", description: "Manage providers" },
    Command { name: "/settings", description: "Manage settings" },
    Command { name: "/exit", description: "Exit application" },
];

#[derive(Debug, PartialEq, Clone, Copy)]
#[allow(clippy::upper_case_acronyms)]
pub enum ApprovalMode {
    Normal,
    Plan,
    YOLO,
    Shell,
}

impl ApprovalMode {
    pub fn next(&self) -> Self {
        match self {
            ApprovalMode::Normal => ApprovalMode::Plan,
            ApprovalMode::Plan => ApprovalMode::YOLO,
            ApprovalMode::YOLO => ApprovalMode::Shell,
            ApprovalMode::Shell => ApprovalMode::Normal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ApprovalMode::Normal => "NORMAL",
            ApprovalMode::Plan => "PLAN",
            ApprovalMode::YOLO => "YOLO",
            ApprovalMode::Shell => "SHELL",
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Screen {
    Welcome,
    Session,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ApiKeyInputStage {
    None,
    ApiKey,
    VertexProject,
    VertexLocation,
    CloudflareAccountId,
    CloudflareGatewayId,
    CloudflareApiKey,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SettingsMenuItem {
    Header(String),
    Option { name: String, val: String, key: String },
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
    pub show_settings_menu: bool,
    pub menu_state: ListState,
    pub filtered_commands: Vec<&'static Command>,
    pub filtered_models: Vec<ModelMenuItem>,
    pub all_available_models: Vec<DynamicModelInfo>,
    pub history_scroll: u16,
    pub max_scroll: u16,
    pub auto_scroll: bool,
    pub is_generating: bool,
    pub tick_count: u64,
    pub active_tool: Option<String>,
    pub tasks: JoinSet<()>,
    pub prompt_history: Vec<String>,
    pub prompt_history_index: Option<usize>,
    pub api_key_input: TextArea<'static>,
    pub model_search_input: TextArea<'static>,
    pub is_inputting_api_key: bool,
    pub pending_provider_id: Option<String>,
    pub api_key_input_stage: ApiKeyInputStage,
    pub pending_account_id: Option<String>,
    pub pending_gateway_id: Option<String>,
    pub pending_clear: bool,
    pub pending_exit: bool,
    pub is_fetching_models: bool,
    pub collapse_thinking: bool,
    pub mouse_row: Option<u16>,
    pub mouse_col: Option<u16>,
    pub mouse_moved: bool,
    pub mouse_events_count: u64,
    pub logo_anim_frames: u16,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<StreamChunk>,
    pub tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>,
    pub settings_items: Vec<SettingsMenuItem>,
    pub last_click_up: Option<(std::time::Instant, u16, u16)>,
    pub mouse_down_start: Option<(std::time::Instant, u16, u16)>,
    pub temp_expand_thinking: bool,
    pub last_toggle_time: Option<std::time::Instant>,
    pub thinking_hover_rendered: bool,
    pub usage: Usage,
    pub cached_history_len: usize,
    pub cached_width: u16,
    pub cached_is_collapsed: bool,
    pub cached_thinking_hovered: bool,
    pub cached_total_height: usize,
    pub cached_text: Option<ratatui::text::Text<'static>>,
    pub pending_command_confirmation: Option<(String, String, ConfirmationSender)>,
    pub inputting_command_feedback: bool,
    pub show_user_msg_modal: Option<usize>,
    pub user_msg_modal_selected: usize,
    pub cached_hovered_msg_idx: Option<usize>,
    pub session_id: String,
    pub pending_update: Option<String>,
    pub pending_update_changelog: String,
    pub pending_update_published_at: String,
    pub update_modal_selected: usize,
    pub pending_update_install: bool,
    pub render_dirty: bool,
    pub last_cache_update: std::time::Instant,
    pub approval_mode: ApprovalMode,
    pub startup_input_buffer: Vec<String>,
    pub startup_ready: bool,
    pub hide_cwd: bool,
    pub hide_model_info: bool,
    pub hide_context_summary: bool,
}

impl App {
    pub fn new(orchestrator: Arc<AgentOrchestrator>, provider_name: String, default_model: String) -> Self {
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
            current_model: default_model,
            current_provider_id: provider_name.clone(),
            provider_name,
            show_menu: false,
            show_provider_menu: false,
            show_model_menu: false,
            show_settings_menu: false,
            menu_state: ListState::default(),
            filtered_commands: Vec::new(),
            filtered_models: Vec::new(),
            all_available_models: Vec::new(),
            settings_items: Vec::new(),
            history_scroll: 0,
            max_scroll: 0,
            auto_scroll: true,
            is_generating: false,
            tick_count: 0,
            active_tool: None,
            tasks: JoinSet::new(),
            prompt_history: Vec::new(),
            prompt_history_index: None,
            api_key_input,
            model_search_input,
            is_inputting_api_key: false,
            pending_provider_id: None,
            api_key_input_stage: ApiKeyInputStage::None,
            pending_account_id: None,
            pending_gateway_id: None,
            pending_clear: false,
            pending_exit: false,
            is_fetching_models: false,
            collapse_thinking: false,
            mouse_row: None,
            mouse_col: None,
            mouse_moved: false,
            mouse_events_count: 0,
            logo_anim_frames: 0,
            rx,
            tx,
            usage: Usage::default(),
            last_click_up: None,
            mouse_down_start: None,
            temp_expand_thinking: false,
            last_toggle_time: None,
            thinking_hover_rendered: false,
            cached_history_len: 0,
            cached_width: 0,
            cached_is_collapsed: false,
            cached_thinking_hovered: false,
            cached_total_height: 0,
            cached_text: None,
            pending_command_confirmation: None,
            inputting_command_feedback: false,
            show_user_msg_modal: None,
            user_msg_modal_selected: 0,
            cached_hovered_msg_idx: None,
            session_id: format!("session_{}", uuid::Uuid::new_v4()),
            pending_update: None,
            pending_update_changelog: String::new(),
            pending_update_published_at: String::new(),
            update_modal_selected: 1,
            pending_update_install: false,
            render_dirty: true,
            last_cache_update: std::time::Instant::now(),
            approval_mode: ApprovalMode::Normal,
            startup_input_buffer: Vec::new(),
            startup_ready: false,
            hide_cwd: false,
            hide_model_info: false,
            hide_context_summary: false,
        }
    }

    pub async fn populate_settings(&mut self) {
        let config = self.orchestrator.config.lock().await;
        self.settings_items = vec![
            SettingsMenuItem::Header("Appearance".to_string()),
            SettingsMenuItem::Option {
                name: "Logo Animation".to_string(),
                val: config.logo_animation.clone(),
                key: "logo_animation".to_string(),
            },
            SettingsMenuItem::Option {
                name: "Animation Theme".to_string(),
                val: config.logo_animation_color.clone(),
                key: "logo_animation_color".to_string(),
            },
            SettingsMenuItem::Header("Footer".to_string()),
            SettingsMenuItem::Option {
                name: "Show Model Info".to_string(),
                val: if self.hide_model_info { "hide" } else { "show" }.to_string(),
                key: "hide_model_info".to_string(),
            },
            SettingsMenuItem::Option {
                name: "Show Context Summary".to_string(),
                val: if self.hide_context_summary { "hide" } else { "show" }.to_string(),
                key: "hide_context_summary".to_string(),
            },
            SettingsMenuItem::Option {
                name: "Show Directory".to_string(),
                val: if self.hide_cwd { "hide" } else { "show" }.to_string(),
                key: "hide_cwd".to_string(),
            },
        ];
    }

    pub fn update_filtered_commands(&mut self) {
        let input_line = self.input.lines().first().map(|l| l.to_lowercase()).unwrap_or_default();
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

/// Compute whether the mouse is hovering over a thinking block, accounting for text wrapping.
/// Uses the same wrapping calculation as the auto-scroll logic in ui_session.
pub fn compute_thinking_hover(app: &App, size: ratatui::layout::Rect) -> bool {
    let mouse_row = match app.mouse_row {
        Some(r) => r,
        None => return false,
    };
    if app.screen != Screen::Session {
        return false;
    }
    let has_thinking = app.history.iter().any(|m| m.thought.is_some());
    if !has_thinking {
        return false;
    }

    // Compute layout: header=1 row, then history area, then input, then status bar
    let input_height = (app.input.lines().len() as u16 + 2).min(12);
    // area starts at row 1 (after header). History is area minus input and status.
    let area_height = size.height.saturating_sub(1); // main area below header
    let history_height = area_height.saturating_sub(input_height).saturating_sub(1);

    // Check mouse is in history area (row 1 to 1+history_height exclusive)
    if mouse_row < 1 || mouse_row > history_height {
        return false;
    }

    // The visual row within the history viewport (0-indexed from top of visible area)
    let viewport_row = mouse_row - 1;
    // The absolute visual row including scroll
    let target_visual_row = viewport_row as usize + app.history_scroll as usize;

    // Build the history text and compute wrapping to find which logical line the target row maps to
    let is_collapsed = app.collapse_thinking && !app.temp_expand_thinking;
    let history_text = render_history(&app.history, is_collapsed, app.thinking_hover_rendered, None, 0);
    let available_width = size.width.max(1) as usize;
    let calc_width = (available_width as f32 * 0.95).floor().max(1.0) as usize;

    let mut cumulative_visual_row: usize = 0;
    for line in &history_text.lines {
        let line_width: usize = line.spans.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref())).sum();
        let wrapped_height = if line_width == 0 { 1 } else {
            line_width.div_ceil(calc_width)
        };

        // Check if target_visual_row falls within this logical line's visual rows
        if target_visual_row >= cumulative_visual_row && target_visual_row < cumulative_visual_row + wrapped_height {
            // Found the line - check if it's a thinking line
            return line.spans.iter().any(|span| {
                span.content.contains('\u{2502}') || span.content.contains('\u{2503}') || span.content.contains("Thinking...")
            });
        }
        cumulative_visual_row += wrapped_height;
    }
    false
}

/// Compute which message is hovered by the mouse.
pub fn compute_message_hover(app: &App, size: ratatui::layout::Rect) -> Option<usize> {
    let mouse_row = app.mouse_row?;
    if app.screen != Screen::Session {
        return None;
    }

    let input_height = (app.input.lines().len() as u16 + 2).min(12);
    let area_height = size.height.saturating_sub(1);
    let history_height = area_height.saturating_sub(input_height).saturating_sub(1);

    if mouse_row < 1 || mouse_row > history_height {
        return None;
    }

    let viewport_row = mouse_row - 1;
    let target_visual_row = viewport_row as usize + app.history_scroll as usize;

    let is_collapsed = app.collapse_thinking && !app.temp_expand_thinking;
    let available_width = size.width.max(1) as usize;
    let calc_width = (available_width as f32 * 0.95).floor().max(1.0) as usize;

    let mut cumulative_visual_row: usize = 0;

    for (msg_idx, m) in app.history.iter().enumerate() {
        let msg_slice = std::slice::from_ref(m);
        let msg_text = session::render_history(msg_slice, is_collapsed, app.thinking_hover_rendered, None, msg_idx);

        let mut msg_height: usize = 0;
        for line in &msg_text.lines {
            let line_width: usize = line.spans.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref())).sum();
            let wrapped_height = if line_width == 0 { 1 } else {
                line_width.div_ceil(calc_width)
            };
            msg_height += wrapped_height;
        }

        if target_visual_row >= cumulative_visual_row && target_visual_row < cumulative_visual_row + msg_height {
            return Some(msg_idx);
        }
        cumulative_visual_row += msg_height;
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyEventResult {
    Continue,
    Exit,
}

async fn handle_key_event(
    app: &mut App,
    key: event::KeyEvent,
    is_burst: bool,
) -> io::Result<KeyEventResult> {
    if app.pending_update.is_some() {
        match key.code {
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                app.update_modal_selected = if app.update_modal_selected == 0 { 1 } else { 0 };
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.update_modal_selected = if app.update_modal_selected == 1 { 0 } else { 1 };
            }
            KeyCode::Enter => {
                if app.update_modal_selected == 1 {
                    app.pending_update_install = true;
                    return Ok(KeyEventResult::Exit);
                } else {
                    app.pending_update = None;
                }
            }
            KeyCode::Esc => {
                app.pending_update = None;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if let Some(msg_idx) = app.show_user_msg_modal {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.user_msg_modal_selected = if app.user_msg_modal_selected == 0 { 1 } else { 0 };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.user_msg_modal_selected = if app.user_msg_modal_selected == 1 { 0 } else { 1 };
            }
            KeyCode::Enter => {
                let text = app.history[msg_idx].content.as_ref().map(|s| s.to_string()).unwrap_or_default();
                if app.user_msg_modal_selected == 0 {
                    let text_clone = text.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = copy_to_clipboard(&text_clone) {
                            log::error!("Clipboard copy failed: {}", e);
                        }
                    });
                    app.history.push(Message::system("Message copied to clipboard!".to_string()));
                } else {
                    app.history.truncate(msg_idx);
                    app.input = tui_textarea::TextArea::from(text.lines().map(|s| s.to_string()));
                    app.input.move_cursor(tui_textarea::CursorMove::End);
                }
                app.show_user_msg_modal = None;
            }
            KeyCode::Esc => {
                app.show_user_msg_modal = None;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.pending_command_confirmation.is_some() {
        if app.inputting_command_feedback {
            match key.code {
                KeyCode::Esc => {
                    app.inputting_command_feedback = false;
                    app.input.delete_line_by_head();
                    while app.input.cursor() != (0, 0) {
                        app.input.move_cursor(tui_textarea::CursorMove::Head);
                        app.input.delete_line_by_head();
                    }
                    app.input.set_placeholder_text(" Ask anything... \"How do I use this?\"");
                }
                KeyCode::Enter => {
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let lines = app.input.lines().to_vec();
                        app.input.delete_line_by_head();
                        while app.input.cursor() != (0, 0) {
                            app.input.move_cursor(tui_textarea::CursorMove::Head);
                            app.input.delete_line_by_head();
                        }
                        app.input.set_placeholder_text(" Ask anything... \"How do I use this?\"");
                        
                        let msg = lines.join("\n").trim().to_string();
                        let feedback = if msg.is_empty() { "Command cancelled.".to_string() } else { msg };
                        
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(routecode_sdk::agents::types::ConfirmationResponse::Feedback(feedback));
                        }
                    }
                    app.inputting_command_feedback = false;
                }
                _ => {
                    app.input.input(key);
                }
            }
        } else {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(routecode_sdk::agents::types::ConfirmationResponse::AllowOnce);
                        }
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let mut config = routecode_sdk::utils::storage::load_session_config(&app.session_id).unwrap_or_default();
                    config.allow_all_commands = true;
                    let _ = routecode_sdk::utils::storage::save_session_config(&app.session_id, &config);
                    
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(routecode_sdk::agents::types::ConfirmationResponse::AllowSession);
                        }
                    }
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    let mut config = routecode_sdk::utils::storage::load_workspace_config().unwrap_or_default();
                    config.allow_all_outside_access = true;
                    let _ = routecode_sdk::utils::storage::save_workspace_config(&config);
                    
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(routecode_sdk::agents::types::ConfirmationResponse::AllowWorkspace);
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Esc => {
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(routecode_sdk::agents::types::ConfirmationResponse::Deny);
                        }
                    }
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.inputting_command_feedback = true;
                    app.input.set_placeholder_text(" Tell agent (e.g. 'don't run without backup')...");
                }
                _ => {}
            }
        }
        return Ok(KeyEventResult::Continue);
    }

    if app.pending_clear {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.history.clear();
                app.screen = Screen::Welcome;
                app.history_scroll = 0;
                app.pending_clear = false;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.pending_clear = false;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.pending_exit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.tasks.abort_all();
                app.is_generating = false;
                app.active_tool = None;
                if !app.history.is_empty() {
                    let session = routecode_sdk::utils::storage::Session {
                        messages: app.history.clone(),
                        model: app.current_model.clone(),
                        usage: app.orchestrator.usage.lock().await.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    let _ = routecode_sdk::utils::storage::save_session(&app.session_id, &session);
                }
                return Ok(KeyEventResult::Exit);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.pending_exit = false;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
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
                app.tasks.abort_all();
                app.is_generating = false;
                app.active_tool = None;
            }
        }
        KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.history.clear();
            app.screen = Screen::Welcome;
            app.history_scroll = 0;
        }
        KeyCode::Enter if key.modifiers.contains(event::KeyModifiers::SHIFT) || key.modifiers.contains(event::KeyModifiers::ALT) => {
            app.input.insert_newline();
        }
        KeyCode::Enter => {
            let mut should_send = !is_burst;
            if should_send {
                let lines = app.input.lines();
                if let Some(last_line) = lines.last() {
                    if last_line.ends_with('\\') {
                        app.input.delete_char();
                        app.input.insert_newline();
                        should_send = false;
                    }
                }
            }

            if !should_send {
                app.input.insert_newline();
            } else if app.show_menu {
                if let Some(selected) = app.menu_state.selected() {
                    if let Some(cmd) = app.filtered_commands.get(selected) {
                        let name = cmd.name.to_string();
                        app.show_menu = false;
                        app.input = TextArea::default();
                        handle_command(app, &name).await;
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
            } else if app.show_settings_menu {
                if let Some(selected) = app.menu_state.selected() {
                    if let Some(SettingsMenuItem::Option { key, val: _, .. }) = app.settings_items.get(selected) {
                        if key == "logo_animation" {
                            let next_val = {
                                let config = app.orchestrator.config.lock().await;
                                match config.logo_animation.as_str() {
                                    "always" => "hover",
                                    "hover" => "click",
                                    _ => "always",
                                }
                            };
                            {
                                let mut config = app.orchestrator.config.lock().await;
                                config.logo_animation = next_val.to_string();
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                            }
                            app.populate_settings().await;
                        } else if key == "logo_animation_color" {
                            let next_val = {
                                let config = app.orchestrator.config.lock().await;
                                match config.logo_animation_color.as_str() {
                                    "rainbow" => "neon",
                                    "neon" => "cyberpunk",
                                    "cyberpunk" => "sunset",
                                    "sunset" => "mono",
                                    _ => "rainbow",
                                }
                            };
                            {
                                let mut config = app.orchestrator.config.lock().await;
                                config.logo_animation_color = next_val.to_string();
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                            }
                            app.populate_settings().await;
                        } else if key == "hide_cwd" {
                            app.hide_cwd = !app.hide_cwd;
                            app.populate_settings().await;
                        } else if key == "hide_model_info" {
                            app.hide_model_info = !app.hide_model_info;
                            app.populate_settings().await;
                        } else if key == "hide_context_summary" {
                            app.hide_context_summary = !app.hide_context_summary;
                            app.populate_settings().await;
                        }
                    }
                }
            } else if app.show_model_menu {
                if let Some(selected) = app.menu_state.selected() {
                    if let Some(ModelMenuItem::Model(model_info)) = app.filtered_models.get(selected) {
                        let model_info = model_info.clone();
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
                            if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                            if app.provider_name.to_lowercase() != *provider_id {
                                let vertex_project = config.vertex_project.clone();
                                let vertex_location = config.vertex_location.clone();
                                drop(config);
                                let provider = if provider_id == "vertex" {
                                    routecode_sdk::agents::resolve_provider_with_config(provider_id, key, &vertex_project, &vertex_location)
                                } else {
                                    routecode_sdk::agents::resolve_provider(provider_id, key)
                                };
                                app.provider_name = provider.name().to_string();
                                app.current_provider_id = provider_id.clone();
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
                            if let Some(provider_id) = app.pending_provider_id.clone() {
                                if provider_id == "vertex" {
                                    app.api_key_input_stage = ApiKeyInputStage::VertexProject;
                                    app.api_key_input = TextArea::default();
                                    app.api_key_input.set_placeholder_text(" Your GCP project ID...");
                                } else {
                                    app.pending_provider_id.take();
                                    let mut config = app.orchestrator.config.lock().await;
                                    config.api_keys.insert(provider_id, input_value);
                                    if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                                    app.history.push(Message::system("API Key saved"));
                                    app.is_inputting_api_key = false;
                                    app.api_key_input_stage = ApiKeyInputStage::None;
                                }
                            } else {
                                app.is_inputting_api_key = false;
                                app.api_key_input_stage = ApiKeyInputStage::None;
                            }
                        }
                        ApiKeyInputStage::VertexProject => {
                            app.pending_account_id = Some(input_value);
                            app.api_key_input = TextArea::default();
                            app.api_key_input_stage = ApiKeyInputStage::VertexLocation;
                        }
                        ApiKeyInputStage::VertexLocation => {
                            if let Some(provider_id) = app.pending_provider_id.take() {
                                let project = app.pending_account_id.take().unwrap_or_default();
                                let location = input_value;
                                let api_key = app.api_key_input.lines().join("\n").trim().to_string();
                                let mut config = app.orchestrator.config.lock().await;
                                config.vertex_project = project;
                                config.vertex_location = location;
                                config.api_keys.insert(provider_id, api_key);
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                                app.history.push(Message::system("Vertex AI credentials saved"));
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
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
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
                        handle_command(app, &input_text).await;
                    } else if !app.startup_ready {
                        app.startup_input_buffer.push(input_text.clone());
                        app.history.push(Message::system(format!("Queued: {}", input_text)));
                        app.input = TextArea::default();
                    } else {
                        app.history.push(Message::user(input_text.clone()));
                        app.prompt_history.push(input_text.clone());
                        app.prompt_history.truncate(100);
                        app.prompt_history_index = None;
                        app.input = TextArea::default();
                        app.screen = Screen::Session;
                        app.is_generating = true;
                        app.auto_scroll = true;
                        let orchestrator = app.orchestrator.clone();
                        let mut history = app.history.clone();
                        let model = app.current_model.clone();
                        let tx = app.tx.clone();
                        app.tasks.spawn(async move {
                            if let Err(e) = orchestrator.run(&mut history, &model, Some(tx)).await {
                                log::error!("Orchestrator run failed: {}", e);
                            }
                        });
                    }
                    app.input = TextArea::default();
                }
            }
        }
        KeyCode::Esc => {
            if app.show_menu { app.show_menu = false; }
            else if app.show_provider_menu { app.show_provider_menu = false; }
            else if app.show_model_menu { app.show_model_menu = false; }
            else if app.show_settings_menu { app.show_settings_menu = false; }
            else if app.is_inputting_api_key {
                app.is_inputting_api_key = false;
                app.api_key_input_stage = ApiKeyInputStage::None;
                app.pending_account_id = None;
                app.pending_gateway_id = None;
            } else if app.is_generating {
                app.tasks.abort_all();
                app.is_generating = false;
                app.active_tool = None;
            } else {
                app.pending_exit = true;
            }
        }
        KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.auto_scroll = !app.auto_scroll;
            app.history.push(Message::system(format!("Auto-scroll {}", if app.auto_scroll { "enabled" } else { "disabled" })));
        }
        KeyCode::Char('o') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.collapse_thinking = !app.collapse_thinking;
        }
        KeyCode::End => {
            app.auto_scroll = true;
            app.history_scroll = app.max_scroll;
        }
        KeyCode::Up if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            let (row, _) = app.input.cursor();
            if row == 0 && app.input.lines().len() == 1 && app.input.lines()[0].is_empty() && !app.prompt_history.is_empty() {
                let idx = match app.prompt_history_index {
                    Some(i) => if i == 0 { 0 } else { i - 1 },
                    None => app.prompt_history.len() - 1,
                };
                app.prompt_history_index = Some(idx);
                let prev = app.prompt_history[idx].clone();
                app.input = TextArea::from(prev.lines().map(|s| s.to_string()));
                app.input.move_cursor(tui_textarea::CursorMove::End);
            }
        }
        KeyCode::Down if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
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
            }
        }
        KeyCode::Up => {
            if app.show_menu || app.show_provider_menu || app.show_model_menu || app.show_settings_menu {
                let items_len = if app.show_menu { app.filtered_commands.len() }
                               else if app.show_provider_menu { PROVIDERS.len() }
                               else if app.show_settings_menu { app.settings_items.len() }
                               else { app.filtered_models.len() };
                if items_len > 0 {
                    let selected = app.menu_state.selected().unwrap_or(0);
                    let mut new_selected = if selected == 0 { items_len - 1 } else { selected - 1 };
                    if app.show_model_menu {
                        while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(new_selected) {
                            new_selected = if new_selected == 0 { items_len - 1 } else { new_selected - 1 };
                            if new_selected == selected { break; }
                        }
                    } else if app.show_settings_menu {
                        while let Some(SettingsMenuItem::Header(_)) = app.settings_items.get(new_selected) {
                            new_selected = if new_selected == 0 { items_len - 1 } else { new_selected - 1 };
                            if new_selected == selected { break; }
                        }
                    }
                    app.menu_state.select(Some(new_selected));
                }
            } else if app.input.lines().len() == 1 && app.input.lines()[0].is_empty() || app.history_scroll > 0 || app.is_generating || key.modifiers.contains(event::KeyModifiers::SHIFT) {
                app.history_scroll = app.history_scroll.saturating_sub(15);
                app.auto_scroll = false;
            } else {
                app.input.input(Event::Key(key));
            }
        }
        KeyCode::Down => {
            if app.show_menu || app.show_provider_menu || app.show_model_menu || app.show_settings_menu {
                let items_len = if app.show_menu { app.filtered_commands.len() }
                               else if app.show_provider_menu { PROVIDERS.len() }
                               else if app.show_settings_menu { app.settings_items.len() }
                               else { app.filtered_models.len() };
                if items_len > 0 {
                    let selected = app.menu_state.selected().unwrap_or(0);
                    let mut new_selected = if selected >= items_len - 1 { 0 } else { selected + 1 };
                    if app.show_model_menu {
                        while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(new_selected) {
                            new_selected = if new_selected >= items_len - 1 { 0 } else { new_selected + 1 };
                            if new_selected == selected { break; }
                        }
                    } else if app.show_settings_menu {
                        while let Some(SettingsMenuItem::Header(_)) = app.settings_items.get(new_selected) {
                            new_selected = if new_selected >= items_len - 1 { 0 } else { new_selected + 1 };
                            if new_selected == selected { break; }
                        }
                    }
                    app.menu_state.select(Some(new_selected));
                }
            } else if app.input.lines().len() == 1 && app.input.lines()[0].is_empty() || app.history_scroll < app.max_scroll || app.is_generating || key.modifiers.contains(event::KeyModifiers::SHIFT) {
                app.history_scroll = app.history_scroll.saturating_add(15);
                if app.history_scroll >= app.max_scroll { app.auto_scroll = true; }
            } else {
                app.input.input(Event::Key(key));
            }
        }
        KeyCode::Right if app.show_model_menu => {
            let len = app.filtered_models.len();
            if len > 0 {
                let current = app.menu_state.selected().unwrap_or(0);
                let mut next_header_idx = None;
                for i in (current + 1)..len { if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) { next_header_idx = Some(i); break; } }
                if next_header_idx.is_none() { for i in 0..current { if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) { next_header_idx = Some(i); break; } } }
                if let Some(h_idx) = next_header_idx {
                    let mut target = (h_idx + 1) % len;
                    while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) { target = (target + 1) % len; if target == h_idx { break; } }
                    app.menu_state.select(Some(target));
                }
            }
        }
        KeyCode::Left if app.show_model_menu => {
            let len = app.filtered_models.len();
            if len > 0 {
                let current = app.menu_state.selected().unwrap_or(0);
                let mut headers = Vec::new();
                for (i, item) in app.filtered_models.iter().enumerate() { if let ModelMenuItem::Header(_) = item { headers.push(i); } }
                if !headers.is_empty() {
                    let current_header_idx_in_headers = headers.iter().enumerate().rev().find(|(_, &h_idx)| h_idx < current).map(|(i, _)| i);
                    let target_header_idx = match current_header_idx_in_headers { Some(i) => if i == 0 { *headers.last().unwrap() } else { headers[i - 1] }, None => *headers.last().unwrap() };
                    let mut target = (target_header_idx + 1) % len;
                    while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) { target = (target + 1) % len; if target == target_header_idx { break; } }
                    app.menu_state.select(Some(target));
                }
            }
        }
        KeyCode::Char('f') if key.modifiers.contains(event::KeyModifiers::CONTROL) && app.show_model_menu => {
            if let Some(selected) = app.menu_state.selected() {
                if let Some(ModelMenuItem::Model(model_info)) = app.filtered_models.get(selected) {
                    let model_info = model_info.clone();
                    let mut config = app.orchestrator.config.lock().await;
                    if config.favorites.iter().any(|m| m.name == model_info.name && m.provider_id == model_info.provider_id) { config.favorites.retain(|m| m.name != model_info.name || m.provider_id != model_info.provider_id); app.history.push(Message::system(format!("Removed {} from favorites", model_info.name))); }
                    else { config.favorites.push(model_info.clone()); app.history.push(Message::system(format!("Added {} to favorites", model_info.name))); }
                    if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                }
            }
        }
        KeyCode::BackTab => {
            app.approval_mode = app.approval_mode.next();
            let info = match app.approval_mode {
                ApprovalMode::YOLO => "YOLO — commands will auto-approve",
                ApprovalMode::Plan => "PLAN — tool calls will be denied (read-only review)",
                ApprovalMode::Shell => "SHELL — shell commands shown first, auto-approved",
                ApprovalMode::Normal => "Normal mode — confirm each tool call",
            };
            app.history.push(Message::system(format!("Mode: {}", info)));
        }
        _ => {
            let event = Event::Key(key);
            if app.is_inputting_api_key { app.api_key_input.input(event); }
            else if app.show_model_menu { if app.model_search_input.input(event) { let search = app.model_search_input.lines().first().map(|l| l.trim().to_lowercase()).unwrap_or_default(); handle_model_search(app, &search, true).await; } }
            else { app.input.input(event); app.update_filtered_commands(); }
        }
    }
    Ok(KeyEventResult::Continue)
}

async fn handle_mouse_event<B: ratatui::backend::Backend>(
    app: &mut App,
    mouse: event::MouseEvent,
    terminal: &mut Terminal<B>,
) -> io::Result<()> {
    app.mouse_events_count += 1;
    // Always store current mouse position for render-time hover detection
    app.mouse_row = Some(mouse.row);
    app.mouse_col = Some(mouse.column);
    match mouse.kind {
        MouseEventKind::Moved => {
            app.mouse_moved = true;
        }
         MouseEventKind::ScrollUp => {
            if app.show_menu || app.show_provider_menu || app.show_model_menu || app.show_settings_menu {
                let mut current = app.menu_state.selected().unwrap_or(0);
                current = current.saturating_sub(3);
                if app.show_model_menu {
                    while current > 0 && matches!(app.filtered_models.get(current), Some(crate::ui::ModelMenuItem::Header(_))) {
                        current -= 1;
                    }
                } else if app.show_settings_menu {
                    while current > 0 && matches!(app.settings_items.get(current), Some(crate::ui::SettingsMenuItem::Header(_))) {
                        current -= 1;
                    }
                }
                app.menu_state.select(Some(current));
            } else {
                app.history_scroll = app.history_scroll.saturating_sub(15); 
                app.auto_scroll = false;
            }
        }
        MouseEventKind::ScrollDown => {
            if app.show_menu || app.show_provider_menu || app.show_model_menu || app.show_settings_menu {
                let current = app.menu_state.selected().unwrap_or(0);
                let max = if app.show_menu { 
                    app.filtered_commands.len() 
                } else if app.show_provider_menu { 
                    crate::ui::PROVIDERS.len() 
                } else if app.show_settings_menu {
                    app.settings_items.len()
                } else { 
                    app.filtered_models.len() 
                };
                let mut next = current.saturating_add(3).min(max.saturating_sub(1));
                if app.show_model_menu {
                    while next < max - 1 && matches!(app.filtered_models.get(next), Some(crate::ui::ModelMenuItem::Header(_))) {
                        next += 1;
                    }
                } else if app.show_settings_menu {
                    while next < max - 1 && matches!(app.settings_items.get(next), Some(crate::ui::SettingsMenuItem::Header(_))) {
                        next += 1;
                    }
                }
                app.menu_state.select(Some(next));
            } else {
                app.history_scroll = app.history_scroll.saturating_add(15); 
                if app.history_scroll >= app.max_scroll { app.auto_scroll = true; }
            }
        }
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
            if let Some(msg_idx) = app.show_user_msg_modal {
                if let Ok(size) = terminal.size() {
                    let width = (size.width as f32 * 0.40) as u16;
                    let height = 8;
                    let modal_x = (size.width.saturating_sub(width)) / 2;
                    let modal_y = (size.height.saturating_sub(height)) / 2;
                    
                    let is_outside = mouse.column < modal_x || mouse.column >= modal_x + width || mouse.row < modal_y || mouse.row >= modal_y + height;
                    
                    if is_outside {
                        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                            app.show_user_msg_modal = None;
                        }
                    } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                        let click_row = mouse.row;
                        if click_row == modal_y + 2 {
                            app.user_msg_modal_selected = 0;
                            let text = app.history[msg_idx].content.as_ref().map(|s| s.to_string()).unwrap_or_default();
                            let text_clone = text.clone();
                            tokio::task::spawn_blocking(move || {
                                if let Err(e) = copy_to_clipboard(&text_clone) {
                                    log::error!("Clipboard copy failed: {}", e);
                                }
                            });
                            app.history.push(Message::system("Message copied to clipboard!".to_string()));
                            app.show_user_msg_modal = None;
                        } else if click_row == modal_y + 3 {
                            app.user_msg_modal_selected = 1;
                            let text = app.history[msg_idx].content.as_ref().map(|s| s.to_string()).unwrap_or_default();
                            app.history.truncate(msg_idx);
                            app.input = tui_textarea::TextArea::from(text.lines().map(|s| s.to_string()));
                            app.input.move_cursor(tui_textarea::CursorMove::End);
                            app.show_user_msg_modal = None;
                        }
                    }
                }
            } else if app.pending_update.is_some() {
                if let Ok(size) = terminal.size() {
                    let width = (size.width as f32 * 0.50) as u16;
                    let height = 8;
                    let modal_x = (size.width.saturating_sub(width)) / 2;
                    let modal_y = (size.height.saturating_sub(height)) / 2;
                    
                    let is_outside = mouse.column < modal_x || mouse.column >= modal_x + width || mouse.row < modal_y || mouse.row >= modal_y + height;
                    
                    if is_outside {
                        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                            app.pending_update = None;
                        }
                    } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                        && mouse.row == modal_y + height.saturating_sub(2) {
                            if mouse.column >= modal_x + width.saturating_sub(25) && mouse.column < modal_x + width.saturating_sub(15) {
                                app.pending_update = None;
                            } else if mouse.column >= modal_x + width.saturating_sub(15) && mouse.column < modal_x + width {
                                app.pending_update_install = true;
                            }
                        }
                }
            } else if app.show_menu || app.show_provider_menu || app.show_model_menu || app.show_settings_menu {
                if let Ok(size) = terminal.size() {
                    let (width, height) = if app.show_menu {
                        (60, (app.filtered_commands.len() + 6).min(15) as u16)
                    } else if app.show_provider_menu {
                        (60, (crate::ui::PROVIDERS.len() + 6).min(15) as u16)
                    } else if app.show_settings_menu {
                        (60, (app.settings_items.len() + 6).min(15) as u16)
                    } else {
                        (70, (app.filtered_models.len() + 7).min(18) as u16)
                    };
                    let modal_x = (size.width.saturating_sub(width)) / 2;
                    let modal_y = (size.height.saturating_sub(height)) / 2;
                    
                    let is_outside = mouse.column < modal_x || mouse.column >= modal_x + width || mouse.row < modal_y || mouse.row >= modal_y + height;
                    let is_esc = mouse.row <= modal_y + 2 && mouse.column >= modal_x + width.saturating_sub(10) && mouse.column <= modal_x + width;
                    let is_inside_list = mouse.row >= modal_y + 2 && mouse.row < modal_y + height - 1 && mouse.column > modal_x && mouse.column < modal_x + width - 1;
                    
                    if is_outside || is_esc {
                        app.show_menu = false;
                        app.show_provider_menu = false;
                        app.show_model_menu = false;
                        app.show_settings_menu = false;
                    } else if is_inside_list && matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                        && app.show_settings_menu {
                            let idx = (mouse.row - (modal_y + 2)) as usize + app.menu_state.offset();
                            if idx < app.settings_items.len() {
                                if let Some(SettingsMenuItem::Option { key, val: _, .. }) = app.settings_items.get(idx) {
                                    if key == "logo_animation" {
                                        let next_val = {
                                            let config = app.orchestrator.config.lock().await;
                                            match config.logo_animation.as_str() {
                                                "always" => "hover",
                                                "hover" => "click",
                                                _ => "always",
                                            }
                                        };
                                        {
                                            let mut config = app.orchestrator.config.lock().await;
                                            config.logo_animation = next_val.to_string();
                                            if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                                        }
                                        app.populate_settings().await;
                                    } else if key == "logo_animation_color" {
                                        let next_val = {
                                            let config = app.orchestrator.config.lock().await;
                                            match config.logo_animation_color.as_str() {
                                                "rainbow" => "neon",
                                                "neon" => "cyberpunk",
                                                "cyberpunk" => "sunset",
                                                "sunset" => "mono",
                                                _ => "rainbow",
                                            }
                                        };
                                        {
                                            let mut config = app.orchestrator.config.lock().await;
                                            config.logo_animation_color = next_val.to_string();
                                            if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
    log::error!("Failed to save config: {}", e);
}
                                        }
                                        app.populate_settings().await;
                                    } else if key == "hide_cwd" {
                                        app.hide_cwd = !app.hide_cwd;
                                        app.populate_settings().await;
                                    } else if key == "hide_model_info" {
                                        app.hide_model_info = !app.hide_model_info;
                                        app.populate_settings().await;
                                    } else if key == "hide_context_summary" {
                                        app.hide_context_summary = !app.hide_context_summary;
                                        app.populate_settings().await;
                                    }
                                }
                            }
                        }
                }
            } else if app.screen == Screen::Session {
                let has_thinking = app.history.iter().any(|m| m.thought.is_some());
                if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                    if let Ok(size) = terminal.size() {
                        if let Some(msg_idx) = compute_message_hover(app, size) {
                            if app.history[msg_idx].role == Role::User {
                                app.show_user_msg_modal = Some(msg_idx);
                                app.user_msg_modal_selected = 0;
                                return Ok(());
                            }
                        }
                    }
                }
                
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    let in_cooldown = app.last_toggle_time.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(400));
                    
                    if !in_cooldown && has_thinking {
                        let is_double_click = if let Some((last_time, col, row)) = app.last_click_up {
                            let col_diff = (col as i32 - mouse.column as i32).abs();
                            let row_diff = (row as i32 - mouse.row as i32).abs();
                            last_time.elapsed() < std::time::Duration::from_millis(600) && col_diff <= 4 && row_diff <= 3
                        } else {
                            false
                        };
                        
                        if is_double_click {
                            app.collapse_thinking = !app.collapse_thinking;
                            app.last_click_up = None;
                            app.mouse_down_start = None;
                            app.last_toggle_time = Some(std::time::Instant::now());
                        } else if let Ok(size) = terminal.size() {
                            // Compute hover FRESH with current mouse position
                            let hover = compute_thinking_hover(app, size);
                            if hover {
                                app.last_click_up = Some((std::time::Instant::now(), mouse.column, mouse.row));
                                app.mouse_down_start = Some((std::time::Instant::now(), mouse.column, mouse.row));
                            } else {
                                app.last_click_up = None;
                            }
                        }
                    }
                }
                if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                    app.mouse_down_start = None;
                    app.temp_expand_thinking = false;
                }
            } else if app.screen == Screen::Welcome && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                if let Ok(size) = terminal.size() {
                    let logo_height = if size.height < 20 { 0 } else { 6 };
                    let spacer_height = if size.height < 15 { 0 } else { size.height / 3 };
                    if logo_height > 0 && mouse.row >= spacer_height && mouse.row < spacer_height + logo_height {
                        app.logo_anim_frames = 20; // 2 seconds at 100ms tick
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

async fn handle_stream_chunks(app: &mut App) {
    let max_per_frame: u32 = 50;
    let mut processed: u32 = 0;
    while processed < max_per_frame {
        let chunk = match app.rx.try_recv() {
            Ok(c) => c,
            Err(_) => break,
        };
        processed += 1;
        // Throttle cache invalidation: mark dirty only once per burst
        if processed <= 1 {
            app.render_dirty = true;
        }
        match chunk {
            StreamChunk::Text { content } => {
                if let Some(last) = app.history.last_mut() {
                    if last.role == Role::Assistant {
                        let mut current = last.content.as_ref().map(|s| s.to_string()).unwrap_or_default();
                        current.push_str(&content);
                        last.content = Some(std::sync::Arc::from(current));
                    } else { app.history.push(Message::assistant(Some(std::sync::Arc::from(content)), None, None)); }
                } else { app.history.push(Message::assistant(Some(std::sync::Arc::from(content)), None, None)); }
            }
            StreamChunk::Thought { content } => {
                if let Some(last) = app.history.last_mut() {
                    if last.role == Role::Assistant {
                        let mut current = last.thought.as_ref().map(|s| s.to_string()).unwrap_or_default();
                        current.push_str(&content);
                        last.thought = Some(std::sync::Arc::from(current));
                    } else { app.history.push(Message::assistant(None, Some(std::sync::Arc::from(content)), None)); }
                } else { app.history.push(Message::assistant(None, Some(std::sync::Arc::from(content)), None)); }
            }
            StreamChunk::ToolCall { tool_call } => {
                app.active_tool = Some(tool_call.function.name.clone());
                if let Some(last) = app.history.last_mut() {
                    if last.role == Role::Assistant {
                        let mut calls = last.tool_calls.clone().unwrap_or_default();
                        if let Some(idx) = tool_call.index { if let Some(existing) = calls.iter_mut().find(|tc| tc.index == Some(idx)) { *existing = tool_call; } else { calls.push(tool_call); } }
                        else if !calls.iter().any(|tc| tc.id == tool_call.id && !tc.id.is_empty()) { calls.push(tool_call); }
                        last.tool_calls = Some(calls);
                    } else { app.history.push(Message::assistant(None, None, Some(vec![tool_call]))); }
                } else { app.history.push(Message::assistant(None, None, Some(vec![tool_call]))); }
            }
            StreamChunk::ToolResult { name, content, tool_call_id } => { app.active_tool = None; app.history.push(Message::tool(tool_call_id, name, content)); }
            StreamChunk::Done => {
                app.is_generating = false;
                app.active_tool = None;
                if !app.history.is_empty() {
                    let session = routecode_sdk::utils::storage::Session {
                        messages: app.history.clone(),
                        model: app.current_model.clone(),
                        usage: app.orchestrator.usage.lock().await.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    if let Err(e) = routecode_sdk::utils::storage::save_session(&app.session_id, &session) {
                        log::error!("Failed to auto-save session: {}", e);
                    }
                }
            }
            StreamChunk::Error { content } => {
                let mut display_error = content.clone();
                let json_part = if let Some(idx) = content.find('{') { &content[idx..] } else { &content };
                if json_part.len() > 10 && json_part.starts_with('{') {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_part) {
                        if let Some(msg) = val["error"]["message"].as_str() { display_error = msg.to_string(); }
                        else if let Some(error_obj) = val["error"].as_object() { if let Some(msg) = error_obj["message"].as_str() { display_error = msg.to_string(); } }
                        else if let Some(msg) = val["message"].as_str() { display_error = msg.to_string(); }
                        else if let Some(errors) = val["errors"].as_array() { if let Some(msg) = errors.first().and_then(|e| e["message"].as_str()) { display_error = msg.to_string(); } }
                    }
                }
                app.history.push(Message::system(format!("Error: {}", display_error)));
                app.is_generating = false;
                app.active_tool = None;
            }
            StreamChunk::Models { models } => { 
                app.all_available_models.extend(models); 
                let search = app.model_search_input.lines().first().map(|l| l.trim().to_lowercase()).unwrap_or_default();
                handle_model_search(app, &search, false).await;
            }
            StreamChunk::ModelsDone => {
                app.is_fetching_models = false;
                if !app.startup_ready {
                    app.startup_ready = true;
                    let buffered = app.startup_input_buffer.drain(..).collect::<Vec<_>>();
                    for msg in buffered {
                        app.history.push(Message::user(msg));
                        app.screen = Screen::Session;
                        app.prompt_history.truncate(100);
                        app.prompt_history_index = None;
                        app.input = TextArea::default();
                        app.is_generating = true;
                        app.auto_scroll = true;
                        let orchestrator = app.orchestrator.clone();
                        let mut history = app.history.clone();
                        let model = app.current_model.clone();
                        let tx = app.tx.clone();
                        app.tasks.spawn(async move {
                            if let Err(e) = orchestrator.run(&mut history, &model, Some(tx)).await {
                                log::error!("Orchestrator run failed: {}", e);
                            }
                        });
                    }
                }
            }
            StreamChunk::FinalHistory { history } => { app.history = history; }
            StreamChunk::RequestConfirmation { message, target, tx } => {
                match app.approval_mode {
                    ApprovalMode::YOLO | ApprovalMode::Shell => {
                        if let Some(sender) = tx {
                            let mut tx_opt = sender.lock().await;
                            if let Some(tx) = tx_opt.take() {
                                let _ = tx.send(ConfirmationResponse::AllowOnce);
                            }
                        }
                    }
                    ApprovalMode::Plan => {
                        if let Some(sender) = tx {
                            let mut tx_opt = sender.lock().await;
                            if let Some(tx) = tx_opt.take() {
                                let _ = tx.send(ConfirmationResponse::Deny);
                            }
                        }
                    }
                    ApprovalMode::Normal => {
                        if let Some(sender) = tx {
                            app.pending_command_confirmation = Some((message, target, sender));
                        } else {
                            log::error!("RequestConfirmation received without a response channel");
                        }
                    }
                }
            }
            StreamChunk::UpdateAvailable { version, changelog, published_at } => {
                app.pending_update = Some(version);
                app.pending_update_changelog = changelog;
                app.pending_update_published_at = published_at;
                app.update_modal_selected = 1;
            }
            _ => {}
        }
    }
}

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<bool> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(100);
    let render_rate = std::time::Duration::from_millis(16); // ~60 FPS for smooth rendering

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = render_rate;

        if event::poll(timeout)? {
            let mut events = Vec::new();
            while event::poll(std::time::Duration::from_millis(0))? {
                events.push(event::read()?);
            }

            let is_burst = events.len() > 1;

            for event in events {
                match event {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press {
                            match handle_key_event(&mut app, key, is_burst).await? {
                                KeyEventResult::Exit => return Ok(app.pending_update_install),
                                KeyEventResult::Continue => {}
                            }
                        }
                    }
                    Event::Paste(text) => { app.input.insert_str(&text); }
                    Event::Mouse(mouse) => {
                        handle_mouse_event(&mut app, mouse, terminal).await?;
                    }
                    Event::Resize(_, _) => {
                        app.cached_text = None;
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick_count += 1;
            app.logo_anim_frames = app.logo_anim_frames.saturating_sub(1);
            
            if app.screen == Screen::Session {
                if let Some((start_time, _, _)) = app.mouse_down_start {
                    if start_time.elapsed() >= std::time::Duration::from_millis(400)
                        && app.thinking_hover_rendered {
                            app.temp_expand_thinking = true;
                        }
                }
            }
            
            last_tick = std::time::Instant::now();
        }

        if app.pending_update_install {
            return Ok(true);
        }
        handle_stream_chunks(&mut app).await;
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.size();
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), area);
    let main_layout = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(0)]).split(area);
    let current_dir = std::env::current_dir().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).unwrap_or_else(|_| "workspace".to_string());

    let mode_label = app.approval_mode.label();
    let mode_style = match app.approval_mode {
        ApprovalMode::Normal => Style::default().fg(COLOR_SECONDARY),
        ApprovalMode::Plan => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ApprovalMode::YOLO => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ApprovalMode::Shell => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
    };
    let mode_indicator = format!("[{}]", mode_label);

    let mut header_left = Vec::new();
    if app.approval_mode != ApprovalMode::Normal {
        header_left.push(Span::styled(mode_indicator, mode_style));
        header_left.push(Span::raw(" "));
    }
    if !app.hide_cwd {
        header_left.push(Span::styled(format!("{} ", current_dir), Style::default().fg(COLOR_SECONDARY)));
    }

    let header_right_len = if app.hide_model_info { 0u16 } else { 25u16 };
    let header_layout = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Min(0), Constraint::Length(header_right_len)]).split(main_layout[0]);
    f.render_widget(Paragraph::new(Line::from(header_left)), header_layout[0]);

    if !app.hide_model_info {
        let version = env!("CARGO_PKG_VERSION");
        let header_title = format!(" RouteCode v{} ", version);
        f.render_widget(Paragraph::new(Span::styled(header_title, Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Right), header_layout[1]);
    }
    let input_area = match app.screen {
        Screen::Welcome => ui_welcome(f, app, main_layout[1]),
        Screen::Session => ui_session(f, app, main_layout[1]),
    };
    if app.show_menu { render_menu(f, app, input_area); }
    else if app.show_provider_menu { render_provider_menu(f, app, input_area); }
    else if app.show_model_menu { render_model_menu(f, app, input_area); }
    else if app.show_settings_menu { render_settings_menu(f, app, input_area); }
    else if app.is_inputting_api_key { render_api_key_dialog(f, app); }
    else if app.pending_clear { render_confirmation_dialog(f, "Are you sure you want to clear all history? (y/n)"); }
    else if app.pending_exit { render_confirmation_dialog(f, "Are you sure you want to exit RouteCode? (y/n)"); }
    else if app.pending_command_confirmation.is_some() { render_command_confirmation_dialog(f, app); }
    else if app.show_user_msg_modal.is_some() { render_user_msg_modal(f, app); }
    else if app.pending_update.is_some() { render_update_modal(f, app); }
    app.mouse_moved = false;
}

fn render_command_confirmation_dialog(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(10),
            Constraint::Percentage(30),
        ])
        .split(area);

    let popup_horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(popup_layout[1]);

    let inner_area = popup_horiz[1];

    let block = Block::default()
        .title(" Command Confirmation Required ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .style(Style::default().bg(COLOR_BG));

    let (message, target, _) = app.pending_command_confirmation.as_ref().unwrap();

    let mut lines = vec![
        ratatui::text::Line::from(vec![Span::styled(message, Style::default().fg(COLOR_TEXT))]),
        ratatui::text::Line::from(vec![Span::styled(format!("> {}", target), Style::default().fg(COLOR_SYSTEM).add_modifier(Modifier::BOLD))]),
        ratatui::text::Line::from(""),
    ];

    if app.inputting_command_feedback {
        lines.push(ratatui::text::Line::from(vec![Span::styled("Please type your feedback below and press Enter (Esc to cancel):", Style::default().fg(COLOR_SECONDARY))]));
    } else {
        lines.push(ratatui::text::Line::from(vec![
            Span::styled("[Y]", Style::default().fg(COLOR_SUCCESS).add_modifier(Modifier::BOLD)),
            Span::raw(" Allow once  "),
            Span::styled("[S]", Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::raw(" Allow for session  "),
            Span::styled("[W]", Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
            Span::raw(" Allow for Workspace  "),
            Span::styled("[F]", Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::BOLD)),
            Span::raw(" Tell Agent something else  "),
            Span::styled("[D] or [Esc]", Style::default().fg(ratatui::style::Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw(" Deny"),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, inner_area);
    f.render_widget(paragraph, inner_area);

    if app.inputting_command_feedback {
        let input_rect = ratatui::layout::Rect {
            x: inner_area.x + 2,
            y: inner_area.y + 5,
            width: inner_area.width.saturating_sub(4),
            height: 3,
        };
        let input_block = Block::default().borders(ratatui::widgets::Borders::ALL).border_style(Style::default().fg(COLOR_PRIMARY));
        app.input.set_block(input_block);
        f.render_widget(app.input.widget(), input_rect);
        f.set_cursor(input_rect.x + app.input.cursor().1 as u16 + 1, input_rect.y + app.input.cursor().0 as u16 + 1);
    }
}

fn render_confirmation_dialog(f: &mut Frame, message: &str) {
    let area = f.size();
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(5),
            Constraint::Percentage(40),
        ])
        .split(area);

    let popup_horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup_layout[1]);

    let block = Block::default()
        .title(" Confirmation ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY));

    let p = Paragraph::new(Span::styled(message, Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD)))
        .alignment(ratatui::layout::Alignment::Center)
        .block(block);

    f.render_widget(ratatui::widgets::Clear, popup_horiz[1]);
    f.render_widget(p, popup_horiz[1]);
}

fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let (prog, args): (&str, &[&str]) = if cfg!(target_os = "windows") {
        ("clip", &[])
    } else if cfg!(target_os = "macos") {
        ("pbcopy", &[])
    } else {
        ("xclip", &["-selection", "clipboard"])
    };
    let mut child = Command::new(prog).args(args).stdin(Stdio::piped()).spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let result = child.wait();
    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => Err(std::io::Error::other("clipboard command failed")),
        Err(e) => Err(e),
    }
}

fn render_user_msg_modal(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(8),
            Constraint::Percentage(40),
        ])
        .split(area);

    let popup_horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(popup_layout[1]);

    let inner_area = popup_horiz[1];

    let block = Block::default()
        .title(" Message Action ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .style(Style::default().bg(COLOR_BG));

    let options = ["Copy Message", "Rewind & Edit"];
    let mut lines = vec![
        ratatui::text::Line::from(vec![Span::styled(" Choose an action:", Style::default().fg(COLOR_SECONDARY))]),
        ratatui::text::Line::from(""),
    ];

    for (idx, opt) in options.iter().enumerate() {
        let is_selected = idx == app.user_msg_modal_selected;
        let prefix = if is_selected { " ➜ " } else { "   " };
        let style = if is_selected {
            Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT)
        };
        lines.push(ratatui::text::Line::from(vec![
            Span::styled(prefix, Style::default().fg(COLOR_PRIMARY)),
            Span::styled(opt.to_string(), style),
        ]));
    }
    
    lines.push(ratatui::text::Line::from(""));
    lines.push(ratatui::text::Line::from(vec![Span::styled(" Press Enter/Click to select, Esc to close", Style::default().fg(COLOR_DIM))]));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(ratatui::widgets::Clear, inner_area);
    f.render_widget(paragraph, inner_area);
}

fn render_update_modal(f: &mut Frame, app: &mut App) {
    let version = app.pending_update.as_ref().unwrap();
    let area = f.size();
    
    let modal_height = 12;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(((area.height as f32 * 0.3) as u16).min(area.height.saturating_sub(modal_height + 2))),
            Constraint::Length(modal_height),
            Constraint::Percentage(((area.height as f32 * 0.3) as u16).min(area.height.saturating_sub(modal_height + 2))),
        ])
        .split(area);

    let popup_horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(popup_layout[1]);

    let inner_area = popup_horiz[1];
    
    f.render_widget(ratatui::widgets::Clear, inner_area);
    
    let block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .title(ratatui::widgets::block::Title::from(Span::styled(" Update Available ", Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD))).alignment(ratatui::layout::Alignment::Left))
        .title(ratatui::widgets::block::Title::from(Span::styled(" esc ", Style::default().fg(COLOR_DIM))).alignment(ratatui::layout::Alignment::Right))
        .style(Style::default().bg(COLOR_BG));

    f.render_widget(block, inner_area);

    let content_area = ratatui::layout::Rect {
        x: inner_area.x + 2,
        y: inner_area.y + 1,
        width: inner_area.width.saturating_sub(4),
        height: inner_area.height.saturating_sub(4),
    };
    
    let mut lines = vec![
        ratatui::text::Line::from(vec![Span::styled(format!("Version {} is available (current: {})", version, env!("CARGO_PKG_VERSION")), Style::default().fg(COLOR_TEXT))]),
        ratatui::text::Line::from(""),
    ];

    if !app.pending_update_changelog.is_empty() {
        let changelog_lines: Vec<&str> = app.pending_update_changelog.lines().take(5).collect();
        for line in changelog_lines {
            let trimmed = if line.len() > 60 { format!("{}...", &line[..57]) } else { line.to_string() };
            lines.push(ratatui::text::Line::from(vec![Span::styled(trimmed.to_string(), Style::default().fg(COLOR_SECONDARY))]));
        }
    }
    
    let p = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(p, content_area);
    
    let button_area = ratatui::layout::Rect {
        x: inner_area.x + 2,
        y: inner_area.y + inner_area.height.saturating_sub(2),
        width: inner_area.width.saturating_sub(4),
        height: 1,
    };
    
    let skip_style = if app.update_modal_selected == 0 { Style::default().fg(ratatui::style::Color::Black).bg(COLOR_TEXT).add_modifier(Modifier::BOLD) } else { Style::default().fg(COLOR_DIM) };
    let confirm_style = if app.update_modal_selected == 1 { Style::default().fg(ratatui::style::Color::Black).bg(ratatui::style::Color::Rgb(255, 179, 138)).add_modifier(Modifier::BOLD) } else { Style::default().fg(ratatui::style::Color::Rgb(255, 179, 138)) };
    
    let buttons = ratatui::text::Line::from(vec![
        Span::styled("  Skip  ", skip_style),
        Span::raw("   "),
        Span::styled("  Confirm  ", confirm_style),
    ]);
    
    let p_buttons = Paragraph::new(buttons).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(p_buttons, button_area);
}

/// Try to lock config with short retry loop for use in sync render paths.
/// Rare contention from config saves resolves within microseconds; this avoids
/// silently showing "Loading..." or stale fallbacks.
pub fn try_lock_config(app: &App) -> Option<tokio::sync::MutexGuard<'_, routecode_sdk::core::Config>> {
    for _ in 0..10 {
        match app.orchestrator.config.try_lock() {
            Ok(guard) => return Some(guard),
            Err(_) => std::thread::sleep(std::time::Duration::from_micros(200)),
        }
    }
    None
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
        async fn ask(&self, _: Vec<Message>, _: &str, _: Option<Vec<serde_json::Value>>, _: Option<&str>) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
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
        let app = App::new(orchestrator, "Mock".to_string(), "gpt-4o".to_string());
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
        let mut app = App::new(orchestrator, "Mock".to_string(), "gpt-4o".to_string());
        
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
        let mut app = App::new(orchestrator, "Mock".to_string(), "gpt-4o".to_string());
        
        app.input.insert_str("/nonexistent");
        app.update_filtered_commands();
        
        assert!(!app.show_menu);
        assert!(app.filtered_commands.is_empty());
    }

    #[tokio::test]
    async fn test_user_msg_modal_rewind() {
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(MockProvider),
            Arc::new(ToolRegistry::new()),
            Arc::new(Mutex::new(Config::default())),
        ));
        let mut app = App::new(orchestrator, "Mock".to_string(), "gpt-4o".to_string());
        
        app.history.push(Message::user("First message".to_string()));
        app.history.push(Message::assistant(Some("Assistant reply".into()), None, None));
        app.history.push(Message::user("Second message".to_string()));
        
        app.show_user_msg_modal = Some(2);
        app.user_msg_modal_selected = 1;
        
        let enter_key = event::KeyEvent::new(event::KeyCode::Enter, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();
        
        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.show_user_msg_modal, None);
        assert_eq!(app.history.len(), 2);
        assert_eq!(app.history[0].role, Role::User);
        assert_eq!(app.history[1].role, Role::Assistant);
        assert_eq!(app.input.lines()[0], "Second message");
    }

    #[tokio::test]
    async fn test_update_system_modal() {
        let orchestrator = Arc::new(AgentOrchestrator::new(
            Arc::new(MockProvider),
            Arc::new(ToolRegistry::new()),
            Arc::new(Mutex::new(Config::default())),
        ));
        let mut app = App::new(orchestrator, "Mock".to_string(), "gpt-4o".to_string());
        
        app.pending_update = Some("v1.15.4".to_string());
        app.update_modal_selected = 1;
        
        let left_key = event::KeyEvent::new(event::KeyCode::Left, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, left_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.update_modal_selected, 0);
        
        let enter_key = event::KeyEvent::new(event::KeyCode::Enter, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.pending_update, None);
        assert!(!app.pending_update_install);
        
        app.pending_update = Some("v1.15.4".to_string());
        app.update_modal_selected = 1;
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Exit);
        assert!(app.pending_update_install);
    }
}
