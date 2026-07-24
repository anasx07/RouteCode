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
    ActiveModal, ApiKeyInputStage, ApprovalMode, ModelMenuItem, Screen, SettingsMenuItem, PROVIDERS,
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
    if app.active_modal == ActiveModal::Update {
        match key.code {
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Tab => {
                app.update.selected = if app.update.selected == 0 { 1 } else { 0 };
            }
            KeyCode::Left | KeyCode::Char('h') => {
                app.update.selected = if app.update.selected == 1 { 0 } else { 1 };
            }
            KeyCode::Enter => {
                if app.update.selected == 1 {
                    app.update.install = true;
                    return Ok(KeyEventResult::Exit);
                } else {
                    app.update.pending_version = None;
                    app.active_modal = ActiveModal::None;
                }
            }
            KeyCode::Esc => {
                app.update.pending_version = None;
                app.active_modal = ActiveModal::None;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::UserMessage {
        if let Some(msg_idx) = app.user_msg.msg_idx {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    app.user_msg.selected = if app.user_msg.selected == 0 {
                        1
                    } else {
                        0
                    };
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    app.user_msg.selected = if app.user_msg.selected == 1 {
                        0
                    } else {
                        1
                    };
                }
                KeyCode::Enter => {
                    let text = app.session.history[msg_idx]
                        .content
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    if app.user_msg.selected == 0 {
                        let text_clone = text.clone();
                        match copy_to_clipboard(&text_clone) {
                            Ok(()) => {
                                app.session.history
                                    .push(Message::system("Message copied to clipboard!".to_string()));
                            }
                            Err(e) => {
                                log::error!("Clipboard copy failed: {}", e);
                                app.session.history.push(Message::system(format!(
                                    "Failed to copy to clipboard: {}. Make sure a clipboard utility (e.g. xclip/wl-clipboard on Linux, clip on Windows, pbcopy on macOS) is installed.",
                                    e
                                )));
                            }
                        }
                    } else {
                        app.session.history.truncate(msg_idx);
                        app.input = tui_textarea::TextArea::from(text.lines().map(|s| s.to_string()));
                        app.input.move_cursor(tui_textarea::CursorMove::End);
                    }
                    app.user_msg.msg_idx = None;
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Esc => {
                    app.user_msg.msg_idx = None;
                    app.active_modal = ActiveModal::None;
                }
                _ => {}
            }
        } else {
            app.active_modal = ActiveModal::None;
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::CommandConfirmation {
        if app.cmd_confirmation.inputting_feedback {
            match key.code {
                KeyCode::Esc => {
                    app.cmd_confirmation.inputting_feedback = false;
                    app.input.delete_line_by_head();
                    while app.input.cursor() != (0, 0) {
                        app.input.move_cursor(tui_textarea::CursorMove::Head);
                        app.input.delete_line_by_head();
                    }
                    app.input
                        .set_placeholder_text(" Ask anything... \"How do I use this?\"");
                }
                KeyCode::Enter => {
                    if let Some((_, _, tx_mutex)) = app.cmd_confirmation.pending.take() {
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
                    app.cmd_confirmation.inputting_feedback = false;
                    app.active_modal = ActiveModal::None;
                }
                _ => {
                    app.input.input(key);
                }
            }
        } else {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    if let Some((_, _, tx_mutex)) = app.cmd_confirmation.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::AllowOnce);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let mut config =
                        routecode_sdk::utils::storage::load_session_config(&app.session.id)
                            .unwrap_or_default();
                    config.allow_all_commands = true;
                    let _ = routecode_sdk::utils::storage::save_session_config(
                        &app.session.id,
                        &config,
                    );

                    if let Some((_, _, tx_mutex)) = app.cmd_confirmation.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::AllowSession);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('w') | KeyCode::Char('W') => {
                    let mut config =
                        routecode_sdk::utils::storage::load_workspace_config().unwrap_or_default();
                    config.allow_all_outside_access = true;
                    let _ = routecode_sdk::utils::storage::save_workspace_config(&config);

                    if let Some((_, _, tx_mutex)) = app.cmd_confirmation.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::AllowWorkspace);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Esc => {
                    if let Some((_, _, tx_mutex)) = app.cmd_confirmation.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(tx) = tx_opt.take() {
                            let _ = tx.send(ConfirmationResponse::Deny);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.cmd_confirmation.inputting_feedback = true;
                    app.input
                        .set_placeholder_text(" Tell agent (e.g. 'don't run without backup')...");
                }
                _ => {}
            }
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::PlanApproval {
        use routecode_sdk::agents::types::PlanApprovalResponse;
        if app.plan_approval.inputting_feedback {
            match key.code {
                KeyCode::Esc => {
                    app.plan_approval.inputting_feedback = false;
                    app.input.delete_line_by_head();
                    while app.input.cursor() != (0, 0) {
                        app.input.move_cursor(tui_textarea::CursorMove::Head);
                        app.input.delete_line_by_head();
                    }
                    app.input
                        .set_placeholder_text(" Ask anything... \"How do I use this?\"");
                }
                KeyCode::Enter => {
                    if let Some((_, _, _, tx_mutex)) = app.plan_approval.pending.take() {
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
                            "Plan cancelled.".to_string()
                        } else {
                            msg
                        };
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(s) = tx_opt.take() {
                            let _ = s.send(PlanApprovalResponse::Feedback(feedback));
                        }
                    }
                    app.plan_approval.inputting_feedback = false;
                    app.active_modal = ActiveModal::None;
                }
                _ => {
                    app.input.input(key);
                }
            }
        } else {
            match key.code {
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    if let Some((_, _, _, tx_mutex)) = app.plan_approval.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(s) = tx_opt.take() {
                            let _ = s.send(PlanApprovalResponse::ApproveAndUnlock);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    if let Some((_, _, _, tx_mutex)) = app.plan_approval.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(s) = tx_opt.take() {
                            let _ = s.send(PlanApprovalResponse::ApproveOnce);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Esc => {
                    if let Some((_, _, _, tx_mutex)) = app.plan_approval.pending.take() {
                        let mut tx_opt = tx_mutex.lock().await;
                        if let Some(s) = tx_opt.take() {
                            let _ = s.send(PlanApprovalResponse::Deny);
                        }
                    }
                    app.active_modal = ActiveModal::None;
                }
                KeyCode::Char('f') | KeyCode::Char('F') => {
                    app.plan_approval.inputting_feedback = true;
                    app.input
                        .set_placeholder_text(" Tell agent how to revise the plan...");
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if app.plan_approval.selected > 0 {
                        app.plan_approval.selected -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if app.plan_approval.selected < 3 {
                        app.plan_approval.selected += 1;
                    }
                }
                KeyCode::Enter => {
                    // Activate the currently highlighted button
                    let which = app.plan_approval.selected;
                    match which {
                        2 => {
                            app.plan_approval.inputting_feedback = true;
                            app.input
                                .set_placeholder_text(
                                    " Tell agent how to revise the plan...",
                                );
                        }
                        _ => {
                            if let Some((_, _, _, tx_mutex)) =
                                app.plan_approval.pending.take()
                            {
                                let mut tx_opt = tx_mutex.lock().await;
                                if let Some(s) = tx_opt.take() {
                                    let resp = match which {
                                        0 => PlanApprovalResponse::ApproveAndUnlock,
                                        1 => PlanApprovalResponse::ApproveOnce,
                                        _ => PlanApprovalResponse::Deny,
                                    };
                                    let _ = s.send(resp);
                                }
                            }
                            app.active_modal = ActiveModal::None;
                        }
                    }
                }
                _ => {}
            }
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::HookTrust {
        use routecode_sdk::agents::types::HookTrustResponse;
        match key.code {
            KeyCode::Char('t') | KeyCode::Char('T') | KeyCode::Enter => {
                if let Some(state) = app.hook_trust.pending.take() {
                    let mut tx_opt = state.tx.lock().await;
                    if let Some(s) = tx_opt.take() {
                        let _ = s.send(HookTrustResponse::Trust);
                    }
                }
                app.active_modal = ActiveModal::None;
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Esc => {
                if let Some(state) = app.hook_trust.pending.take() {
                    let mut tx_opt = state.tx.lock().await;
                    if let Some(s) = tx_opt.take() {
                        let _ = s.send(HookTrustResponse::Deny);
                    }
                }
                app.active_modal = ActiveModal::None;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if app.hook_trust.selected > 0 {
                    app.hook_trust.selected -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if app.hook_trust.selected < 1 {
                    app.hook_trust.selected += 1;
                }
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::ClearConfirmation {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.session.history.clear();
                app.screen = Screen::Welcome;
                app.session.history_scroll = 0;
                app.active_modal = ActiveModal::None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.active_modal = ActiveModal::None;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    if app.active_modal == ActiveModal::ExitConfirmation {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                app.tasks.abort_all();
                app.session.is_generating = false;
                app.session.active_tool = None;
                if !app.session.history.is_empty() {
                    let session = routecode_sdk::utils::storage::Session {
                        messages: app.session.history.clone(),
                        model: app.provider.current_model.clone(),
                        usage: app.orchestrator.usage.lock().await.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    let _ = routecode_sdk::utils::storage::save_session(&app.session.id, &session);
                }
                return Ok(KeyEventResult::Exit);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.active_modal = ActiveModal::None;
            }
            _ => {}
        }
        return Ok(KeyEventResult::Continue);
    }
    match key.code {
        KeyCode::Char('p') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.active_modal = ActiveModal::CommandMenu;
            app.menu.list_state.select(Some(0));
            app.update_filtered_commands();
        }
        KeyCode::Char('a') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            if app.active_modal == ActiveModal::ModelMenu {
                app.active_modal = ActiveModal::None;
            }
            app.active_modal = ActiveModal::ProviderMenu;
            app.menu.list_state.select(Some(0));
        }
        KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            if app.session.is_generating {
                app.tasks.abort_all();
                app.session.is_generating = false;
                app.session.active_tool = None;
            }
        }
        KeyCode::Char('l') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.session.history.clear();
            app.screen = Screen::Welcome;
            app.session.history_scroll = 0;
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
            } else if app.active_modal == ActiveModal::CommandMenu {
                if let Some(selected) = app.menu.list_state.selected() {
                    if let Some(cmd) = app.menu.filtered_commands.get(selected) {
                        let name = cmd.name.to_string();
                        app.active_modal = ActiveModal::None;
                        app.input = TextArea::default();
                        handle_command(app, &name).await;
                    }
                }
            } else if app.active_modal == ActiveModal::ProviderMenu {
                if let Some(selected) = app.menu.list_state.selected() {
                    if let Some(p) = PROVIDERS.get(selected) {
                        app.api_key.pending_provider_id = Some(p.id.to_string());
                        app.active_modal = ActiveModal::ApiKeyInput;
                        app.api_key.input = TextArea::default();
                        if p.id == "cloudflare-workers" || p.id == "cloudflare-gateway" {
                            app.api_key.stage = ApiKeyInputStage::CloudflareAccountId;
                        } else {
                            app.api_key.stage = ApiKeyInputStage::ApiKey;
                        }
                    }
                }
            } else if app.active_modal == ActiveModal::SettingsMenu {
                if let Some(selected) = app.menu.list_state.selected() {
                    if let Some(SettingsMenuItem::Option { key, val: _, .. }) =
                        app.menu.settings_items.get(selected)
                    {
                        let key = key.clone();
                        apply_settings_toggle(app, &key).await;
                    }
                }
            } else if app.active_modal == ActiveModal::ModelMenu {
                if let Some(selected) = app.menu.list_state.selected() {
                    if let Some(ModelMenuItem::Model(model_info)) =
                        app.model_search.filtered_models.get(selected)
                    {
                        let model_info = model_info.clone();
                        let provider_id = &model_info.provider_id;
                        let model_name = &model_info.name;
                        let mut config = app.orchestrator.config.lock().await;
                        let env_key =
                            format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                        let api_key = std::env::var(env_key)
                            .ok()
                            .or_else(|| {
                                if provider_id == "vertex" {
                                    std::env::var("GOOGLE_API_KEY").ok()
                                } else {
                                    None
                                }
                            })
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
                            if app.provider.provider_name.to_lowercase() != *provider_id {
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
                                app.provider.provider_name = provider.name().to_string();
                                app.provider.current_provider_id = provider_id.clone();
                                app.orchestrator.change_provider(provider).await;
                            } else {
                                drop(config);
                            }
                            app.provider.current_model = model_name.clone();
                            app.session.history.push(Message::system(format!(
                                "Switched to {} on {}",
                                model_name, app.provider.provider_name
                            )));
                            app.active_modal = ActiveModal::None;
                        } else {
                            app.session.history.push(Message::system(format!(
                                "Error: No API key for {}",
                                provider_id
                            )));
                        }
                    }
                }
            } else if app.active_modal == ActiveModal::ApiKeyInput {
                let input_value = app.api_key.input.lines().join("\n").trim().to_string();
                if !input_value.is_empty() {
                    match app.api_key.stage {
                        ApiKeyInputStage::ApiKey => {
                            if let Some(provider_id) = app.api_key.pending_provider_id.clone() {
                                if provider_id == "vertex" {
                                    app.api_key.pending_account_id = Some(input_value);
                                    app.api_key.stage = ApiKeyInputStage::VertexProject;
                                    app.api_key.input = TextArea::default();
                                    app.api_key.input
                                        .set_placeholder_text(" Google Cloud Project ID...");
                                } else {
                                    app.api_key.pending_provider_id.take();
                                    let mut config = app.orchestrator.config.lock().await;
                                    config.api_keys.insert(provider_id, input_value);
                                    if let Err(e) =
                                        routecode_sdk::utils::storage::save_config(&config)
                                    {
                                        log::error!("Failed to save config: {}", e);
                                    }
                                    app.session.history.push(Message::system("API Key saved"));
                                    app.active_modal = ActiveModal::None;
                                    app.api_key.stage = ApiKeyInputStage::None;
                                }
                            } else {
                                app.active_modal = ActiveModal::None;
                                app.api_key.stage = ApiKeyInputStage::None;
                            }
                        }
                        ApiKeyInputStage::VertexProject => {
                            app.api_key.pending_gateway_id = Some(input_value);
                            app.api_key.stage = ApiKeyInputStage::VertexLocation;
                            app.api_key.input = TextArea::default();
                            app.api_key.input
                                .set_placeholder_text(" Location (e.g. us-central1)...");
                        }
                        ApiKeyInputStage::VertexLocation => {
                            if let Some(provider_id) = app.api_key.pending_provider_id.take() {
                                let location = input_value;
                                let api_key = app.api_key.pending_account_id.take().unwrap_or_default();
                                let project = app.api_key.pending_gateway_id.take().unwrap_or_default();
                                let mut config = app.orchestrator.config.lock().await;
                                config.vertex_project = project;
                                config.vertex_location = location;
                                config.api_keys.insert(provider_id, api_key);
                                if let Err(e) = routecode_sdk::utils::storage::save_config(&config)
                                {
                                    log::error!("Failed to save config: {}", e);
                                }
                                app.session.history
                                    .push(Message::system("Vertex AI credentials saved"));
                            }
                            app.active_modal = ActiveModal::None;
                            app.api_key.stage = ApiKeyInputStage::None;
                        }
                        ApiKeyInputStage::CloudflareAccountId => {
                            app.api_key.pending_account_id = Some(input_value);
                            app.api_key.input = TextArea::default();
                            if app.api_key.pending_provider_id.as_deref() == Some("cloudflare-gateway") {
                                app.api_key.stage = ApiKeyInputStage::CloudflareGatewayId;
                            } else {
                                app.api_key.stage = ApiKeyInputStage::CloudflareApiKey;
                            }
                        }
                        ApiKeyInputStage::CloudflareGatewayId => {
                            app.api_key.pending_gateway_id = Some(input_value);
                            app.api_key.input = TextArea::default();
                            app.api_key.stage = ApiKeyInputStage::CloudflareApiKey;
                        }
                        ApiKeyInputStage::CloudflareApiKey => {
                            if let Some(provider_id) = app.api_key.pending_provider_id.take() {
                                let account_id = app.api_key.pending_account_id.take().unwrap_or_default();
                                let final_key = if provider_id == "cloudflare-gateway" {
                                    let gateway_id =
                                        app.api_key.pending_gateway_id.take().unwrap_or_default();
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
                                app.session.history.push(Message::system(format!(
                                    "Credentials saved for {}",
                                    provider_id
                                )));
                            }
                            app.active_modal = ActiveModal::None;
                            app.api_key.stage = ApiKeyInputStage::None;
                        }
                        _ => {
                            app.active_modal = ActiveModal::None;
                        }
                    }
                } else {
                    app.active_modal = ActiveModal::None;
                    app.api_key.stage = ApiKeyInputStage::None;
                    app.api_key.pending_provider_id = None;
                    app.api_key.pending_account_id = None;
                    app.api_key.pending_gateway_id = None;
                }
            } else {
                let input_text = app.input.lines().join("\n");
                if !input_text.trim().is_empty() {
                    if input_text.starts_with('/') {
                        handle_command(app, &input_text).await;
                    } else if app.session.is_generating {
                        // Ignore normal text submissions while generating to avoid parallel tasks
                    } else if !app.session.startup_ready {
                        app.session.startup_input_buffer.push(input_text.clone());
                        app.session.history
                            .push(Message::system(format!("Queued: {}", input_text)));
                        app.input = TextArea::default();
                    } else {
                        let provider_id = &app.provider.current_provider_id;
                        let env_key =
                            format!("{}_API_KEY", provider_id.to_uppercase().replace("-", "_"));
                        let mut api_key = std::env::var(&env_key).ok();
                        if api_key.is_none() && provider_id.starts_with("cloudflare") {
                            api_key = std::env::var("CLOUDFLARE_API_KEY").ok();
                        }
                        if api_key.is_none() && provider_id == "vertex" {
                            api_key = std::env::var("GOOGLE_API_KEY").ok();
                        }
                        if api_key.is_none() {
                            let config = app.orchestrator.config.lock().await;
                            api_key = config.api_keys.get(provider_id).cloned();
                        }

                        let has_valid_key = api_key.is_some_and(|k| !k.trim().is_empty());

                        if !has_valid_key && super::types::provider_requires_api_key(provider_id) {
                            app.session.history.push(Message::system(format!(
                                "No API key found for {}. Please enter it to continue.",
                                provider_id
                            )));
                            app.active_modal = ActiveModal::ProviderMenu;
                            if let Some(pos) = PROVIDERS.iter().position(|p| p.id == *provider_id) {
                                app.menu.list_state.select(Some(pos));
                            } else {
                                app.menu.list_state.select(Some(0));
                            }
                            app.input = TextArea::default();
                            return Ok(KeyEventResult::Continue);
                        }

                        app.session.history.push(Message::user(input_text.clone()));
                        app.session.prompt_history.push(input_text.clone());
                        app.session.prompt_history.truncate(100);
                        app.session.prompt_history_index = None;
                        app.input = TextArea::default();
                        app.screen = Screen::Session;
                        app.session.is_generating = true;
                        app.session.auto_scroll = true;
                        let orchestrator = app.orchestrator.clone();
                        let mut history = app.session.history.clone();
                        let model = app.provider.current_model.clone();
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
            if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                app.active_modal = ActiveModal::None;
            } else if app.active_modal == ActiveModal::ApiKeyInput {
                app.active_modal = ActiveModal::None;
                app.api_key.stage = ApiKeyInputStage::None;
                app.api_key.pending_provider_id = None;
                app.api_key.pending_account_id = None;
                app.api_key.pending_gateway_id = None;
            } else if app.session.is_generating {
                app.tasks.abort_all();
                app.session.is_generating = false;
                app.session.active_tool = None;
            } else {
                app.active_modal = ActiveModal::ExitConfirmation;
            }
        }
        KeyCode::Char('t') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.session.auto_scroll = !app.session.auto_scroll;
            app.session.history.push(Message::system(format!(
                "Auto-scroll {}",
                if app.session.auto_scroll {
                    "enabled"
                } else {
                    "disabled"
                }
            )));
        }
        KeyCode::Char('o') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            app.session.collapse_thinking = !app.session.collapse_thinking;
        }
        KeyCode::End => {
            app.session.auto_scroll = true;
            app.session.history_scroll = app.session.max_scroll;
        }
        KeyCode::Up if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            let (row, _) = app.input.cursor();
            if row == 0
                && app.input.lines().len() == 1
                && app.input.lines()[0].is_empty()
                && !app.session.prompt_history.is_empty()
            {
                let idx = match app.session.prompt_history_index {
                    Some(i) => {
                        if i == 0 {
                            0
                        } else {
                            i - 1
                        }
                    }
                    None => app.session.prompt_history.len() - 1,
                };
                app.session.prompt_history_index = Some(idx);
                let prev = app.session.prompt_history[idx].clone();
                app.input = TextArea::from(prev.lines().map(|s| s.to_string()));
                app.input.move_cursor(tui_textarea::CursorMove::End);
            }
        }
        KeyCode::Down if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
            let (row, _) = app.input.cursor();
            let lines_len = app.input.lines().len();
            if row >= lines_len - 1 && app.session.prompt_history_index.is_some() {
                let idx = app.session.prompt_history_index.unwrap();
                if idx >= app.session.prompt_history.len() - 1 {
                    app.session.prompt_history_index = None;
                    app.input = TextArea::default();
                } else {
                    let new_idx = idx + 1;
                    app.session.prompt_history_index = Some(new_idx);
                    let next = app.session.prompt_history[new_idx].clone();
                    app.input = TextArea::from(next.lines().map(|s| s.to_string()));
                    app.input.move_cursor(tui_textarea::CursorMove::End);
                }
            }
        }
        KeyCode::Up => {
            if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                let items_len = match app.active_modal {
                    ActiveModal::CommandMenu => app.menu.filtered_commands.len(),
                    ActiveModal::ProviderMenu => PROVIDERS.len(),
                    ActiveModal::SettingsMenu => app.menu.settings_items.len(),
                    ActiveModal::ModelMenu => app.model_search.filtered_models.len(),
                    _ => 0,
                };
                if items_len > 0 {
                    let selected = app.menu.list_state.selected().unwrap_or(0);
                    let mut new_selected = if selected == 0 {
                        items_len - 1
                    } else {
                        selected - 1
                    };
                    if app.active_modal == ActiveModal::ModelMenu {
                        while let Some(ModelMenuItem::Header(_)) =
                            app.model_search.filtered_models.get(new_selected)
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
                    } else if app.active_modal == ActiveModal::SettingsMenu {
                        while let Some(SettingsMenuItem::Header(_)) =
                            app.menu.settings_items.get(new_selected)
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
                    app.menu.list_state.select(Some(new_selected));
                }
            } else {
                let (cursor_row, _) = app.input.cursor();
                if (app.input.lines().len() == 1 && app.input.lines()[0].is_empty())
                    || (cursor_row == 0 && (app.session.history_scroll > 0 || app.session.is_generating || key.modifiers.contains(event::KeyModifiers::SHIFT)))
                {
                    app.session.history_scroll = app.session.history_scroll.saturating_sub(15);
                    app.session.auto_scroll = false;
                } else {
                    app.input.input(Event::Key(key));
                }
            }
        }
        KeyCode::Down => {
            if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                let items_len = match app.active_modal {
                    ActiveModal::CommandMenu => app.menu.filtered_commands.len(),
                    ActiveModal::ProviderMenu => PROVIDERS.len(),
                    ActiveModal::SettingsMenu => app.menu.settings_items.len(),
                    ActiveModal::ModelMenu => app.model_search.filtered_models.len(),
                    _ => 0,
                };
                if items_len > 0 {
                    let selected = app.menu.list_state.selected().unwrap_or(0);
                    let mut new_selected = if selected >= items_len - 1 {
                        0
                    } else {
                        selected + 1
                    };
                    if app.active_modal == ActiveModal::ModelMenu {
                        while let Some(ModelMenuItem::Header(_)) =
                            app.model_search.filtered_models.get(new_selected)
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
                    } else if app.active_modal == ActiveModal::SettingsMenu {
                        while let Some(SettingsMenuItem::Header(_)) =
                            app.menu.settings_items.get(new_selected)
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
                    app.menu.list_state.select(Some(new_selected));
                }
            } else {
                let (cursor_row, _) = app.input.cursor();
                let lines_len = app.input.lines().len();
                if (lines_len == 1 && app.input.lines()[0].is_empty())
                    || (cursor_row == lines_len - 1 && (app.session.history_scroll < app.session.max_scroll || app.session.is_generating || key.modifiers.contains(event::KeyModifiers::SHIFT)))
                {
                    app.session.history_scroll = app.session.history_scroll.saturating_add(15);
                    if app.session.history_scroll >= app.session.max_scroll {
                        app.session.auto_scroll = true;
                    }
                } else {
                    app.input.input(Event::Key(key));
                }
            }
        }
        KeyCode::Right if app.active_modal == ActiveModal::ModelMenu => {
            let len = app.model_search.filtered_models.len();
            if len > 0 {
                let current = app.menu.list_state.selected().unwrap_or(0);
                let mut next_header_idx = None;
                for i in (current + 1)..len {
                    if let Some(ModelMenuItem::Header(_)) = app.model_search.filtered_models.get(i) {
                        next_header_idx = Some(i);
                        break;
                    }
                }
                if next_header_idx.is_none() {
                    for i in 0..current {
                        if let Some(ModelMenuItem::Header(_)) = app.model_search.filtered_models.get(i) {
                            next_header_idx = Some(i);
                            break;
                        }
                    }
                }
                if let Some(h_idx) = next_header_idx {
                    let mut target = (h_idx + 1) % len;
                    while let Some(ModelMenuItem::Header(_)) = app.model_search.filtered_models.get(target) {
                        target = (target + 1) % len;
                        if target == h_idx {
                            break;
                        }
                    }
                    app.menu.list_state.select(Some(target));
                }
            }
        }
        KeyCode::Left if app.active_modal == ActiveModal::ModelMenu => {
            let len = app.model_search.filtered_models.len();
            if len > 0 {
                let current = app.menu.list_state.selected().unwrap_or(0);
                let mut headers = Vec::new();
                for (i, item) in app.model_search.filtered_models.iter().enumerate() {
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
                    while let Some(ModelMenuItem::Header(_)) = app.model_search.filtered_models.get(target) {
                        target = (target + 1) % len;
                        if target == target_header_idx {
                            break;
                        }
                    }
                    app.menu.list_state.select(Some(target));
                }
            }
        }
        KeyCode::Char('f')
            if key.modifiers.contains(event::KeyModifiers::CONTROL) && app.active_modal == ActiveModal::ModelMenu =>
        {
            if let Some(selected) = app.menu.list_state.selected() {
                if let Some(ModelMenuItem::Model(model_info)) = app.model_search.filtered_models.get(selected) {
                    let model_info = model_info.clone();
                    let mut config = app.orchestrator.config.lock().await;
                    if config.favorites.iter().any(|m| {
                        m.name == model_info.name && m.provider_id == model_info.provider_id
                    }) {
                        config.favorites.retain(|m| {
                            m.name != model_info.name || m.provider_id != model_info.provider_id
                        });
                        app.session.history.push(Message::system(format!(
                            "Removed {} from favorites",
                            model_info.name
                        )));
                    } else {
                        config.favorites.push(model_info.clone());
                        app.session.history.push(Message::system(format!(
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
            app.session.approval_mode = app.session.approval_mode.next();
            let info = match app.session.approval_mode {
                ApprovalMode::YOLO => {
                    app.orchestrator.exit_plan_mode(false);
                    "YOLO -- commands will auto-approve"
                }
                ApprovalMode::Plan => {
                    // Mirror the UI state into the orchestrator: enter
                    // plan mode, force bash to read-only, reset
                    // session-unlock.
                    app.orchestrator.enter_plan_mode();
                    let mut cfg = app.orchestrator.config.lock().await;
                    cfg.bash_mode = routecode_sdk::core::config::BashMode::ReadOnly;
                    drop(cfg);
                    "PLAN -- plan mode active: write tools hidden, bash read-only. \
                     Use exit_plan_mode (model) to unlock writes."
                }
                ApprovalMode::Shell => {
                    app.orchestrator.exit_plan_mode(false);
                    "SHELL -- shell commands shown first, auto-approved"
                }
                ApprovalMode::Normal => {
                    // Leaving Plan mode (either toward YOLO/Shell or
                    // back to Normal from a previous Plan): exit plan
                    // mode in the orchestrator.
                    app.orchestrator.exit_plan_mode(false);
                    "Normal mode -- confirm each tool call"
                }
            };
            app.session.history.push(Message::system(format!("Mode: {}", info)));
        }
        _ => {
            let event = Event::Key(key);
            if app.active_modal == ActiveModal::ApiKeyInput {
                app.api_key.input.input(event);
            } else if app.active_modal == ActiveModal::ModelMenu {
                if app.model_search.search_input.input(event) {
                    let search = app
                        .model_search.search_input
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
    app.mouse.events_count += 1;
    // Always store current mouse position for render-time hover detection
    app.mouse.row = Some(mouse.row);
    app.mouse.col = Some(mouse.column);
    match mouse.kind {
        MouseEventKind::Moved => {
            app.mouse.moved = true;
        }
        MouseEventKind::ScrollUp => {
            if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                let mut current = app.menu.list_state.selected().unwrap_or(0);
                current = current.saturating_sub(3);
                if app.active_modal == ActiveModal::ModelMenu {
                    while current > 0
                        && matches!(
                            app.model_search.filtered_models.get(current),
                            Some(ModelMenuItem::Header(_))
                        )
                    {
                        current -= 1;
                    }
                } else if app.active_modal == ActiveModal::SettingsMenu {
                    while current > 0
                        && matches!(
                            app.menu.settings_items.get(current),
                            Some(SettingsMenuItem::Header(_))
                        )
                    {
                        current -= 1;
                    }
                }
                app.menu.list_state.select(Some(current));
            } else {
                app.session.history_scroll = app.session.history_scroll.saturating_sub(15);
                app.session.auto_scroll = false;
            }
        }
        MouseEventKind::ScrollDown => {
            if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                let current = app.menu.list_state.selected().unwrap_or(0);
                let max = match app.active_modal {
                    ActiveModal::CommandMenu => app.menu.filtered_commands.len(),
                    ActiveModal::ProviderMenu => PROVIDERS.len(),
                    ActiveModal::SettingsMenu => app.menu.settings_items.len(),
                    ActiveModal::ModelMenu => app.model_search.filtered_models.len(),
                    _ => 0,
                };
                let mut next = current.saturating_add(3).min(max.saturating_sub(1));
                if app.active_modal == ActiveModal::ModelMenu {
                    while next < max - 1
                        && matches!(
                            app.model_search.filtered_models.get(next),
                            Some(ModelMenuItem::Header(_))
                        )
                    {
                        next += 1;
                    }
                } else if app.active_modal == ActiveModal::SettingsMenu {
                    while next < max - 1
                        && matches!(
                            app.menu.settings_items.get(next),
                            Some(SettingsMenuItem::Header(_))
                        )
                    {
                        next += 1;
                    }
                }
                app.menu.list_state.select(Some(next));
            } else {
                app.session.history_scroll = app.session.history_scroll.saturating_add(15);
                if app.session.history_scroll >= app.session.max_scroll {
                    app.session.auto_scroll = true;
                }
            }
        }
        MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left) => {
            if app.active_modal == ActiveModal::UserMessage {
                if let Some(msg_idx) = app.user_msg.msg_idx {
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
                                app.user_msg.msg_idx = None;
                                app.active_modal = ActiveModal::None;
                            }
                        } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                            let click_row = mouse.row;
                            if click_row == modal_y + 2 {
                                app.user_msg.selected = 0;
                                let text = app.session.history[msg_idx]
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
                                app.session.history
                                    .push(Message::system("Message copied to clipboard!".to_string()));
                                app.user_msg.msg_idx = None;
                                app.active_modal = ActiveModal::None;
                            } else if click_row == modal_y + 3 {
                                app.user_msg.selected = 1;
                                let text = app.session.history[msg_idx]
                                    .content
                                    .as_ref()
                                    .map(|s| s.to_string())
                                    .unwrap_or_default();
                                app.session.history.truncate(msg_idx);
                                app.input =
                                    tui_textarea::TextArea::from(text.lines().map(|s| s.to_string()));
                                app.input.move_cursor(tui_textarea::CursorMove::End);
                                app.user_msg.msg_idx = None;
                                app.active_modal = ActiveModal::None;
                            }
                        }
                    }
                }
            } else if app.active_modal == ActiveModal::Update {
                if app.update.pending_version.is_some() {
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
                                app.update.pending_version = None;
                                app.active_modal = ActiveModal::None;
                            }
                        } else if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                            && mouse.row == modal_y + height.saturating_sub(2)
                        {
                            if mouse.column >= modal_x + width.saturating_sub(25)
                                && mouse.column < modal_x + width.saturating_sub(15)
                            {
                                app.update.pending_version = None;
                                app.active_modal = ActiveModal::None;
                            } else if mouse.column >= modal_x + width.saturating_sub(15)
                                && mouse.column < modal_x + width
                            {
                                app.update.install = true;
                            }
                        }
                    }
                }
            } else if app.active_modal == ActiveModal::CommandMenu
                || app.active_modal == ActiveModal::ProviderMenu
                || app.active_modal == ActiveModal::ModelMenu
                || app.active_modal == ActiveModal::SettingsMenu
            {
                if let Ok(size) = terminal.size() {
                    let (width, height) = match app.active_modal {
                        ActiveModal::CommandMenu => {
                            (60, (app.menu.filtered_commands.len() + 6).min(15) as u16)
                        }
                        ActiveModal::ProviderMenu => {
                            (60, (PROVIDERS.len() + 6).min(15) as u16)
                        }
                        ActiveModal::SettingsMenu => {
                            (60, (app.menu.settings_items.len() + 6).min(15) as u16)
                        }
                        _ => {
                            (70, (app.model_search.filtered_models.len() + 7).min(18) as u16)
                        }
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
                        app.active_modal = ActiveModal::None;
                    } else if is_inside_list
                        && matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left))
                        && app.active_modal == ActiveModal::SettingsMenu
                    {
                        let idx = (mouse.row - (modal_y + 2)) as usize + app.menu.list_state.offset();
                        if idx < app.menu.settings_items.len() {
                            if let Some(SettingsMenuItem::Option { key, val: _, .. }) =
                                app.menu.settings_items.get(idx)
                            {
                                let key = key.clone();
                                apply_settings_toggle(app, &key).await;
                            }
                        }
                    }
                }
            } else if app.screen == Screen::Session {
                let has_thinking = app.session.history.iter().any(|m| m.thought.is_some());
                if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                    if let Ok(size) = terminal.size() {
                        if let Some(msg_idx) = compute_message_hover(app, size) {
                            if app.session.history[msg_idx].role == Role::User {
                                app.user_msg.msg_idx = Some(msg_idx);
                                app.user_msg.selected = 0;
                                app.active_modal = ActiveModal::UserMessage;
                                return Ok(());
                            }
                        }
                    }
                }

                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
                    let in_cooldown = app
                        .session.last_toggle_time
                        .is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(400));

                    if !in_cooldown && has_thinking {
                        let is_double_click = if let Some((last_time, col, row)) = app.mouse.last_click_up
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
                            app.session.collapse_thinking = !app.session.collapse_thinking;
                            app.mouse.last_click_up = None;
                            app.mouse.down_start = None;
                            app.session.last_toggle_time = Some(std::time::Instant::now());
                        } else if let Ok(size) = terminal.size() {
                            // Compute hover FRESH with current mouse position
                            let hover = compute_thinking_hover(app, size);
                            if hover {
                                app.mouse.last_click_up =
                                    Some((std::time::Instant::now(), mouse.column, mouse.row));
                                app.mouse.down_start =
                                    Some((std::time::Instant::now(), mouse.column, mouse.row));
                            } else {
                                app.mouse.last_click_up = None;
                            }
                        }
                    }
                }
                if matches!(mouse.kind, MouseEventKind::Up(MouseButton::Left)) {
                    app.mouse.down_start = None;
                    app.session.temp_expand_thinking = false;
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

        app.session.history.push(Message::user("First message".to_string()));
        app.session.history.push(Message::assistant(
            Some("Assistant reply".into()),
            None,
            None,
        ));
        app.session.history
            .push(Message::user("Second message".to_string()));

        app.user_msg.msg_idx = Some(2);
        app.user_msg.selected = 1;
        app.active_modal = ActiveModal::UserMessage;

        let enter_key = event::KeyEvent::new(event::KeyCode::Enter, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();

        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.user_msg.msg_idx, None);
        assert_eq!(app.active_modal, ActiveModal::None);
        assert_eq!(app.session.history.len(), 2);
        assert_eq!(app.session.history[0].role, Role::User);
        assert_eq!(app.session.history[1].role, Role::Assistant);
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

        app.update.pending_version = Some("v1.15.4".to_string());
        app.update.selected = 1;
        app.active_modal = ActiveModal::Update;

        let left_key = event::KeyEvent::new(event::KeyCode::Left, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, left_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.update.selected, 0);

        let enter_key = event::KeyEvent::new(event::KeyCode::Enter, event::KeyModifiers::empty());
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Continue);
        assert_eq!(app.update.pending_version, None);
        assert_eq!(app.active_modal, ActiveModal::None);
        assert!(!app.update.install);

        app.update.pending_version = Some("v1.15.4".to_string());
        app.update.selected = 1;
        app.active_modal = ActiveModal::Update;
        let res = handle_key_event(&mut app, enter_key, false).await.unwrap();
        assert_eq!(res, KeyEventResult::Exit);
        assert!(app.update.install);
    }
}
