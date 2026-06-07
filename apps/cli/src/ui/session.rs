use crate::ui::components::{
    clean_model_name, COLOR_DIM, COLOR_INPUT_BG, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS,
    COLOR_SYSTEM, COLOR_TEXT,
};
use crate::ui::App;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use routecode_sdk::core::{Message, Role};
use unicode_width::UnicodeWidthStr;

pub fn ui_session(f: &mut Frame, app: &mut App, area: Rect) -> Rect {
    let input_height = (app.input.lines().len() as u16 + 2).min(12);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(area);

    // Compute thinking hover at render time using actual frame dimensions
    let thinking_hovered = crate::ui::compute_thinking_hover(app, f.size());
    app.thinking_hover_rendered = thinking_hovered;
    let is_collapsed = app.collapse_thinking && !app.temp_expand_thinking;

    let hovered_msg_idx = crate::ui::compute_message_hover(app, chunks[0]);

    let needs_rebuild = app.render_dirty
        || app.cached_text.is_none()
        || app.history.len() != app.cached_history_len
        || chunks[0].width != app.cached_width
        || is_collapsed != app.cached_is_collapsed
        || thinking_hovered != app.cached_thinking_hovered
        || hovered_msg_idx != app.cached_hovered_msg_idx;

    let throttle_ok = app.last_cache_update.elapsed() >= std::time::Duration::from_millis(200);

    if needs_rebuild && (throttle_ok || !app.is_generating) {
        app.render_dirty = false;
        app.last_cache_update = std::time::Instant::now();
        let mut lines = Vec::new();
        let mut total_height: usize = 0;
        let mut layout = Vec::new();
        let available_width = chunks[0].width.max(1) as usize;
        let calc_width = (available_width as f32 * 0.95).floor().max(1.0) as usize;

        for (msg_idx, m) in app.history.iter().enumerate() {
            let msg_slice = std::slice::from_ref(m);
            let msg_text = render_history(
                msg_slice,
                is_collapsed,
                thinking_hovered,
                hovered_msg_idx,
                msg_idx,
            );

            for line in msg_text.lines {
                let line_width: usize = line.spans.iter().map(|s| s.content.width()).sum();
                let wrapped_height = if line_width == 0 {
                    1
                } else {
                    (line_width + calc_width - 1) / calc_width.max(1)
                };

                let is_thinking = line.spans.iter().any(|span| {
                    span.content.contains('\u{2502}')
                        || span.content.contains('\u{2503}')
                        || span.content.contains("Thinking...")
                });

                for _ in 0..wrapped_height {
                    layout.push((msg_idx, is_thinking));
                }

                total_height += wrapped_height;
                lines.push(line);
            }
        }

        let history = Text::from(lines);
        app.cached_layout = layout;
        // Safety buffer
        total_height += 2;

        app.cached_history_len = app.history.len();
        app.cached_width = chunks[0].width;
        app.cached_is_collapsed = is_collapsed;
        app.cached_thinking_hovered = thinking_hovered;
        app.cached_hovered_msg_idx = hovered_msg_idx;
        app.cached_total_height = total_height;
        app.cached_text = Some(history);
    }

    let history_text = app.cached_text.as_ref().unwrap().clone();
    let total_height = app.cached_total_height;

    let max_scroll = total_height
        .saturating_sub(chunks[0].height as usize)
        .min(u16::MAX as usize) as u16;
    app.max_scroll = max_scroll;

    if app.auto_scroll {
        app.history_scroll = max_scroll;
    } else {
        // Only re-enable auto-scroll if the user manually scrolls to the bottom of long content
        if app.history_scroll >= max_scroll && max_scroll > 0 {
            app.auto_scroll = true;
            app.history_scroll = max_scroll;
        }
    }

    f.render_widget(
        Paragraph::new(history_text)
            .wrap(Wrap { trim: false })
            .scroll((app.history_scroll, 0)),
        chunks[0],
    );

    f.render_widget(
        Block::default().style(Style::default().bg(COLOR_INPUT_BG)),
        chunks[1],
    );

    let inner_input_area = Rect::new(
        chunks[1].x + 1,
        chunks[1].y + 1,
        chunks[1].width.saturating_sub(2),
        chunks[1].height.saturating_sub(2),
    );
    app.input.set_block(Block::default().borders(Borders::NONE));
    f.render_widget(app.input.widget(), inner_input_area);

    f.set_cursor(
        inner_input_area.x + app.input.cursor().1 as u16,
        inner_input_area.y + app.input.cursor().0 as u16,
    );

    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];

    let generating_text = if app.is_generating {
        if let Some(tool) = &app.active_tool {
            format!(" {} [Running {}...] ", frame, tool)
        } else {
            format!(" {} [Thinking...] ", frame)
        }
    } else {
        "".to_string()
    };

    let cleaned_model = clean_model_name(&app.current_model, &app.current_provider_id);

    let config_thinking = crate::ui::try_lock_config(app)
        .map(|c| c.thinking_level.clone())
        .unwrap_or("default".to_string());

    let thinking_tag = if config_thinking != "default" {
        format!(" • [{}] ", config_thinking)
    } else {
        "".to_string()
    };

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(app.provider_name.len() as u16 + 2),
        ])
        .split(chunks[2]);

    let mut left_spans: Vec<Span> = Vec::new();
    if !app.hide_model_info {
        left_spans.push(Span::styled(
            format!(" {} ", cleaned_model),
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        left_spans.push(Span::styled(
            thinking_tag,
            Style::default()
                .fg(COLOR_SYSTEM)
                .add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(qir) = app.qir_retry_status {
        let (color, label) = if qir.is_recovered() {
            (COLOR_SUCCESS, format!(" ∞ {} ", qir.label()))
        } else {
            (COLOR_SYSTEM, format!(" ∞ {} ", qir.label()))
        };
        left_spans.push(Span::styled(
            label,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    if !app.hide_context_summary {
        let mut summary = format!(
            " • Tokens: {} • Cost: ${:.4}",
            app.usage.total_tokens, app.usage.total_cost
        );
        if app.usage.qir_attempts > 0 {
            summary.push_str(&format!(" • Retries: {}", app.usage.qir_attempts));
        }
        left_spans.push(Span::styled(summary, Style::default().fg(COLOR_SECONDARY)));
        left_spans.push(Span::styled(
            format!(" • Scroll: {}/{} ", app.history_scroll, app.max_scroll),
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::DIM),
        ));
    }
    left_spans.push(Span::styled(
        generating_text,
        Style::default().fg(COLOR_SYSTEM),
    ));
    if !app.hide_context_summary {
        left_spans.push(Span::styled(
            " • ctrl+o toggle thinking • ctrl+p help ",
            Style::default()
                .fg(COLOR_SECONDARY)
                .add_modifier(Modifier::DIM),
        ));
    }
    let left_status = Line::from(left_spans);

    let right_status = Paragraph::new(Span::styled(
        format!(" {} ", app.provider_name),
        Style::default()
            .fg(COLOR_SECONDARY)
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(ratatui::layout::Alignment::Right);

    f.render_widget(Paragraph::new(left_status), status_layout[0]);
    f.render_widget(right_status, status_layout[1]);

    chunks[1]
}

pub fn render_history(
    history: &[Message],
    collapse_thinking: bool,
    thinking_hovered: bool,
    hovered_msg_idx: Option<usize>,
    history_offset: usize,
) -> Text<'static> {
    let mut lines = Vec::new();
    for (idx, m) in history.iter().enumerate() {
        let original_idx = idx + history_offset;
        match m.role {
            Role::User => {
                let is_hovered = Some(original_idx) == hovered_msg_idx;
                let user_style = if is_hovered {
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default()
                        .fg(COLOR_PRIMARY)
                        .add_modifier(Modifier::BOLD)
                };
                let tag = if is_hovered {
                    " ● User (Click to edit/copy)"
                } else {
                    " ● User"
                };
                lines.push(Line::from(vec![Span::styled(tag, user_style)]));
                if let Some(content) = &m.content {
                    for line in content.lines() {
                        let text_style = if is_hovered {
                            Style::default()
                                .fg(COLOR_TEXT)
                                .add_modifier(Modifier::UNDERLINED)
                        } else {
                            Style::default().fg(COLOR_TEXT)
                        };
                        lines.push(Line::from(vec![
                            Span::raw("   "),
                            Span::styled(line.to_string(), text_style),
                        ]));
                    }
                }
            }
            Role::Assistant => {
                lines.push(Line::from(vec![Span::styled(
                    " ● RouteCode",
                    Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
                )]));

                if let Some(thought) = &m.thought {
                    if collapse_thinking {
                        if thinking_hovered {
                            lines.push(Line::from(vec![
                                Span::styled("   ┃ ", Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
                                Span::styled("▶ Thinking... (Double click to expand, one hold click to see thought)", Style::default().fg(COLOR_PRIMARY).add_modifier(Modifier::BOLD)),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled("   │ ", Style::default().fg(COLOR_DIM)),
                                Span::styled(
                                    "▶ Thinking... (ctrl+o to expand)",
                                    Style::default()
                                        .fg(COLOR_SECONDARY)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    } else {
                        if thinking_hovered {
                            lines.push(Line::from(vec![
                                Span::styled(
                                    "   ┃ ",
                                    Style::default()
                                        .fg(COLOR_PRIMARY)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    "▼ Thinking... (Double click to collapse)",
                                    Style::default()
                                        .fg(COLOR_PRIMARY)
                                        .add_modifier(Modifier::BOLD),
                                ),
                            ]));
                        } else {
                            lines.push(Line::from(vec![
                                Span::styled("   │ ", Style::default().fg(COLOR_DIM)),
                                Span::styled(
                                    "▼ Thinking... (ctrl+o to collapse)",
                                    Style::default()
                                        .fg(COLOR_SECONDARY)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }

                        let guide = if thinking_hovered {
                            Span::styled(
                                "   ┃ ",
                                Style::default()
                                    .fg(COLOR_PRIMARY)
                                    .add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Span::styled("   │ ", Style::default().fg(COLOR_DIM))
                        };

                        for line in thought.lines() {
                            let text = if thinking_hovered {
                                Span::styled(
                                    line.to_string(),
                                    Style::default()
                                        .fg(COLOR_PRIMARY)
                                        .add_modifier(Modifier::ITALIC),
                                )
                            } else {
                                Span::styled(
                                    line.to_string(),
                                    Style::default()
                                        .fg(COLOR_SECONDARY)
                                        .add_modifier(Modifier::ITALIC),
                                )
                            };
                            lines.push(Line::from(vec![guide.clone(), text]));
                        }
                    }
                }

                if let Some(tool_calls) = &m.tool_calls {
                    for tc in tool_calls {
                        let args: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(serde_json::json!({}));
                        let arg_preview = if let Some(path) = args["path"].as_str() {
                            format!("({})", path)
                        } else {
                            format!("({})", tc.function.name)
                        };

                        lines.push(Line::from(vec![
                            Span::styled("   🛠 ", Style::default().fg(COLOR_PRIMARY)),
                            Span::styled(
                                format!("Using {} ", tc.function.name),
                                Style::default().fg(COLOR_TEXT),
                            ),
                            Span::styled(
                                arg_preview,
                                Style::default()
                                    .fg(COLOR_SECONDARY)
                                    .add_modifier(Modifier::DIM),
                            ),
                        ]));
                    }
                }

                if let Some(content) = &m.content {
                    let mut in_code_block = false;
                    let raw_lines: Vec<&str> = content.lines().collect();
                    let mut i = 0;
                    while i < raw_lines.len() {
                        let line = raw_lines[i];

                        let mut table_start = None;
                        if !in_code_block && line.contains('|') {
                            if i + 1 < raw_lines.len() && is_delimiter_row(raw_lines[i + 1]) {
                                table_start = Some((i + 1, vec![line]));
                            } else if i + 2 < raw_lines.len()
                                && raw_lines[i + 1].contains('|')
                                && is_delimiter_row(raw_lines[i + 2])
                            {
                                table_start = Some((i + 2, vec![line, raw_lines[i + 1]]));
                            }
                        }

                        if let Some((delimiter_idx, header_lines)) = table_start {
                            let parsed_headers: Vec<Vec<String>> =
                                header_lines.iter().map(|l| parse_table_row(l)).collect();
                            let num_cols =
                                parsed_headers.iter().map(|h| h.len()).max().unwrap_or(0);
                            let mut header_row = vec![String::new(); num_cols];
                            for h in &parsed_headers {
                                for col_idx in 0..num_cols {
                                    if col_idx < h.len() {
                                        let cell = h[col_idx].trim();
                                        if !cell.is_empty() {
                                            if !header_row[col_idx].is_empty() {
                                                header_row[col_idx].push_str(" / ");
                                            }
                                            header_row[col_idx].push_str(cell);
                                        }
                                    }
                                }
                            }

                            let delimiter_row = parse_table_row(raw_lines[delimiter_idx]);
                            let mut alignments = Vec::new();
                            for cell in delimiter_row {
                                alignments.push(parse_alignment(&cell));
                            }

                            while alignments.len() < num_cols {
                                alignments.push(TableAlignment::Left);
                            }

                            let mut rows = Vec::new();
                            let mut j = delimiter_idx + 1;
                            while j < raw_lines.len()
                                && raw_lines[j].contains('|')
                                && !is_delimiter_row(raw_lines[j])
                            {
                                rows.push(parse_table_row(raw_lines[j]));
                                j += 1;
                            }

                            render_table_spans(&header_row, &alignments, &rows, &mut lines);
                            i = j;
                        } else {
                            if line.trim().starts_with("```") {
                                in_code_block = !in_code_block;
                                lines.push(Line::from(vec![
                                    Span::raw("   "),
                                    Span::styled(
                                        line.to_string(),
                                        Style::default()
                                            .fg(COLOR_PRIMARY)
                                            .add_modifier(Modifier::BOLD),
                                    ),
                                ]));
                            } else {
                                let mut line_spans = vec![Span::raw("   ")];
                                line_spans.extend(parse_markdown_line(line, in_code_block));
                                lines.push(Line::from(line_spans));
                            }
                            i += 1;
                        }
                    }
                }
            }
            Role::Tool => {
                lines.push(Line::from(vec![Span::styled(
                    format!("   ✓ Tool ({})", m.name.as_deref().unwrap_or("result")),
                    Style::default().fg(COLOR_SECONDARY),
                )]));
                if let Some(content) = &m.content {
                    if let Ok(res) =
                        serde_json::from_str::<routecode_sdk::core::ToolResult>(content)
                    {
                        if let Some(diff) = res.diff {
                            for line in diff.lines() {
                                let style = if line.starts_with('+') {
                                    Style::default().fg(COLOR_SUCCESS)
                                } else if line.starts_with('-') {
                                    Style::default().fg(Color::Red)
                                } else {
                                    Style::default().fg(COLOR_DIM)
                                };
                                lines.push(Line::from(vec![
                                    Span::raw("     "),
                                    Span::styled(line.to_string(), style),
                                ]));
                            }
                        } else if let Some(out) = res.content {
                            let preview = if out.len() > 100 {
                                let end = out
                                    .char_indices()
                                    .map(|(i, _)| i)
                                    .take_while(|&i| i <= 100)
                                    .last()
                                    .unwrap_or(0);
                                format!("{}...", &out[..end])
                            } else {
                                out
                            };
                            lines.push(Line::from(vec![Span::styled(
                                format!("     {}", preview),
                                Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM),
                            )]));
                        } else if let Some(err) = res.error {
                            lines.push(Line::from(vec![Span::styled(
                                format!("     Error: {}", err),
                                Style::default().fg(Color::Red),
                            )]));
                        }
                    } else {
                        let preview = if content.len() > 100 {
                            let end = content
                                .char_indices()
                                .map(|(i, _)| i)
                                .take_while(|&i| i <= 100)
                                .last()
                                .unwrap_or(0);
                            format!("{}...", &content[..end])
                        } else {
                            content.to_string()
                        };
                        lines.push(Line::from(vec![Span::styled(
                            format!("     {}", preview),
                            Style::default().fg(COLOR_DIM).add_modifier(Modifier::DIM),
                        )]));
                    }
                }
            }
            Role::System => {
                lines.push(Line::from(vec![Span::styled(
                    " ● System",
                    Style::default()
                        .fg(COLOR_SYSTEM)
                        .add_modifier(Modifier::DIM),
                )]));
                if let Some(content) = &m.content {
                    lines.push(Line::from(vec![Span::styled(
                        format!("   {}", content),
                        Style::default()
                            .fg(COLOR_SYSTEM)
                            .add_modifier(Modifier::DIM),
                    )]));
                }
            }
        }
        lines.push(Line::from(""));
    }
    Text::from(lines)
}

fn parse_markdown_line(line: &str, in_code_block: bool) -> Vec<Span<'static>> {
    if in_code_block {
        return vec![Span::styled(
            line.to_string(),
            Style::default().fg(COLOR_SECONDARY),
        )];
    }

    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if trimmed.starts_with("# ")
        || trimmed.starts_with("## ")
        || trimmed.starts_with("### ")
        || trimmed.starts_with("#### ")
    {
        let mut spans = vec![Span::raw(indent.to_string())];
        spans.push(Span::styled(
            trimmed.to_string(),
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        return spans;
    } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        let mut spans = vec![
            Span::raw(indent.to_string()),
            Span::styled(
                trimmed[..2].to_string(),
                Style::default()
                    .fg(COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        spans.extend(parse_inline_markdown(&trimmed[2..]));
        return spans;
    }

    let mut spans = vec![Span::raw(indent.to_string())];
    spans.extend(parse_inline_markdown(trimmed));
    spans
}

fn parse_inline_markdown(text: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    let mut bold = false;
    let mut italic = false;
    let mut code = false;

    while let Some(c) = chars.next() {
        if c == '`' {
            if !current.is_empty() {
                let mut style = Style::default();
                if code {
                    style = style.fg(COLOR_PRIMARY);
                } else {
                    if bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if italic {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                }
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            code = !code;
            continue;
        }

        if !code && c == '*' {
            if let Some(&next_c) = chars.peek() {
                if next_c == '*' {
                    chars.next(); // Consume second '*'
                    if !current.is_empty() {
                        let mut style = Style::default();
                        if bold {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if italic {
                            style = style.add_modifier(Modifier::ITALIC);
                        }
                        spans.push(Span::styled(current.clone(), style));
                        current.clear();
                    }
                    bold = !bold;
                    continue;
                }
            }
            if !current.is_empty() {
                let mut style = Style::default();
                if bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                if italic {
                    style = style.add_modifier(Modifier::ITALIC);
                }
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }
            italic = !italic;
            continue;
        }

        current.push(c);
    }

    if !current.is_empty() {
        let mut style = Style::default();
        if code {
            style = style.fg(COLOR_PRIMARY);
        } else {
            if bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
        }
        spans.push(Span::styled(current, style));
    }

    spans
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum TableAlignment {
    Left,
    Center,
    Right,
}

fn is_delimiter_row(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|c| c == '-' || c == ':' || c == '|' || c.is_whitespace())
}

fn parse_table_row(line: &str) -> Vec<String> {
    let trimmed_line = line.trim();
    let mut cells: Vec<String> = trimmed_line
        .split('|')
        .map(|s| s.trim().to_string())
        .collect();

    if trimmed_line.starts_with('|') && !cells.is_empty() {
        cells.remove(0);
    }
    if trimmed_line.ends_with('|') && !cells.is_empty() && cells.last().unwrap().is_empty() {
        cells.pop();
    }
    cells
}

fn parse_alignment(cell: &str) -> TableAlignment {
    let trimmed = cell.trim();
    let left = trimmed.starts_with(':');
    let right = trimmed.ends_with(':');
    if left && right {
        TableAlignment::Center
    } else if right {
        TableAlignment::Right
    } else {
        TableAlignment::Left
    }
}

fn pad_cell(text: &str, width: usize, alignment: TableAlignment) -> String {
    let text_len = text.chars().count();
    if text_len >= width {
        return text.to_string();
    }

    let total_pad = width - text_len;
    match alignment {
        TableAlignment::Left => {
            let left_pad = 1;
            let right_pad = total_pad - 1;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        TableAlignment::Right => {
            let left_pad = total_pad - 1;
            let right_pad = 1;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        TableAlignment::Center => {
            let left_pad = total_pad / 2;
            let right_pad = total_pad - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
    }
}

fn render_table_spans(
    header_row: &[String],
    alignments: &[TableAlignment],
    rows: &[Vec<String>],
    lines: &mut Vec<Line<'static>>,
) {
    let num_cols = header_row.len();
    if num_cols == 0 {
        return;
    }

    let mut col_widths = vec![0; num_cols];
    for (col_idx, cell) in header_row.iter().enumerate() {
        col_widths[col_idx] = col_widths[col_idx].max(cell.chars().count());
    }
    for row in rows {
        for col_idx in 0..num_cols {
            if col_idx < row.len() {
                col_widths[col_idx] = col_widths[col_idx].max(row[col_idx].chars().count());
            }
        }
    }

    for w in col_widths.iter_mut() {
        *w += 2;
    }

    let mut top_line = String::from("   ┌");
    for (idx, &w) in col_widths.iter().enumerate() {
        top_line.push_str(&"─".repeat(w));
        if idx + 1 < num_cols {
            top_line.push('┬');
        } else {
            top_line.push('┐');
        }
    }
    lines.push(Line::from(vec![Span::styled(
        top_line,
        Style::default().fg(COLOR_DIM),
    )]));

    let mut header_spans = vec![Span::styled("   │", Style::default().fg(COLOR_DIM))];
    for (idx, cell) in header_row.iter().enumerate() {
        let width = col_widths[idx];
        let padded = pad_cell(cell, width, alignments[idx]);
        header_spans.push(Span::styled(
            padded,
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
        header_spans.push(Span::styled("│", Style::default().fg(COLOR_DIM)));
    }
    lines.push(Line::from(header_spans));

    let mut mid_line = String::from("   ├");
    for (idx, &w) in col_widths.iter().enumerate() {
        mid_line.push_str(&"─".repeat(w));
        if idx + 1 < num_cols {
            mid_line.push('┼');
        } else {
            mid_line.push('┤');
        }
    }
    lines.push(Line::from(vec![Span::styled(
        mid_line,
        Style::default().fg(COLOR_DIM),
    )]));

    for row in rows {
        let mut row_spans = vec![Span::styled("   │", Style::default().fg(COLOR_DIM))];
        for col_idx in 0..num_cols {
            let width = col_widths[col_idx];
            let cell_text = if col_idx < row.len() {
                &row[col_idx]
            } else {
                ""
            };
            let padded = pad_cell(cell_text, width, alignments[col_idx]);

            let cell_style = if cell_text.contains("Complete")
                || cell_text.contains("Active")
                || cell_text.contains("[x]")
                || cell_text.contains("✅")
            {
                Style::default()
                    .fg(COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD)
            } else if cell_text.contains("Pending") || cell_text.contains("Waiting") {
                Style::default()
                    .fg(COLOR_SYSTEM)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_TEXT)
            };

            row_spans.push(Span::styled(padded, cell_style));
            row_spans.push(Span::styled("│", Style::default().fg(COLOR_DIM)));
        }
        lines.push(Line::from(row_spans));
    }

    let mut bot_line = String::from("   └");
    for (idx, &w) in col_widths.iter().enumerate() {
        bot_line.push_str(&"─".repeat(w));
        if idx + 1 < num_cols {
            bot_line.push('┴');
        } else {
            bot_line.push('┘');
        }
    }
    lines.push(Line::from(vec![Span::styled(
        bot_line,
        Style::default().fg(COLOR_DIM),
    )]));
}
