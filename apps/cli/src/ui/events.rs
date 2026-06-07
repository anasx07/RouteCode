use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use ratatui::Terminal;
use routecode_sdk::agents::types::ConfirmationResponse;
use routecode_sdk::core::{Message, Role};
use std::io;
use tui_textarea::TextArea;

use super::app::{apply_settings_toggle, compute_message_hover, compute_thinking_hover, App};
use super::logic::{handle_command, handle_model_search};
use super::render::copy_to_clipboard;
use super::types::{
    ApiKeyInputStage, ApprovalMode, ModelMenuItem, Screen, SettingsMenuItem, PROVIDERS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyEventResult {
    Continue,
    Exit,
}

pub(crate) async fn handle_key_event(
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
                app.user_msg_modal_selected = if app.user_msg_modal_selected == 0 {
                    1
                } else {
                    0
                };
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.user_msg_modal_selected = if app.user_msg_modal_selected == 1 {
                    0
                } else {
                    1
                };
            }
            KeyCode::Enter => {
                let text = app.history[msg_idx]
                    .content
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if app.user_msg_modal_selected == 0 {
                    let text_clone = text.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = copy_to_clipboard(&text_clone) {
                            log::error!("Clipboard copy failed: {}", e);
                        }
                    });
                    app.history
                        .push(Message::system("Message copied to clipboard!".to_string()));
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
                    app.input
                        .set_placeholder_text(" Ask anything... \"How do I use this?\"");
                }
                KeyCode::Enter => {
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let lines = app.input.lines().to_vec();
                        app.input.delete_line_by_head();
                        while app.input.cursor() != (0, 0) {
                            app.input.move_cursor(tui_textarea::CursorMove::Head);
                            app.input.delete_line_by_head();
                        }
                        app.input
                            .set_placeholder_text(" Ask anything... \"How do I use this?\"");

                        let msg = lines.join("\n").trim().to_string();
                        let feedback = if msg.is_empty() {
                            "Command cancelled.".to_string()
                        } else {
                            msg
                        };

                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::Feedback(feedback));
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
                            let _ = tx.send(ConfirmationResponse::AllowOnce);
                        }
                    }
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let mut config =
                        routecode_sdk::utils::storage::load_session_config(&app.session_id)
                            .unwrap_or_default();
                    config.allow_all_commands = true;
                    let _ = routecode_sdk::utils::storage::save_session_config(
                        &app.session_id,
                        &config,
                    );

                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::AllowSession);
                        }
                    }
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    let mut config =
                        routecode_sdk::utils::storage::load_workspace_config().unwrap_or_default();
                    config.allow_all_outside_access = true;
                    let _ = routecode_sdk::utils::storage::save_workspace_config(&config);

                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::AllowWorkspace);
                        }
                    }
                }
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Esc => {
                    if let Some((_, _, tx_mutex)) = app.pending_command_confirmation.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::Deny);
                        }
                    }
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.inputting_command_feedback = true;
                    app.input
                        .set_placeholder_text(" Tell agent (e.g. 'don't run without backup')...");
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
            if app.show_model_menu {
                app.show_model_menu = false;
            }
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
        KeyCode::Enter
            if key.modifiers.contains(event::KeyModifiers::SHIFT)
                || key.modifiers.contains(event::KeyModifiers::ALT) =>
        {
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
                    if let Some(SettingsMenuItem::Option { key, val: _, .. }) =
                        app.settings_items.get(selected)
                    {
                        let key = key.clone();
                        apply_settings_toggle(app, &key).await;
                    }
                }
            } else if app.show_model_menu {
                if let Some(selected) = app.menu_state.selected() {
                    if let Some(ModelMenuItem::Model(model_info)) =
                        app.filtered_models.get(selected)
                    {
                        let model_info = model_info.clone();
                        let provider_id = &model_info.provider_id;
                        let model_name = &model_info.name;
                        let mut config = app.orchestrator.config.lock().await;
                        let env_key =
                            format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                        let api_key = std::env::var(env_key)
                            .ok()
                            .or_else(|| config.api_keys.get(provider_id).cloned());
                        if let Some(key) = api_key {
                            config.model = model_name.clone();
                            config.provider = provider_id.clone();
                            config
                                .recent_models
                                .retain(|m| m.name != *model_name || m.provider_id != *provider_id);
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
                                    routecode_sdk::agents::resolve_provider_with_config(
                                        provider_id,
                                        key,
                                        &vertex_project,
                                        &vertex_location,
                                    )
                                } else {
                                    routecode_sdk::agents::resolve_provider(provider_id, key)
                                };
                                app.provider_name = provider.name().to_string();
                                app.current_provider_id = provider_id.clone();
                                app.orchestrator.change_provider(provider).await;
                            } else {
                                drop(config);
                            }
                            app.current_model = model_name.clone();
                            app.history.push(Message::system(format!(
                                "Switched to {} on {}",
                                model_name, app.provider_name
                            )));
                            app.show_model_menu = false;
                        } else {
                            app.history.push(Message::system(format!(
                                "Error: No API key for {}",
                                provider_id
                            )));
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
                                    app.pending_account_id = Some(input_value);
                                    app.api_key_input_stage = ApiKeyInputStage::VertexProject;
                                    app.api_key_input = TextArea::default();
                                    app.api_key_input
                                        .set_placeholder_text(" Google Cloud Project ID...");
                                } else {
                                    app.pending_provider_id.take();
                                    let mut config = app.orchestrator.config.lock().await;
                                    config.api_keys.insert(provider_id, input_value);
                                    if let Err(e) =
                                        routecode_sdk::utils::storage::save_config(&config)
                                    {
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
                            app.pending_gateway_id = Some(input_value);
                            app.api_key_input_stage = ApiKeyInputStage::VertexLocation;
                            app.api_key_input = TextArea::default();
                            app.api_key_input
                                .set_placeholder_text(" Location (e.g. us-central1)...");
                        }
                        ApiKeyInputStage::VertexLocation => {
                            if let Some(provider_id) = app.pending_provider_id.take() {
                                let location = input_value;
                                let api_key = app.pending_account_id.take().unwrap_or_default();
                                let project = app.pending_gateway_id.take().unwrap_or_default();
                                let mut config = app.orchestrator.config.lock().await;
                                config.vertex_project = project;
                                config.vertex_location = location;
                                config.api_keys.insert(provider_id, api_key);
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config)
                                {
                                    log::error!("Failed to save config: {}", e);
                                }
                                app.history
                                    .push(Message::system("Vertex AI credentials saved"));
                            }
                            app.is_inputting_api_key = false;
                            app.api_key_input_stage = ApiKeyInputStage::None;
                        }
                        ApiKeyInputStage::CloudflareAccountId => {
                            app.pending_account_id = Some(input_value);
                            app.api_key_input = TextArea::default();
                            if app.pending_provider_id.as_deref() == Some("cloudflare-gateway") {
                                app.api_key_input_stage = ApiKeyInputStage::CloudflareGatewayId;
                            } else {
                                app.api_key_input_stage = ApiKeyInputStage::CloudflareApiKey;
                            }
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
                                    let gateway_id =
                                        app.pending_gateway_id.take().unwrap_or_default();
                                    format!("{}:{}:{}", account_id, gateway_id, input_value)
                                } else {
                                    format!("{}:{}", account_id, input_value)
                                };
                                let mut config = app.orchestrator.config.lock().await;
                                config.api_keys.insert(provider_id.clone(), final_key);
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config)
                                {
                                    log::error!("Failed to save config: {}", e);
                                }
                                app.history.push(Message::system(format!(
                                    "Credentials saved for {}",
                                    provider_id
                                )));
                            }
                            app.is_inputting_api_key = false;
                            app.api_key_input_stage = ApiKeyInputStage::None;
                        }
                        _ => {
                            app.is_inputting_api_key = false;
                        }
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
                        app.history
                            .push(Message::system(format!("Queued: {}", input_text)));
                        app.input = TextArea::default();
                    } else {
                        let provider_id = &app.current_provider_id;
                        let env_key =
                            format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                        let mut api_key = std::env::var(&env_key).ok();
                        if api_key.is_none() && provider_id.starts_with("cloudflare") {
                            api_key = std::env::var("CLOUDFLARE_API_KEY").ok();
                        }
                        if api_key.is_none() {
                            let config = app.orchestrator.config.lock().await;
                            api_key = config.api_keys.get(provider_id).cloned();
                        }

                        let has_valid_key = api_key.is_some_and(|k| !k.trim().is_empty());

                        if !has_valid_key && super::types::provider_requires_api_key(provider_id) {
                            app.history.push(Message::system(format!(
                                "No API key found for {}. Please enter it to continue.",
                                provider_id
                            )));
                            app.show_provider_menu = true;
                            if let Some(pos) = PROVIDERS.iter().position(|p| p.id == *provider_id) {
                                app.menu_state.select(Some(pos));
                            } else {
                                app.menu_state.select(Some(0));
                            }
                            app.input = TextArea::default();
                            return Ok(KeyEventResult::Continue);
                        }

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
                            if let Err(e) =
                                orchestrator.run(&mut history, &model, Some(tx), None).await
                            {
                                log::error!("Orchestrator run failed: {}", e);
                            }
                        });
                    }
                }
            }
        }
        KeyCode::Esc => {
            if app.show_menu {
                app.show_menu = false;
            } else if app.show_provider_menu {
                app.show_provider_menu = false;
            } else if app.show_model_menu {
                app.show_model_menu = false;
            } else if app.show_settings_menu {
                app.show_settings_menu = false;
            } else if app.is_inputting_api_key {
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
            app.history.push(Message::system(format!(
                "Auto-scroll {}",
                if app.auto_scroll {
                    "enabled"
                } else {
                    "disabled"
                }
            )));
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
            if row == 0
                && app.input.lines().len() == 1
                && app.input.lines()[0].is_empty()
                && !app.prompt_history.is_empty()
            {
                let idx = match app.prompt_history_index {
                    Some(i) => {
                        if i == 0 {
                            0
                        } else {
                            i - 1
                        }
                    }
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
            if app.show_menu
                || app.show_provider_menu
                || app.show_model_menu
                || app.show_settings_menu
            {
                let items_len = if app.show_menu {
                    app.filtered_commands.len()
                } else if app.show_provider_menu {
                    PROVIDERS.len()
                } else if app.show_settings_menu {
                    app.settings_items.len()
                } else {
                    app.filtered_models.len()
                };
                if items_len > 0 {
                    let selected = app.menu_state.selected().unwrap_or(0);
                    let mut new_selected = if selected == 0 {
                        items_len - 1
                    } else {
                        selected - 1
                    };
                    if app.show_model_menu {
                        while let Some(ModelMenuItem::Header(_)) =
                            app.filtered_models.get(new_selected)
                        {
                            new_selected = if new_selected == 0 {
                                items_len - 1
                            } else {
                                new_selected - 1
                            };
                            if new_selected == selected {
                                break;
                            }
                        }
                    } else if app.show_settings_menu {
                        while let Some(SettingsMenuItem::Header(_)) =
                            app.settings_items.get(new_selected)
                        {
                            new_selected = if new_selected == 0 {
                                items_len - 1
                            } else {
                                new_selected - 1
                            };
                            if new_selected == selected {
                                break;
                            }
                        }
                    }
                    app.menu_state.select(Some(new_selected));
                }
            } else if app.input.lines().len() == 1 && app.input.lines()[0].is_empty()
                || app.history_scroll > 0
                || app.is_generating
                || key.modifiers.contains(event::KeyModifiers::SHIFT)
            {
                app.history_scroll = app.history_scroll.saturating_sub(15);
                app.auto_scroll = false;
            } else {
                app.input.input(Event::Key(key));
            }
        }
        KeyCode::Down => {
            if app.show_menu
                || app.show_provider_menu
                || app.show_model_menu
                || app.show_settings_menu
            {
                let items_len = if app.show_menu {
                    app.filtered_commands.len()
                } else if app.show_provider_menu {
                    PROVIDERS.len()
                } else if app.show_settings_menu {
                    app.settings_items.len()
                } else {
                    app.filtered_models.len()
                };
                if items_len > 0 {
                    let selected = app.menu_state.selected().unwrap_or(0);
                    let mut new_selected = if selected >= items_len - 1 {
                        0
                    } else {
                        selected + 1
                    };
                    if app.show_model_menu {
                        while let Some(ModelMenuItem::Header(_)) =
                            app.filtered_models.get(new_selected)
                        {
                            new_selected = if new_selected >= items_len - 1 {
                                0
                            } else {
                                new_selected + 1
                            };
                            if new_selected == selected {
                                break;
                            }
                        }
                    } else if app.show_settings_menu {
                        while let Some(SettingsMenuItem::Header(_)) =
                            app.settings_items.get(new_selected)
                        {
                            new_selected = if new_selected >= items_len - 1 {
                                0
                            } else {
                                new_selected + 1
                            };
                            if new_selected == selected {
                                break;
                            }
                        }
                    }
                    app.menu_state.select(Some(new_selected));
                }
            } else if app.input.lines().len() == 1 && app.input.lines()[0].is_empty()
                || app.history_scroll < app.max_scroll
                || app.is_generating
                || key.modifiers.contains(event::KeyModifiers::SHIFT)
            {
                app.history_scroll = app.history_scroll.saturating_add(15);
                if app.history_scroll >= app.max_scroll {
                    app.auto_scroll = true;
                }
            } else {
                app.input.input(Event::Key(key));
            }
        }
        KeyCode::Right if app.show_model_menu => {
            let len = app.filtered_models.len();
            if len > 0 {
                let current = app.menu_state.selected().unwrap_or(0);
                let mut next_header_idx = None;
                for i in (current + 1)..len {
                    if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) {
                        next_header_idx = Some(i);
                        break;
                    }
                }
                if next_header_idx.is_none() {
                    for i in 0..current {
                        if let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(i) {
                            next_header_idx = Some(i);
                            break;
                        }
                    }
                }
                if let Some(h_idx) = next_header_idx {
                    let mut target = (h_idx + 1) % len;
                    while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) {
                        target = (target + 1) % len;
                        if target == h_idx {
                            break;
                        }
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
                    if let ModelMenuItem::Header(_) = item {
                        headers.push(i);
                    }
                }
                if !headers.is_empty() {
                    let current_header_idx_in_headers = headers
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, &h_idx)| h_idx < current)
                        .map(|(i, _)| i);
                    let target_header_idx = match current_header_idx_in_headers {
                        Some(i) => {
                            if i == 0 {
                                *headers.last().unwrap()
                            } else {
                                headers[i - 1]
                            }
                        }
                        None => *headers.last().unwrap(),
                    };
                    let mut target = (target_header_idx + 1) % len;
                    while let Some(ModelMenuItem::Header(_)) = app.filtered_models.get(target) {
                        target = (target + 1) % len;
                        if target == target_header_idx {
                            break;
                        }
                    }
                    app.menu_state.select(Some(target));
                }
            }
        }
        KeyCode::Char('f')
            if key.modifiers.contains(event::KeyModifiers::CONTROL) && app.show_model_menu =>
        {
            if let Some(selected) = app.menu_state.selected() {
                if let Some(ModelMenuItem::Model(model_info)) = app.filtered_models.get(selected) {
                    let model_info = model_info.clone();
                    let mut config = app.orchestrator.config.lock().await;
                    if config.favorites.iter().any(|m| {
                        m.name == model_info.name && m.provider_id == model_info.provider_id
                    }) {
                        config.favorites.retain(|m| {
                            m.name != model_info.name || m.provider_id != model_info.provider_id
                        });
                        app.history.push(Message::system(format!(
                            "Removed {} from favorites",
                            model_info.name
                        )));
                    } else {
                        config.favorites.push(model_info.clone());
                        app.history.push(Message::system(format!(
                            "Added {} to favorites",
                            model_info.name
                        )));
                    }
                    if let Err(e) = routecode_sdk::utils::storage::save_config(&config) {
                        log::error!("Failed to save config: {}", e);
                    }
                }
            }
        }
        KeyCode::BackTab => {
            app.approval_mode = app.approval_mode.next();
            let info = match app.approval_mode {
                ApprovalMode::YOLO => "YOLO -- commands will auto-approve",
                ApprovalMode::Plan => "PLAN -- tool calls will be denied (read-only review)",
                ApprovalMode::Shell => "SHELL -- shell commands shown first, auto-approved",
                ApprovalMode::Normal => "Normal mode -- confirm each tool call",
            };
            app.history.push(Message::system(format!("Mode: {}", info)));
        }
        _ => {
            let event = Event::Key(key);
            if app.is_inputting_api_key {
                app.api_key_input.input(event);
            } else if app.show_model_menu {
                if app.model_search_input.input(event) {
                    let search = app
                        .model_search_input
                        .lines()
                        .first()
                        .map(|l| l.trim().to_lowercase())
                        .unwrap_or_default();
                    handle_model_search(app, &search, true).await;
                }
            } else {
                app.input.input(event);
                app.update_filtered_commands();
            }
        }
    }
    Ok(KeyEventResult::Continue)
}

pub(crate) async fn handle_mouse_event<B: ratatui::backend::Backend>(
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
            if app.show_menu
                || app.show_provider_menu
                || app.show_model_menu
                || app.show_settings_menu
            {
                let mut current = app.menu_state.selected().unwrap_or(0);
                current = current.saturating_sub(3);
                if app.show_model_menu {
                    while current > 0
                        && matches!(
                            app.filtered_models.get(current),
                            Some(ModelMenuItem::Header(_))
                        )
                    {
                        current -= 1;
                    }
                } else if app.show_settings_menu {
                    while current > 0
                        && matches!(
                            app.settings_items.get(current),
                            Some(SettingsMenuItem::Header(_))
                        )
                    {
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
            if app.show_menu
                || app.show_provider_menu
                || app.show_model_menu
                || app.show_settings_menu
            {
                let current = app.menu_state.selected().unwrap_or(0);
                let max = if app.show_menu {
                    app.filtered_commands.len()
                } else if app.show_provider_menu {
                    PROVIDERS.len()
                } else if app.show_settings_menu {
                    app.settings_items.len()
                } else {
                    app.filtered_models.len()
                };
                let mut next = current.saturating_add(3).min(max.saturating_sub(1));
                if app.show_model_menu {
                    while next < max - 1
                        && matches!(
                            app.filtered_models.get(next),
                            Some(ModelMenuItem::Header(_))
                        )
                    {
                        next += 1;
                    }
                } else if app.show_settings_menu {
                    while next < max - 1
                        && matches!(
                            app.settings_items.get(next),
                            Some(SettingsMenuItem::Header(_))
                        )
                    {
                        next += 1;
                    }
                }
                app.menu_state.select(Some(next));
            } else {
                app.history_scroll = app.history_scroll.saturating_add(15);
                if app.history_scroll >= app.max_scroll {
                    app.auto_scroll = true;
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
            if let Some(msg_idx) = app.show_user_msg_modal {
                if let Ok(size) = terminal.size() {
                    let width = (size.width as f32 * 0.40) as u16;
                    let height = 8;
                    let modal_x = (size.width.saturating_sub(width)) / 2;
                    let modal_y = (size.height.saturating_sub(height)) / 2;

                    let is_outside = mouse.column < modal_x
                        || mouse.column >= modal_x + width
                        || mouse.row < modal_y
                        || mouse.row >= modal_y + height;

                    if is_outside {
                        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                            app.show_user_msg_modal = None;
                        }
                    } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                        let click_row = mouse.row;
                        if click_row == modal_y + 2 {
                            app.user_msg_modal_selected = 0;
                            let text = app.history[msg_idx]
                                .content
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            let text_clone = text.clone();
                            tokio::task::spawn_blocking(move || {
                                if let Err(e) = copy_to_clipboard(&text_clone) {
                                    log::error!("Clipboard copy failed: {}", e);
                                }
                            });
                            app.history
                                .push(Message::system("Message copied to clipboard!".to_string()));
                            app.show_user_msg_modal = None;
                        } else if click_row == modal_y + 3 {
                            app.user_msg_modal_selected = 1;
                            let text = app.history[msg_idx]
                                .content
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_default();
                            app.history.truncate(msg_idx);
                            app.input =
                                tui_textarea::TextArea::from(text.lines().map(|s| s.to_string()));
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

                    let is_outside = mouse.column < modal_x
                        || mouse.column >= modal_x + width
                        || mouse.row < modal_y
                        || mouse.row >= modal_y + height;

                    if is_outside {
                        if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                            app.pending_update = None;
                        }
                    } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                        && mouse.row == modal_y + height.saturating_sub(2)
                    {
                        if mouse.column >= modal_x + width.saturating_sub(25)
                            && mouse.column < modal_x + width.saturating_sub(15)
                        {
                            app.pending_update = None;
                        } else if mouse.column >= modal_x + width.saturating_sub(15)
                            && mouse.column < modal_x + width
                        {
                            app.pending_update_install = true;
                        }
                    }
                }
            } else if app.show_menu
                || app.show_provider_menu
                || app.show_model_menu
                || app.show_settings_menu
            {
                if let Ok(size) = terminal.size() {
                    let (width, height) = if app.show_menu {
                        (60, (app.filtered_commands.len() + 6).min(15) as u16)
                    } else if app.show_provider_menu {
                        (60, (PROVIDERS.len() + 6).min(15) as u16)
                    } else if app.show_settings_menu {
                        (60, (app.settings_items.len() + 6).min(15) as u16)
                    } else {
                        (70, (app.filtered_models.len() + 7).min(18) as u16)
                    };
                    let modal_x = (size.width.saturating_sub(width)) / 2;
                    let modal_y = (size.height.saturating_sub(height)) / 2;

                    let is_outside = mouse.column < modal_x
                        || mouse.column >= modal_x + width
                        || mouse.row < modal_y
                        || mouse.row >= modal_y + height;
                    let is_esc = mouse.row <= modal_y + 2
                        && mouse.column >= modal_x + width.saturating_sub(10)
                        && mouse.column <= modal_x + width;
                    let is_inside_list = mouse.row >= modal_y + 2
                        && mouse.row < modal_y + height - 1
                        && mouse.column > modal_x
                        && mouse.column < modal_x + width - 1;

                    if is_outside || is_esc {
                        app.show_menu = false;
                        app.show_provider_menu = false;
                        app.show_model_menu = false;
                        app.show_settings_menu = false;
                    } else if is_inside_list
                        && matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                        && app.show_settings_menu
                    {
                        let idx = (mouse.row - (modal_y + 2)) as usize + app.menu_state.offset();
                        if idx < app.settings_items.len() {
                            if let Some(SettingsMenuItem::Option { key, val: _, .. }) =
                                app.settings_items.get(idx)
                            {
                                let key = key.clone();
                                apply_settings_toggle(app, &key).await;
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
                    let in_cooldown = app
                        .last_toggle_time
                        .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(400));

                    if !in_cooldown && has_thinking {
                        let is_double_click = if let Some((last_time, col, row)) = app.last_click_up
                        {
                            let col_diff = (col as i32 - mouse.column as i32).abs();
                            let row_diff = (row as i32 - mouse.row as i32).abs();
                            last_time.elapsed() < std::time::Duration::from_millis(600)
                                && col_diff <= 4
                                && row_diff <= 3
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
                                app.last_click_up =
                                    Some((std::time::Instant::now(), mouse.column, mouse.row));
                                app.mouse_down_start =
                                    Some((std::time::Instant::now(), mouse.column, mouse.row));
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
            } else if app.screen == Screen::Welcome
                && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
            {
                if let Ok(size) = terminal.size() {
                    let logo_height = if size.height < 20 { 0 } else { 6 };
                    let spacer_height = if size.height < 15 { 0 } else { size.height / 3 };
                    if logo_height > 0
                        && mouse.row >= spacer_height
                        && mouse.row < spacer_height + logo_height
                    {
                        app.logo_anim_frames = 20; // 2 seconds at 100ms tick
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use routecode_sdk::agents::traits::StreamResponse;
    use routecode_sdk::agents::AIProvider;
    use routecode_sdk::core::{AgentOrchestrator, Config};
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
        ) -> Result<StreamResponse, anyhow::Error> {
            Err(anyhow::anyhow!("Not implemented"))
        }
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
        app.history.push(Message::assistant(
            Some("Assistant reply".into()),
            None,
            None,
        ));
        app.history
            .push(Message::user("Second message".to_string()));

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
