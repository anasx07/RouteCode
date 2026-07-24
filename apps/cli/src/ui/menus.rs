use crate::ui::components::{
    clean_model_name, draw_modal, COLOR_PRIMARY, COLOR_SECONDARY, COLOR_SUCCESS, COLOR_TEXT,
};
use crate::ui::{ActiveModal, ApiKeyInputStage, App, ModelMenuItem, PROVIDERS};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;
use routecode_sdk::core::DynamicModelInfo;

pub fn render_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (app.menu.filtered_commands.len() + 6).min(15) as u16;
    let body_area = draw_modal(
        f,
        "Commands",
        60,
        height,
        app.mouse.col,
        app.mouse.row,
        vec![
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" select command"),
        ],
    );

    let items: Vec<ListItem> = app
        .menu.filtered_commands
        .iter()
        .map(|cmd| {
            let total_width = body_area.width.saturating_sub(4);
            let left = cmd.name.to_string();
            let right = cmd.description.to_string();
            let padding = total_width
                .saturating_sub(left.len() as u16)
                .saturating_sub(right.len() as u16);
            let spaces = " ".repeat(padding as usize);
            ListItem::new(Line::from(vec![
                Span::raw(format!(" {}", left)),
                Span::raw(spaces),
                Span::styled(right, Style::default().fg(COLOR_SECONDARY)),
                Span::raw(" "),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");

    let items_len = app.menu.filtered_commands.len();
    if app.mouse.moved {
        if let (Some(col), Some(row)) = (app.mouse.col, app.mouse.row) {
            if col >= body_area.x
                && col < body_area.x + body_area.width
                && row >= body_area.y
                && row < body_area.y + body_area.height
            {
                let idx = (row - body_area.y) as usize + app.menu.list_state.offset();
                if idx < items_len {
                    app.menu.list_state.select(Some(idx));
                }
            }
        }
    }

    f.render_stateful_widget(list, body_area, &mut app.menu.list_state);
}

pub fn render_api_key_dialog(f: &mut Frame, app: &mut App) {
    let provider_id = app.api_key.pending_provider_id.as_deref().unwrap_or("provider");
    let p_info = PROVIDERS.iter().find(|p| p.id == provider_id);
    let provider_name = p_info.map(|p| p.name).unwrap_or(provider_id);

    let title = format!("Connect {}", provider_name);
    let body_area = draw_modal(
        f,
        &title,
        60,
        10,
        app.mouse.col,
        app.mouse.row,
        vec![Span::styled(
            "Press Enter to save",
            Style::default().add_modifier(Modifier::BOLD),
        )],
    );

    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
        ])
        .split(body_area);

    let (prompt, placeholder) = match app.api_key.stage {
        ApiKeyInputStage::CloudflareAccountId => {
            ("Enter Cloudflare Account ID:".to_string(), " Account ID...")
        }
        ApiKeyInputStage::CloudflareGatewayId => {
            ("Enter Cloudflare Gateway ID:".to_string(), " Gateway ID...")
        }
        ApiKeyInputStage::CloudflareApiKey => {
            ("Enter Cloudflare API Token:".to_string(), " API Token...")
        }
        ApiKeyInputStage::VertexProject => (
            "Enter GCP Project ID (e.g. my-gcp-project):".to_string(),
            " Project ID...",
        ),
        ApiKeyInputStage::VertexLocation => (
            "Enter GCP location (us-central1, europe-west4, us):".to_string(),
            " us-central1...",
        ),
        _ => (
            format!("Enter API key for {}:", provider_name),
            " Paste your API key here...",
        ),
    };

    f.render_widget(Paragraph::new(prompt), layout[0]);

    app.api_key.input.set_placeholder_text(placeholder);
    app.api_key.input.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_SECONDARY)),
    );
    if app.api_key.stage == ApiKeyInputStage::ApiKey
        || app.api_key.stage == ApiKeyInputStage::CloudflareApiKey
    {
        app.api_key.input.set_mask_char('\u{2022}');
    } else {
        app.api_key.input.set_mask_char('\0');
    }
    f.render_widget(app.api_key.input.widget(), layout[2]);

    let (row, col) = app.api_key.input.cursor();
    f.set_cursor(layout[2].x + 1 + col as u16, layout[2].y + 1 + row as u16);
}

pub fn render_provider_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (PROVIDERS.len() + 6).min(15) as u16;
    let body_area = draw_modal(
        f,
        "AI Providers",
        60,
        height,
        app.mouse.col,
        app.mouse.row,
        vec![
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" configure API key"),
        ],
    );

    let items: Vec<ListItem> = {
        let config = crate::ui::try_lock_config(app);
        let config = match config.as_ref() {
            Some(c) => c,
            None => {
                let items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![Span::styled(
                    " Loading...",
                    Style::default().fg(COLOR_SECONDARY),
                )]))];
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(COLOR_SECONDARY)),
                );
                f.render_widget(list, body_area);
                return;
            }
        };

        PROVIDERS
            .iter()
            .map(|p| {
                let env_key = format!("{}_API_KEY", p.id.to_uppercase().replace("-", "_"));
                let is_connected =
                    config.api_keys.contains_key(p.id) || std::env::var(env_key).is_ok();

                let status = if is_connected {
                    Span::styled(" ✔ connected", Style::default().fg(COLOR_SUCCESS))
                } else {
                    Span::styled(" ✖ disconnected", Style::default().fg(COLOR_SECONDARY))
                };

                let total_width = body_area.width.saturating_sub(4);
                let left = p.name.to_string();
                let status_str = if is_connected {
                    "✔ connected"
                } else {
                    "✖ disconnected"
                };
                let padding = total_width
                    .saturating_sub(left.len() as u16)
                    .saturating_sub(status_str.len() as u16);
                let spaces = " ".repeat(padding as usize);

                ListItem::new(Line::from(vec![
                    Span::raw(format!(" {}", left)),
                    Span::raw(spaces),
                    status,
                    Span::raw(" "),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");

    let items_len = PROVIDERS.len();
    if app.mouse.moved {
        if let (Some(col), Some(row)) = (app.mouse.col, app.mouse.row) {
            if col >= body_area.x
                && col < body_area.x + body_area.width
                && row >= body_area.y
                && row < body_area.y + body_area.height
            {
                let idx = (row - body_area.y) as usize + app.menu.list_state.offset();
                if idx < items_len {
                    app.menu.list_state.select(Some(idx));
                }
            }
        }
        app.mouse.moved = false;
    }

    f.render_stateful_widget(list, body_area, &mut app.menu.list_state);
}

pub fn render_model_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (app.model_search.filtered_models.len() + 7).min(18) as u16;
    let mut footer = vec![
        Span::styled(
            "Connect provider ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("ctrl+a", Style::default().fg(COLOR_SECONDARY)),
        Span::raw("  "),
        Span::styled("Favorite ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("ctrl+f", Style::default().fg(COLOR_SECONDARY)),
    ];

    if app.model_search.is_fetching {
        let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame = spinner[(app.tick_count % spinner.len() as u64) as usize];
        footer.push(Span::raw("  "));
        footer.push(Span::styled(
            format!("{} Fetching models...", frame),
            Style::default()
                .fg(COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let body_area = draw_modal(
        f,
        "Select model",
        70,
        height,
        app.mouse.col,
        app.mouse.row,
        footer,
    );

    let layout = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(body_area);

    let search_text = app
        .model_search.search_input
        .lines()
        .first()
        .cloned()
        .unwrap_or_default();
    let search_para = if search_text.is_empty() {
        Paragraph::new(Span::styled(
            "search models...",
            Style::default().fg(COLOR_SECONDARY),
        ))
    } else {
        Paragraph::new(Span::styled(&search_text, Style::default().fg(COLOR_TEXT)))
    };
    f.render_widget(search_para, layout[0]);

    if app.active_modal == ActiveModal::ModelMenu {
        let (row, col) = app.model_search.search_input.cursor();
        f.set_cursor(layout[0].x + col as u16, layout[0].y + row as u16);
    }

    let favorites: Vec<DynamicModelInfo> = {
        let config = crate::ui::try_lock_config(app);
        match config.as_ref() {
            None => {
                let items: Vec<ListItem> = vec![ListItem::new(Line::from(vec![Span::styled(
                    " Loading...",
                    Style::default().fg(COLOR_SECONDARY),
                )]))];
                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(COLOR_SECONDARY)),
                );
                f.render_widget(list, body_area);
                return;
            }
            Some(c) => c.favorites.clone(),
        }
    };

    let items: Vec<ListItem> = app
        .model_search.filtered_models
        .iter()
        .map(|item| match item {
            ModelMenuItem::Header(title) => ListItem::new(Line::from(vec![Span::styled(
                format!("  {}", title),
                Style::default()
                    .fg(COLOR_SECONDARY)
                    .add_modifier(Modifier::DIM),
            )])),
            ModelMenuItem::Model(m) => {
                let is_fav = favorites
                    .iter()
                    .any(|fav| fav.name == m.name && fav.provider_id == m.provider_id);
                let fav_star = if is_fav { " ★" } else { "" };
                let display_name =
                    clean_model_name(&m.name, &m.provider_id).replace(":free", " Free");
                let p_name = PROVIDERS
                    .iter()
                    .find(|p| p.id == m.provider_id)
                    .map(|p| p.name)
                    .unwrap_or(&m.provider_id);

                let left = format!("{}{}", display_name, fav_star);
                let right = p_name.to_string();
                let total_width = layout[1].width.saturating_sub(4);
                let padding = total_width
                    .saturating_sub(left.len() as u16)
                    .saturating_sub(right.len() as u16);
                let spaces = " ".repeat(padding as usize);

                ListItem::new(Line::from(vec![
                    Span::raw(format!(" {}", left)),
                    Span::raw(spaces),
                    Span::raw(right),
                    Span::raw(" "),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");

    let items_len = app.model_search.filtered_models.len();
    if app.mouse.moved {
        if let (Some(col), Some(row)) = (app.mouse.col, app.mouse.row) {
            if col >= layout[1].x
                && col < layout[1].x + layout[1].width
                && row >= layout[1].y
                && row < layout[1].y + layout[1].height
            {
                let idx = (row - layout[1].y) as usize + app.menu.list_state.offset();
                if idx < items_len
                    && !matches!(
                        app.model_search.filtered_models.get(idx),
                        Some(ModelMenuItem::Header(_))
                    )
                {
                    app.menu.list_state.select(Some(idx));
                }
            }
        }
    }

    f.render_stateful_widget(list, layout[1], &mut app.menu.list_state);
}

use crate::ui::SettingsMenuItem;

pub fn render_settings_menu(f: &mut Frame, app: &mut App, _input_area: Rect) {
    let height = (app.menu.settings_items.len() + 6).min(15) as u16;
    let body_area = draw_modal(
        f,
        "Settings",
        60,
        height,
        app.mouse.col,
        app.mouse.row,
        vec![
            Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" toggle setting"),
        ],
    );

    let items: Vec<ListItem> = app
        .menu.settings_items
        .iter()
        .map(|item| match item {
            SettingsMenuItem::Header(title) => ListItem::new(Line::from(vec![Span::styled(
                format!("[{}]", title),
                Style::default()
                    .fg(COLOR_SECONDARY)
                    .add_modifier(Modifier::BOLD),
            )])),
            SettingsMenuItem::Option { name, val, .. } => {
                let total_width = body_area.width.saturating_sub(4);
                let left = format!("  {}", name);
                let right = val.to_string();
                let padding = total_width
                    .saturating_sub(left.len() as u16)
                    .saturating_sub(right.len() as u16);
                let spaces = " ".repeat(padding as usize);
                ListItem::new(Line::from(vec![
                    Span::raw(left),
                    Span::raw(spaces),
                    Span::styled(
                        right,
                        Style::default()
                            .fg(COLOR_PRIMARY)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(COLOR_PRIMARY).fg(Color::Black))
        .highlight_symbol("");

    let items_len = app.menu.settings_items.len();
    if app.mouse.moved {
        if let (Some(col), Some(row)) = (app.mouse.col, app.mouse.row) {
            if col >= body_area.x
                && col < body_area.x + body_area.width
                && row >= body_area.y
                && row < body_area.y + body_area.height
            {
                let idx = (row - body_area.y) as usize + app.menu.list_state.offset();
                if idx < items_len
                    && !matches!(
                        app.menu.settings_items.get(idx),
                        Some(SettingsMenuItem::Header(_))
                    )
                {
                    app.menu.list_state.select(Some(idx));
                }
            }
        }
    }

    f.render_stateful_widget(list, body_area, &mut app.menu.list_state);
}
