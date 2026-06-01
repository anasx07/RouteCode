use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear};
use ratatui::Frame;

// --- Theme ---
pub const COLOR_PRIMARY: Color = Color::Rgb(0, 150, 255); // Ocean Blue
pub const COLOR_BG: Color = Color::Rgb(25, 25, 25);      // Midnight Charcoal
pub const COLOR_INPUT_BG: Color = Color::Rgb(35, 35, 35);// Soft Obsidian
pub const COLOR_SECONDARY: Color = Color::DarkGray;      // Slate Gray
pub const COLOR_SYSTEM: Color = Color::Yellow;           // Amber Yellow
pub const COLOR_SUCCESS: Color = Color::Green;           // Emerald Green
pub const COLOR_TEXT: Color = Color::White;              // Primary Text
pub const COLOR_DIM: Color = Color::Rgb(50, 50, 50);      // Very Dim Text/Lines

pub fn clean_model_name(name: &str, provider_id: &str) -> String {
    if (provider_id.starts_with("cloudflare") && name.starts_with("@cf/"))
        || ((provider_id == "openrouter" || provider_id == "nvidia") && name.contains('/'))
    {
        name.split('/').next_back().unwrap_or(name).to_string()
    } else {
        name.to_string()
    }
}

pub fn draw_modal(f: &mut Frame, title: &str, width: u16, height: u16, mouse_col: Option<u16>, mouse_row: Option<u16>, footer: Vec<Span>) -> Rect {
    let area = f.size();
    let modal_area = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    f.render_widget(Clear, modal_area);
    f.render_widget(Block::default().style(Style::default().bg(COLOR_BG)), modal_area);

    let main_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(1), // Header
            ratatui::layout::Constraint::Min(0),    // Content
            ratatui::layout::Constraint::Length(1), // Footer Spacer
            ratatui::layout::Constraint::Length(1), // Footer
        ])
        .margin(1)
        .split(modal_area);

    let header_layout = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(5), // "esc"
        ])
        .split(main_layout[0]);

    f.render_widget(
        ratatui::widgets::Paragraph::new(Span::styled(title, Style::default().add_modifier(Modifier::BOLD))),
        header_layout[0],
    );
    let mut esc_style = Style::default().fg(COLOR_SECONDARY);
    if let (Some(col), Some(row)) = (mouse_col, mouse_row) {
        if row <= modal_area.y + 2 && col >= modal_area.x + width.saturating_sub(10) && col <= modal_area.x + width {
            esc_style = Style::default().fg(Color::Red).add_modifier(Modifier::BOLD);
        }
    }

    f.render_widget(
        ratatui::widgets::Paragraph::new(Span::styled("esc", esc_style)),
        header_layout[1],
    );

    f.render_widget(ratatui::widgets::Paragraph::new(Line::from(footer)), main_layout[3]);

    main_layout[1]
}
