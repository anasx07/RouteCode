use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
    Frame, Terminal,
};
use std::io;

use super::app::App;
use super::components::{COLOR_BG, COLOR_DIM, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_TEXT};
use super::events::{handle_key_event, handle_mouse_event, KeyEventResult};
use super::menus::{
    render_api_key_dialog, render_menu, render_model_menu, render_provider_menu,
    render_settings_menu,
};
use super::streaming::handle_stream_chunks;
use super::types::{ApprovalMode, Screen};
use super::welcome::ui_welcome;

pub async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<bool> {
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(100);
    let render_rate = std::time::Duration::from_millis(16); // ~60 FPS for smooth rendering
    let mut needs_draw = true;

    loop {
        if needs_draw || app.render_dirty {
            terminal.draw(|f| ui(f, &mut app))?;
            needs_draw = false;
        }

        let time_to_next_tick = tick_rate.saturating_sub(last_tick.elapsed());
        let timeout = if app.is_generating || app.logo_anim_frames > 0 {
            render_rate.min(time_to_next_tick)
        } else {
            time_to_next_tick
        };

        if event::poll(timeout)? {
            let mut events = Vec::new();
            while event::poll(std::time::Duration::from_millis(0))? {
                events.push(event::read()?);
            }

            if !events.is_empty() {
                needs_draw = true;
            }

            let is_burst = events
                .iter()
                .filter(|e| match e {
                    Event::Key(key) => key.kind == KeyEventKind::Press,
                    _ => false,
                })
                .count()
                > 1;

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
                    Event::Paste(text) => {
                        app.input.insert_str(&text);
                    }
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
                        && app.thinking_hover_rendered
                    {
                        app.temp_expand_thinking = true;
                    }
                }
            }

            needs_draw = true;
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
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);
    let current_dir = std::env::current_dir()
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .unwrap_or_else(|_| "workspace".to_string());

    let mode_label = app.approval_mode.label();
    let mode_style = match app.approval_mode {
        ApprovalMode::Normal => Style::default().fg(COLOR_SECONDARY),
        ApprovalMode::Plan => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        ApprovalMode::YOLO => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ApprovalMode::Shell => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    };
    let mode_indicator = format!("[{}]", mode_label);

    let mut header_left = Vec::new();
    if app.approval_mode != ApprovalMode::Normal {
        header_left.push(Span::styled(mode_indicator, mode_style));
        header_left.push(Span::raw(" "));
    }
    if !app.hide_cwd {
        header_left.push(Span::styled(
            format!("{} ", current_dir),
            Style::default().fg(COLOR_SECONDARY),
        ));
    }

    let header_right_len = if app.hide_model_info { 0u16 } else { 25u16 };
    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(header_right_len)])
        .split(main_layout[0]);
    f.render_widget(Paragraph::new(Line::from(header_left)), header_layout[0]);

    if !app.hide_model_info {
        let version = env!("CARGO_PKG_VERSION");
        let header_title = format!(" RouteCode v{} ", version);
        f.render_widget(
            Paragraph::new(Span::styled(
                header_title,
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(ratatui::layout::Alignment::Right),
            header_layout[1],
        );
    }
    let input_area = match app.screen {
        Screen::Welcome => ui_welcome(f, app, main_layout[1]),
        Screen::Session => super::session::ui_session(f, app, main_layout[1]),
    };
    if app.show_menu {
        render_menu(f, app, input_area);
    } else if app.show_provider_menu {
        render_provider_menu(f, app, input_area);
    } else if app.show_model_menu {
        render_model_menu(f, app, input_area);
    } else if app.show_settings_menu {
        render_settings_menu(f, app, input_area);
    } else if app.is_inputting_api_key {
        render_api_key_dialog(f, app);
    } else if app.pending_clear {
        render_confirmation_dialog(f, "Are you sure you want to clear all history? (y/n)");
    } else if app.pending_exit {
        render_confirmation_dialog(f, "Are you sure you want to exit RouteCode? (y/n)");
    } else if app.pending_command_confirmation.is_some() {
        render_command_confirmation_dialog(f, app);
    } else if app.pending_plan_approval.is_some() {
        render_plan_approval_dialog(f, app);
    } else if app.pending_hook_trust.is_some() {
        render_hook_trust_dialog(f, app);
    } else if app.show_user_msg_modal.is_some() {
        render_user_msg_modal(f, app);
    } else if app.pending_update.is_some() {
        render_update_modal(f, app);
    }
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
        Line::from(vec![Span::styled(message, Style::default().fg(COLOR_TEXT))]),
        Line::from(vec![Span::styled(
            format!("> {}", target),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    if app.inputting_command_feedback {
        lines.push(Line::from(vec![Span::styled(
            "Please type your feedback below and press Enter (Esc to cancel):",
            Style::default().fg(COLOR_SECONDARY),
        )]));
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "[Y]",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Allow once  "),
            Span::styled(
                "[S]",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Allow for session  "),
            Span::styled(
                "[W]",
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Allow for Workspace  "),
            Span::styled(
                "[F]",
                Style::default()
                    .fg(COLOR_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Tell Agent something else  "),
            Span::styled(
                "[D] or [Esc]",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Deny"),
        ]));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, inner_area);
    f.render_widget(paragraph, inner_area);

    if app.inputting_command_feedback {
        let input_rect = ratatui::layout::Rect {
            x: inner_area.x + 2,
            y: inner_area.y + 5,
            width: inner_area.width.saturating_sub(4),
            height: 3,
        };
        let input_block = Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(COLOR_PRIMARY));
        app.input.set_block(input_block);
        f.render_widget(app.input.widget(), input_rect);
        f.set_cursor(
            input_rect.x + app.input.cursor().1 as u16 + 1,
            input_rect.y + app.input.cursor().0 as u16 + 1,
        );
    }
}

fn render_plan_approval_dialog(f: &mut Frame, app: &mut App) {
    use ratatui::text::Text;
    let area = f.size();

    // 80% height, 80% width centered
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(area);

    let popup_horiz = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(popup_layout[1]);

    let inner = popup_horiz[1];

    // Split inner into plan body (top) + action row (bottom)
    let body_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // plan markdown
            Constraint::Length(5), // action row
        ])
        .split(inner);

    let (plan, plan_path, allowed_prompts, _sender) =
        app.pending_plan_approval.as_ref().unwrap().clone();

    let block = Block::default()
        .title(" Plan Approval Required ")
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .style(Style::default().bg(COLOR_BG));

    let mut body_lines: Vec<Line> = Vec::new();
    body_lines.push(Line::from(vec![
        Span::styled(
            "File: ",
            Style::default().fg(COLOR_SECONDARY),
        ),
        Span::styled(
            plan_path,
            Style::default().fg(Color::Cyan),
        ),
    ]));
    body_lines.push(Line::from(""));
    // Plan body — render as plain text wrapped. No markdown parsing in
    // v1; the user can read the plan in their editor via the file path.
    let plan_text = Text::from(plan.clone());
    for line in plan_text.lines {
        body_lines.push(line);
    }
    if !allowed_prompts.is_empty() {
        body_lines.push(Line::from(""));
        body_lines.push(Line::from(vec![Span::styled(
            "Requested permissions:",
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::BOLD),
        )]));
        for (tool, prompt) in &allowed_prompts {
            body_lines.push(Line::from(format!("  - [{}] {}", tool, prompt)));
        }
    }

    let body = Paragraph::new(body_lines)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((app.history_scroll, 0));
    f.render_widget(ratatui::widgets::Clear, body_layout[0]);
    f.render_widget(body, body_layout[0]);

    // Action row: 4 buttons
    let actions_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_DIM))
        .style(Style::default().bg(COLOR_BG));

    let buttons: [(&str, &str, Color); 4] = [
        ("[A]", "Approve & Unlock", Color::Green),
        ("[O]", "Approve Once", COLOR_PRIMARY),
        ("[F]", "Send Feedback", COLOR_SECONDARY),
        ("[D]", "Deny", Color::Red),
    ];
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label, color)) in buttons.iter().enumerate() {
        let style = if i == app.plan_approval_selected {
            Style::default().fg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(*color)
        };
        spans.push(Span::styled(*key, style));
        spans.push(Span::raw(format!(" {}   ", label)));
    }
    let action = Paragraph::new(vec![Line::from(""), Line::from(spans)])
        .block(actions_block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, body_layout[1]);
    f.render_widget(action, body_layout[1]);

    if app.inputting_plan_feedback {
        // Reuse the input box below the action row by overlaying it
        let input_rect = ratatui::layout::Rect {
            x: body_layout[1].x + 2,
            y: body_layout[1].y + 2,
            width: body_layout[1].width.saturating_sub(4),
            height: 3,
        };
        let input_block = Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .border_style(Style::default().fg(COLOR_PRIMARY));
        app.input.set_block(input_block);
        f.render_widget(app.input.widget(), input_rect);
        f.set_cursor(
            input_rect.x + app.input.cursor().1 as u16 + 1,
            input_rect.y + app.input.cursor().0 as u16 + 1,
        );
    }
}

fn render_hook_trust_dialog(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let (signature, project_path, hooks) = {
        let t = app.pending_hook_trust.as_ref();
        match t {
            Some(t) => (t.signature.clone(), t.project_path.clone(), t.hooks.clone()),
            None => (String::new(), String::new(), Vec::new()),
        }
    };
    let _ = signature;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Min(10),
            Constraint::Percentage(20),
        ])
        .split(area);
    let body = popup_layout[1];
    let body_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(body);

    let title_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY))
        .title(Span::styled(
            " Trust project hooks? ",
            Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD),
        ));
    let mut header_lines = vec![Line::from(Span::styled(
        format!("Project: {}", project_path),
        Style::default().fg(COLOR_PRIMARY),
    ))];
    header_lines.push(Line::from(Span::styled(
        format!("This project wants to register {} hook(s):", hooks.len()),
        Style::default().fg(COLOR_PRIMARY),
    )));
    let header =
        Paragraph::new(header_lines).block(title_block).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, body_layout[0]);
    f.render_widget(header, body_layout[0]);

    let list_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY));
    let list_lines: Vec<Line> = hooks
        .iter()
        .take(20)
        .map(|h| {
            Line::from(vec![
                Span::styled(
                    format!("  {} ", h.event),
                    Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("({}) ", h.matcher),
                    Style::default().fg(COLOR_DIM),
                ),
                Span::styled(h.description.clone(), Style::default().fg(COLOR_TEXT)),
            ])
        })
        .collect();
    let list = Paragraph::new(list_lines)
        .block(list_block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, body_layout[1]);
    f.render_widget(list, body_layout[1]);

    let actions_block = Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_style(Style::default().fg(COLOR_PRIMARY));
    let buttons: Vec<(&str, &str, Color)> = vec![
        ("T", "Trust", COLOR_PRIMARY),
        ("D", "Deny", COLOR_SECONDARY),
    ];
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, label, color)) in buttons.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        let style = if i == app.hook_trust_selected {
            Style::default().fg(*color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(*color)
        };
        spans.push(Span::styled(*key, style));
        spans.push(Span::raw(format!(" {}   ", label)));
    }
    let action = Paragraph::new(vec![Line::from(""), Line::from(spans)])
        .block(actions_block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(ratatui::widgets::Clear, body_layout[2]);
    f.render_widget(action, body_layout[2]);
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

    let p = Paragraph::new(Span::styled(
        message,
        Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
    ))
    .alignment(ratatui::layout::Alignment::Center)
    .block(block);

    f.render_widget(ratatui::widgets::Clear, popup_horiz[1]);
    f.render_widget(p, popup_horiz[1]);
}

pub(crate) fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    let mut clipboard = arboard::Clipboard::new()
        .map_err(std::io::Error::other)?;
    clipboard.set_text(text.to_string())
        .map_err(std::io::Error::other)?;
    Ok(())
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
        Line::from(vec![Span::styled(
            " Choose an action:",
            Style::default().fg(COLOR_SECONDARY),
        )]),
        Line::from(""),
    ];

    for (idx, opt) in options.iter().enumerate() {
        let is_selected = idx == app.user_msg_modal_selected;
        let prefix = if is_selected { " -> " } else { "   " };
        let style = if is_selected {
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT)
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(COLOR_PRIMARY)),
            Span::styled(opt.to_string(), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        " Press Enter/Click to select, Esc to close",
        Style::default().fg(COLOR_DIM),
    )]));

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
            Constraint::Percentage(
                ((area.height as f32 * 0.3) as u16)
                    .min(area.height.saturating_sub(modal_height + 2)),
            ),
            Constraint::Length(modal_height),
            Constraint::Percentage(
                ((area.height as f32 * 0.3) as u16)
                    .min(area.height.saturating_sub(modal_height + 2)),
            ),
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
        .title(
            ratatui::widgets::block::Title::from(Span::styled(
                " Update Available ",
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ))
            .alignment(ratatui::layout::Alignment::Left),
        )
        .title(
            ratatui::widgets::block::Title::from(Span::styled(
                " esc ",
                Style::default().fg(COLOR_DIM),
            ))
            .alignment(ratatui::layout::Alignment::Right),
        )
        .style(Style::default().bg(COLOR_BG));

    f.render_widget(block, inner_area);

    let content_area = ratatui::layout::Rect {
        x: inner_area.x + 2,
        y: inner_area.y + 1,
        width: inner_area.width.saturating_sub(4),
        height: inner_area.height.saturating_sub(4),
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(
            format!(
                "Version {} is available (current: {})",
                version,
                env!("CARGO_PKG_VERSION")
            ),
            Style::default().fg(COLOR_TEXT),
        )]),
        Line::from(""),
    ];

    if !app.pending_update_changelog.is_empty() {
        let changelog_lines: Vec<&str> = app.pending_update_changelog.lines().take(5).collect();
        for line in changelog_lines {
            let trimmed = if line.len() > 60 {
                format!("{}...", &line[..57])
            } else {
                line.to_string()
            };
            lines.push(Line::from(vec![Span::styled(
                trimmed.to_string(),
                Style::default().fg(COLOR_SECONDARY),
            )]));
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

    let skip_style = if app.update_modal_selected == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(COLOR_TEXT)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_DIM)
    };
    let confirm_style = if app.update_modal_selected == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Rgb(255, 179, 138))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(255, 179, 138))
    };

    let buttons = Line::from(vec![
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
pub fn try_lock_config(
    app: &App,
) -> Option<tokio::sync::MutexGuard<'_, routecode_sdk::core::Config>> {
    for _ in 0..10 {
        match app.orchestrator.config.try_lock() {
            Ok(guard) => return Some(guard),
            Err(_) => std::thread::sleep(std::time::Duration::from_micros(200)),
        }
    }
    None
}
