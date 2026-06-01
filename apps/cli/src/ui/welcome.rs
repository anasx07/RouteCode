use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use crate::ui::App;
use crate::ui::components::{COLOR_INPUT_BG, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_TEXT, clean_model_name};

pub fn ui_welcome(f: &mut Frame, app: &mut App, area: Rect) -> Rect {
    let logo_height = if area.height < 20 { 0 } else { 6 };
    let spacer_height = if area.height < 15 { 0 } else { area.height / 3 };
    let input_lines = app.input.lines().len() as u16;
    let input_height = (input_lines + 2).min(12);
    
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(spacer_height),
            ratatui::layout::Constraint::Length(logo_height),
            ratatui::layout::Constraint::Length(input_height),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1)
        ])
        .split(area);

    if logo_height > 0 {
        let config_guard = crate::ui::try_lock_config(app);
        let (animation_mode, animation_color) = if let Some(ref config) = config_guard {
            (config.logo_animation.clone(), config.logo_animation_color.clone())
        } else {
            ("always".to_string(), "rainbow".to_string())
        };

        let colors = match animation_color.as_str() {
            "neon" => vec![
                ratatui::style::Color::Rgb(0, 255, 127),
                ratatui::style::Color::Rgb(0, 255, 255),
                ratatui::style::Color::Rgb(57, 255, 20),
                ratatui::style::Color::Rgb(0, 191, 255),
            ],
            "cyberpunk" => vec![
                ratatui::style::Color::Rgb(255, 0, 127),
                ratatui::style::Color::Rgb(255, 0, 255),
                ratatui::style::Color::Rgb(138, 43, 226),
                ratatui::style::Color::Rgb(0, 255, 255),
            ],
            "sunset" => vec![
                ratatui::style::Color::Rgb(255, 69, 0),
                ratatui::style::Color::Rgb(255, 140, 0),
                ratatui::style::Color::Rgb(255, 215, 0),
                ratatui::style::Color::Rgb(255, 105, 180),
            ],
            "mono" => vec![
                ratatui::style::Color::Rgb(240, 240, 240),
                ratatui::style::Color::Rgb(180, 180, 180),
                ratatui::style::Color::Rgb(120, 120, 120),
                ratatui::style::Color::Rgb(80, 80, 80),
            ],
            _ => vec![
                ratatui::style::Color::Rgb(255, 50, 50),
                ratatui::style::Color::Rgb(255, 150, 50),
                ratatui::style::Color::Rgb(255, 255, 50),
                ratatui::style::Color::Rgb(50, 255, 50),
                ratatui::style::Color::Rgb(50, 150, 255),
                ratatui::style::Color::Rgb(150, 50, 255),
                ratatui::style::Color::Rgb(255, 50, 150),
            ],
        };

        let small_logo = [
            "  __          _   ",
            " |__) _|_ _ _/  _  _| _ ",
            " |  \\(_|(_(- \\__(_)(_|(/_ "
        ];
        
        let large_logo = [
            "  ____             _        ____          _      ",
            " |  _ \\ ___  _   _| |_ ___ / ___|___   __| | ___ ",
            " | |_) / _ \\| | | | __/ _ \\ |   / _ \\ / _` |/ _ \\",
            " |  _ < (_) | |_| | ||  __/ |__| (_) | (_| |  __/",
            " |_| \\_\\___/ \\__,_|\\__\\___|\\____\\___/ \\__,_|\\___|"
        ];

        let logo_lines = if area.width < 60 { &small_logo[..] } else { &large_logo[..] };
        let logo_width = logo_lines[0].len() as u16;
        let start_x = area.x + (area.width.saturating_sub(logo_width)) / 2;
        let end_x = start_x + logo_width;
        let start_y = chunks[1].y;
        let end_y = start_y + logo_height;

        let is_hovering = if let (Some(col), Some(row)) = (app.mouse_col, app.mouse_row) {
            col >= start_x && col < end_x && row >= start_y && row < end_y
        } else {
            false
        };

        let is_animating = match animation_mode.as_str() {
            "always" => true,
            "hover" => is_hovering,
            "click" => app.logo_anim_frames > 0,
            _ => true,
        };

        let mut logo_text = Vec::new();

        for (i, line) in logo_lines.iter().enumerate() {
            let style = if is_animating {
                let color_idx = (app.tick_count as usize + i * 2) % colors.len();
                Style::default().fg(colors[color_idx]).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD)
            };
            logo_text.push(Line::from(Span::styled(*line, style)));
        }
        f.render_widget(Paragraph::new(logo_text).alignment(ratatui::layout::Alignment::Center), chunks[1]);
    }

    let input_width_percent = if area.width < 50 { 0.95 } else if area.width < 100 { 0.8 } else { 0.6 };
    let input_width = (area.width as f32 * input_width_percent) as u16;
    let input_area = Rect::new((area.width - input_width) / 2, chunks[2].y, input_width, input_height);

    f.render_widget(Block::default().style(Style::default().bg(COLOR_INPUT_BG)), input_area);
    
    let inner_input_area = Rect::new(input_area.x + 1, input_area.y + 1, input_area.width.saturating_sub(2), input_area.height.saturating_sub(2));
    app.input.set_block(Block::default().borders(Borders::NONE));
    f.render_widget(app.input.widget(), inner_input_area);

    f.set_cursor(inner_input_area.x + app.input.cursor().1 as u16, inner_input_area.y + app.input.cursor().0 as u16);

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
    let tip_text = if app.is_generating {
        format!(" {} AI is working... ", frame)
    } else {
        "ctrl+p help | esc show exit prompt".to_string()
    };
    f.render_widget(Paragraph::new(tip_text).alignment(ratatui::layout::Alignment::Center).style(Style::default().fg(COLOR_SECONDARY).add_modifier(Modifier::DIM)), chunks[6]);

    input_area
}
