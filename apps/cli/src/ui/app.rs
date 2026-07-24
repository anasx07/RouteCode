use ratatui::{layout::Rect, style::Style, widgets::ListState};
use routecode_sdk::agents::StreamChunk;
use routecode_sdk::core::{AgentOrchestrator, DynamicModelInfo, Message};
use routecode_sdk::utils::costs::Usage;
use std::sync::Arc;
use tokio::task::JoinSet;
use tui_textarea::TextArea;

use super::types::{
    ActiveModal, ApiKeyInputStage, ApprovalMode, Command, QirStatus, Screen, SettingsMenuItem,
    COMMANDS,
};

pub struct MenuState {
    pub list_state: ListState,
    pub filtered_commands: Vec<&'static Command>,
    pub settings_items: Vec<SettingsMenuItem>,
}

pub struct ApiKeyInputState {
    pub input: TextArea<'static>,
    pub stage: ApiKeyInputStage,
    pub pending_provider_id: Option<String>,
    pub pending_account_id: Option<String>,
    pub pending_gateway_id: Option<String>,
}

pub struct UpdateState {
    pub pending_version: Option<String>,
    pub changelog: String,
    pub published_at: String,
    pub selected: usize,
    pub install: bool,
}

pub struct ModelSearchState {
    pub search_input: TextArea<'static>,
    pub filtered_models: Vec<super::types::ModelMenuItem>,
    pub all_available_models: Vec<DynamicModelInfo>,
    pub is_fetching: bool,
}

pub struct PlanApprovalState {
    pub pending: Option<super::types::PendingPlan>,
    pub selected: usize,
    pub inputting_feedback: bool,
}

pub struct HookTrustState {
    pub pending: Option<super::types::PendingHookTrust>,
    pub selected: usize,
}

pub struct CommandConfirmationState {
    pub pending: Option<(String, String, super::types::ConfirmationSender)>,
    pub inputting_feedback: bool,
}

pub struct UserMessageState {
    pub msg_idx: Option<usize>,
    pub selected: usize,
}

pub struct ProviderState {
    pub current_model: String,
    pub current_provider_id: String,
    pub provider_name: String,
}

pub struct SessionState {
    pub id: String,
    pub history: Vec<Message>,
    pub history_scroll: u16,
    pub max_scroll: u16,
    pub auto_scroll: bool,
    pub is_generating: bool,
    pub collapse_thinking: bool,
    pub active_tool: Option<String>,
    pub prompt_history: Vec<String>,
    pub prompt_history_index: Option<usize>,
    pub usage: Usage,
    pub temp_expand_thinking: bool,
    pub last_toggle_time: Option<std::time::Instant>,
    pub thinking_hover_rendered: bool,
    pub approval_mode: ApprovalMode,
    pub startup_input_buffer: Vec<String>,
    pub startup_ready: bool,
    pub qir_retry_status: Option<QirStatus>,
}

pub struct MouseState {
    pub row: Option<u16>,
    pub col: Option<u16>,
    pub moved: bool,
    pub events_count: u64,
    pub last_click_up: Option<(std::time::Instant, u16, u16)>,
    pub down_start: Option<(std::time::Instant, u16, u16)>,
}

pub struct RenderCache {
    pub history_len: usize,
    pub width: u16,
    pub is_collapsed: bool,
    pub thinking_hovered: bool,
    pub total_height: usize,
    pub text: Option<ratatui::text::Text<'static>>,
    pub layout: Vec<(usize, bool)>,
    pub hovered_msg_idx: Option<usize>,
    pub last_update: std::time::Instant,
}

pub struct UiSettings {
    pub hide_cwd: bool,
    pub hide_model_info: bool,
    pub hide_context_summary: bool,
}

pub struct App {
    pub screen: Screen,
    pub active_modal: ActiveModal,
    pub input: TextArea<'static>,
    pub orchestrator: Arc<AgentOrchestrator>,
    pub provider: ProviderState,
    pub session: SessionState,
    pub mouse: MouseState,
    pub cache: RenderCache,
    pub ui_settings: UiSettings,
    pub tick_count: u64,
    pub logo_anim_frames: u16,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<StreamChunk>,
    pub tx: tokio::sync::mpsc::UnboundedSender<StreamChunk>,
    pub tasks: JoinSet<()>,
    pub render_dirty: bool,

    // Modal/dialog states
    pub menu: MenuState,
    pub model_search: ModelSearchState,
    pub api_key: ApiKeyInputState,
    pub update: UpdateState,
    pub plan_approval: PlanApprovalState,
    pub hook_trust: HookTrustState,
    pub cmd_confirmation: CommandConfirmationState,
    pub user_msg: UserMessageState,
}

impl App {
    pub fn new(
        orchestrator: Arc<AgentOrchestrator>,
        provider_name: String,
        default_model: String,
    ) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(Style::default());
        input.set_placeholder_style(Style::default().fg(super::components::COLOR_SECONDARY));
        input.set_placeholder_text(" Ask anything... \"How do I use this?\"");

        let mut api_key_input = TextArea::default();
        api_key_input.set_cursor_line_style(Style::default());
        api_key_input.set_placeholder_text(" Paste your API key here...");

        let mut model_search_input = TextArea::default();
        model_search_input.set_cursor_line_style(Style::default());
        model_search_input.set_placeholder_text(" Search models...");
        model_search_input
            .set_placeholder_style(Style::default().fg(super::components::COLOR_SECONDARY));

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            screen: Screen::Welcome,
            active_modal: ActiveModal::None,
            input,
            orchestrator,
            provider: ProviderState {
                current_model: default_model,
                current_provider_id: provider_name.clone(),
                provider_name,
            },
            menu: MenuState {
                list_state: ListState::default(),
                filtered_commands: Vec::new(),
                settings_items: Vec::new(),
            },
            model_search: ModelSearchState {
                search_input: model_search_input,
                filtered_models: Vec::new(),
                all_available_models: Vec::new(),
                is_fetching: false,
            },
            api_key: ApiKeyInputState {
                input: api_key_input,
                stage: ApiKeyInputStage::None,
                pending_provider_id: None,
                pending_account_id: None,
                pending_gateway_id: None,
            },
            update: UpdateState {
                pending_version: None,
                changelog: String::new(),
                published_at: String::new(),
                selected: 1,
                install: false,
            },
            plan_approval: PlanApprovalState {
                pending: None,
                selected: 0,
                inputting_feedback: false,
            },
            hook_trust: HookTrustState {
                pending: None,
                selected: 0,
            },
            cmd_confirmation: CommandConfirmationState {
                pending: None,
                inputting_feedback: false,
            },
            user_msg: UserMessageState {
                msg_idx: None,
                selected: 0,
            },
            session: SessionState {
                id: format!("session_{}", uuid::Uuid::new_v4()),
                history: Vec::new(),
                history_scroll: 0,
                max_scroll: 0,
                auto_scroll: true,
                is_generating: false,
                collapse_thinking: false,
                active_tool: None,
                prompt_history: Vec::new(),
                prompt_history_index: None,
                usage: Usage::default(),
                temp_expand_thinking: false,
                last_toggle_time: None,
                thinking_hover_rendered: false,
                approval_mode: ApprovalMode::Normal,
                startup_input_buffer: Vec::new(),
                startup_ready: false,
                qir_retry_status: None,
            },
            mouse: MouseState {
                row: None,
                col: None,
                moved: false,
                events_count: 0,
                last_click_up: None,
                down_start: None,
            },
            cache: RenderCache {
                history_len: 0,
                width: 0,
                is_collapsed: false,
                thinking_hovered: false,
                total_height: 0,
                text: None,
                layout: Vec::new(),
                hovered_msg_idx: None,
                last_update: std::time::Instant::now(),
            },
            ui_settings: UiSettings {
                hide_cwd: false,
                hide_model_info: false,
                hide_context_summary: false,
            },
            tick_count: 0,
            logo_anim_frames: 0,
            rx,
            tx,
            tasks: JoinSet::new(),
            render_dirty: true,
        }
    }

    pub async fn populate_settings(&mut self) {
        let config = self.orchestrator.config.lock().await;
        self.menu.settings_items = vec![
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
                val: if self.ui_settings.hide_model_info { "hide" } else { "show" }.to_string(),
                key: "hide_model_info".to_string(),
            },
            SettingsMenuItem::Option {
                name: "Show Context Summary".to_string(),
                val: if self.ui_settings.hide_context_summary {
                    "hide"
                } else {
                    "show"
                }
                .to_string(),
                key: "hide_context_summary".to_string(),
            },
            SettingsMenuItem::Option {
                name: "Show Directory".to_string(),
                val: if self.ui_settings.hide_cwd { "hide" } else { "show" }.to_string(),
                key: "hide_cwd".to_string(),
            },
            SettingsMenuItem::Header("Advanced".to_string()),
            SettingsMenuItem::Option {
                name: "Enable Sub-Agents".to_string(),
                val: if config.sub_agents_enabled {
                    "on"
                } else {
                    "off"
                }
                .to_string(),
                key: "sub_agents_enabled".to_string(),
            },
        ];
    }

    pub fn update_filtered_commands(&mut self) {
        let input_line = self
            .input
            .lines()
            .first()
            .map(|l| l.to_lowercase())
            .unwrap_or_default();
        if input_line.starts_with('/') {
            self.menu.filtered_commands = COMMANDS
                .iter()
                .filter(|c| c.name.to_lowercase().starts_with(&input_line))
                .collect();
            let show_menu = !self.menu.filtered_commands.is_empty();
            if show_menu {
                self.active_modal = ActiveModal::CommandMenu;
                self.menu.list_state.select(Some(0));
            } else if self.active_modal == ActiveModal::CommandMenu {
                self.active_modal = ActiveModal::None;
            }
        } else if self.active_modal == ActiveModal::CommandMenu {
            self.active_modal = ActiveModal::None;
        }
    }
}

/// Toggle a settings-menu item. Returns `true` if `key` matched a known
/// setting, `false` otherwise. Centralized so the keyboard and mouse
/// handlers can't drift apart (the original copy-paste had `sub_agents_enabled`
/// missing from the mouse path and `hide_context_summary` missing from the
/// keyboard path).
pub(crate) async fn apply_settings_toggle(app: &mut App, key: &str) -> bool {
    match key {
        "logo_animation" => {
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
        }
        "logo_animation_color" => {
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
        }
        "hide_cwd" => app.ui_settings.hide_cwd = !app.ui_settings.hide_cwd,
        "hide_model_info" => app.ui_settings.hide_model_info = !app.ui_settings.hide_model_info,
        "hide_context_summary" => app.ui_settings.hide_context_summary = !app.ui_settings.hide_context_summary,
        "sub_agents_enabled" => {
            let mut config = app.orchestrator.config.lock().await;
            config.sub_agents_enabled = !config.sub_agents_enabled;
            if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
                log::error!("Failed to save config: {}", e);
            }
        }
        _ => return false,
    }
    app.populate_settings().await;
    true
}

/// Compute whether the mouse is hovering over a thinking block, accounting for text wrapping.
/// Uses the same wrapping calculation as the auto-scroll logic in ui_session.
pub fn compute_thinking_hover(app: &App, size: Rect) -> bool {
    let mouse_row = match app.mouse.row {
        Some(r) => r,
        None => return false,
    };
    if app.screen != Screen::Session {
        return false;
    }
    let has_thinking = app.session.history.iter().any(|m| m.thought.is_some());
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
    let target_visual_row = viewport_row as usize + app.session.history_scroll as usize;

    if let Some(&(_, is_thinking)) = app.cache.layout.get(target_visual_row) {
        return is_thinking;
    }
    false
}

/// Compute which message is hovered by the mouse.
pub fn compute_message_hover(app: &App, size: Rect) -> Option<usize> {
    let mouse_row = app.mouse.row?;
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
    let target_visual_row = viewport_row as usize + app.session.history_scroll as usize;

    if let Some(&(msg_idx, _)) = app.cache.layout.get(target_visual_row) {
        return Some(msg_idx);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use routecode_sdk::agents::AIProvider;
    use routecode_sdk::core::Config;
    use routecode_sdk::tools::ToolRegistry;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockProvider;
    #[async_trait]
    impl AIProvider for MockProvider {
        fn name(&self) -> &str {
            "Mock"
        }
        async fn list_models(&self) -> Result<Vec<String>, anyhow::Error> {
            Ok(vec![])
        }
        async fn ask(
            &self,
            _: Arc<Vec<Message>>,
            _: &str,
            _: Arc<Option<Vec<serde_json::Value>>>,
            _: Option<&str>,
        ) -> Result<routecode_sdk::agents::traits::StreamResponse, anyhow::Error> {
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
        assert_eq!(app.active_modal, ActiveModal::None);
        assert!(app.session.history.is_empty());
        assert_eq!(app.provider.current_model, "gpt-4o");
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

        assert_eq!(app.active_modal, ActiveModal::CommandMenu);
        assert_eq!(app.menu.filtered_commands.len(), 1);
        assert_eq!(app.menu.filtered_commands[0].name, "/help");
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

        assert_eq!(app.active_modal, ActiveModal::None);
        assert!(app.menu.filtered_commands.is_empty());
    }
}
