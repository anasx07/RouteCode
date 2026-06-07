use routecode_sdk::agents::types::ConfirmationResponse;
use routecode_sdk::agents::StreamChunk;
use routecode_sdk::core::{Message, Role};
use tui_textarea::TextArea;

use super::app::App;
use super::types::{format_error_for_display, parse_qir_status, ApprovalMode, Screen};

pub(crate) async fn handle_stream_chunks(app: &mut App) {
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
                        let mut current = last
                            .content
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        current.push_str(&content);
                        last.content = Some(std::sync::Arc::from(current));
                    } else {
                        app.history.push(Message::assistant(
                            Some(std::sync::Arc::from(content)),
                            None,
                            None,
                        ));
                    }
                } else {
                    app.history.push(Message::assistant(
                        Some(std::sync::Arc::from(content)),
                        None,
                        None,
                    ));
                }
            }
            StreamChunk::Thought { content } => {
                if let Some(last) = app.history.last_mut() {
                    if last.role == Role::Assistant {
                        let mut current = last
                            .thought
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_default();
                        current.push_str(&content);
                        last.thought = Some(std::sync::Arc::from(current));
                    } else {
                        app.history.push(Message::assistant(
                            None,
                            Some(std::sync::Arc::from(content)),
                            None,
                        ));
                    }
                } else {
                    app.history.push(Message::assistant(
                        None,
                        Some(std::sync::Arc::from(content)),
                        None,
                    ));
                }
            }
            StreamChunk::ToolCall { tool_call } => {
                app.active_tool = Some(tool_call.function.name.clone());
                if let Some(last) = app.history.last_mut() {
                    if last.role == Role::Assistant {
                        let mut calls = last.tool_calls.clone().unwrap_or_default();
                        if let Some(idx) = tool_call.index {
                            if let Some(existing) =
                                calls.iter_mut().find(|tc| tc.index == Some(idx))
                            {
                                *existing = tool_call;
                            } else {
                                calls.push(tool_call);
                            }
                        } else if !calls
                            .iter()
                            .any(|tc| tc.id == tool_call.id && !tc.id.is_empty())
                        {
                            calls.push(tool_call);
                        }
                        last.tool_calls = Some(calls);
                    } else {
                        app.history
                            .push(Message::assistant(None, None, Some(vec![tool_call])));
                    }
                } else {
                    app.history
                        .push(Message::assistant(None, None, Some(vec![tool_call])));
                }
            }
            StreamChunk::ToolResult {
                name,
                content,
                tool_call_id,
            } => {
                app.active_tool = None;
                app.history.push(Message::tool(tool_call_id, name, content));
            }
            StreamChunk::Status { content } => {
                if let Some(qir) = parse_qir_status(&content) {
                    app.qir_retry_status = Some(qir);
                }
                app.history
                    .push(Message::system(format!("[QIR] {}", content)));
            }
            StreamChunk::SessionStats {
                total_tokens,
                total_cost,
                qir_attempts,
            } => {
                app.usage.total_tokens = total_tokens;
                app.usage.total_cost = total_cost;
                app.usage.qir_attempts = qir_attempts;
            }
            StreamChunk::Done => {
                app.is_generating = false;
                app.active_tool = None;
                app.qir_retry_status = None;
                if !app.history.is_empty() {
                    let session = routecode_sdk::utils::storage::Session {
                        messages: app.history.clone(),
                        model: app.current_model.clone(),
                        usage: app.orchestrator.usage.lock().await.clone(),
                        timestamp: chrono::Utc::now().timestamp(),
                    };
                    if let Err(e) =
                        routecode_sdk::utils::storage::save_session(&app.session_id, &session)
                    {
                        log::error!("Failed to auto-save session: {}", e);
                    }
                }
            }
            StreamChunk::Error { content } => {
                let display = format_error_for_display(&content);
                app.history
                    .push(Message::system(format!("Error: {}", display)));
                app.is_generating = false;
                app.active_tool = None;
                app.qir_retry_status = None;
            }
            StreamChunk::Models { models } => {
                app.all_available_models.extend(models);
                let search = app
                    .model_search_input
                    .lines()
                    .first()
                    .map(|l| l.trim().to_lowercase())
                    .unwrap_or_default();
                super::logic::handle_model_search(app, &search, false).await;
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
                            if let Err(e) =
                                orchestrator.run(&mut history, &model, Some(tx), None).await
                            {
                                log::error!("Orchestrator run failed: {}", e);
                            }
                        });
                    }
                }
            }
            StreamChunk::FinalHistory { history } => {
                app.history = history;
            }
            StreamChunk::RequestConfirmation {
                message,
                target,
                tx,
            } => match app.approval_mode {
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
            },
            StreamChunk::UpdateAvailable {
                version,
                changelog,
                published_at,
            } => {
                app.pending_update = Some(version);
                app.pending_update_changelog = changelog;
                app.pending_update_published_at = published_at;
                app.update_modal_selected = 1;
            }
            _ => {}
        }
    }
}
