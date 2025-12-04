use crate::{app::{App, AppMode}, config::TextAlignment};
use ratatui::{
    prelude::*,
    text::Span,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let config = &app.config;
    let general = &config.general;

    f.render_widget(Clear, area);

    let mut working_area = area;
    if config.window.is_visible() {
        let block = config.window.block(general, "");
        let inner = block.inner(area);
        f.render_widget(block, area);
        working_area = inner;
    }

    if config.outer_box.is_visible() {
        let block = config.outer_box.block(general, "");
        let inner = block.inner(working_area);
        f.render_widget(block, working_area);
        working_area = inner;
    }

    let mut constraints = Vec::new();
    if config.input.is_visible() {
        constraints.push(Constraint::Length(3));
    }
    if app.status_message.is_some() {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(working_area);

    let mut chunk_index = 0;
    let search_chunk = if config.input.is_visible() {
        let chunk = chunks[chunk_index];
        chunk_index += 1;
        Some(chunk)
    } else {
        None
    };

    let status_chunk = if app.status_message.is_some() {
        let chunk = chunks[chunk_index];
        chunk_index += 1;
        Some(chunk)
    } else {
        None
    };

    let list_chunk = chunks[chunk_index];

    if let Some(chunk) = search_chunk {
        let search_widget = Paragraph::new(app.search_query.as_str())
            .style(config.input.style())
            .block(config.input.block(general, " Search "));
        f.render_widget(search_widget, chunk);

        let cursor_offset = config.input.border_offset(general);
        let cursor_x = (chunk.x + cursor_offset + app.search_query.len() as u16)
            .min(chunk.x + chunk.width.saturating_sub(1));
        let cursor_y = (chunk.y + cursor_offset).min(chunk.y + chunk.height.saturating_sub(1));
        f.set_cursor_position((cursor_x, cursor_y));
    } else {
        f.set_cursor_position((list_chunk.x, list_chunk.y));
    }

    if let Some(chunk) = status_chunk {
        if let Some(message) = &app.status_message {
            let status = Paragraph::new(message.as_str()).style(Style::default().fg(Color::Yellow));
            f.render_widget(status, chunk);
        }
    }

    let mut scroll_area = list_chunk;
    if config.scroll.is_visible() {
        let block = config.scroll.block(general, "");
        let inner = block.inner(list_chunk);
        f.render_widget(block, list_chunk);
        scroll_area = inner;
    }

    let padding = if config.inner_box.is_visible() {
        config.inner_box.border_offset(general) * 2
    } else {
        0
    };
    let mut text_area_width = scroll_area.width.saturating_sub(padding);
    text_area_width = text_area_width.saturating_sub(highlight_symbol_width(config));

    let entry_style = if config.entry.is_visible() {
        config.entry.style()
    } else {
        Style::default()
    };
    let entry_selected_visible = config.entry_selected.is_visible();
    let highlight_style = if entry_selected_visible {
        config.entry_selected.style()
    } else {
        Style::default()
    };

    if app.mode == AppMode::SudoPassword {
        let block = if config.inner_box.is_visible() {
            config.inner_box.block(general, " Sudo Password ")
        } else {
            Block::default().borders(Borders::ALL).title(" Sudo Password ")
        };

        let inner = block.inner(scroll_area);
        f.render_widget(block, scroll_area);

        let mut items: Vec<ListItem> = Vec::new();
        for log_line in app.sudo_log.iter() {
            let display = log_line.clone();

            items.push(ListItem::new(Span::raw(display)));
        }

        let list = List::new(items);
        f.render_widget(list, inner);
    } else {
        let items: Vec<ListItem> = if app.mode == AppMode::AppSelection {
            app.filtered_entries
                .iter()
                .map(|entry| {
                    if !config.text.is_visible() {
                        return ListItem::new(Span::raw(""));
                    }

                    let is_fav = app.history.is_favorite(&entry.name);
                    let fav_symbol = config.general.favorite_symbol.as_deref().unwrap_or("★ ");
                    let empty_prefix = " ".repeat(fav_symbol.chars().count());
                    let prefix = if is_fav { fav_symbol } else { &empty_prefix };
                    let name_with_icon = format!("{}{}", prefix, entry.name);

                    let display_text =
                        aligned_text(&name_with_icon, text_area_width, config.text.alignment());
                    ListItem::new(Span::styled(display_text, config.text.style())).style(entry_style)
                })
                .collect()
        } else {
            app.filtered_files
                .iter()
                .map(|file| {
                    if !config.text.is_visible() {
                        return ListItem::new(Span::raw(""));
                    }
                    let display_text = aligned_text(file, text_area_width, config.text.alignment());
                    ListItem::new(Span::styled(display_text, config.text.style())).style(entry_style)
                })
                .collect()
        };

        let highlight_symbol = if entry_selected_visible {
            config.general.highlight_symbol.as_deref().unwrap_or(">> ")
        } else {
            ""
        };

        let mut list = List::new(items)
            .highlight_style(highlight_style)
            .highlight_symbol(highlight_symbol);

        if config.inner_box.is_visible() {
            let title = if app.mode == AppMode::AppSelection {
                " Applications "
            } else {
                " Files "
            };
            list = list.block(config.inner_box.block(general, title));
        }

        f.render_stateful_widget(list, scroll_area, &mut app.list_state);
    }
}

fn aligned_text(text: &str, width: u16, alignment: TextAlignment) -> String {
    if width == 0 {
        return text.to_string();
    }

    let width = width as usize;
    let current = text.chars().count();
    if current >= width {
        return text.to_string();
    }

    let padding = width - current;
    match alignment {
        TextAlignment::Left => text.to_string(),
        TextAlignment::Right => format!("{:>width$}", text, width = width),
        TextAlignment::Center => {
            let left = padding / 2;
            let right = padding - left;
            format!(
                "{left_padding}{text}{right_padding}",
                left_padding = " ".repeat(left),
                right_padding = " ".repeat(right)
            )
        }
    }
}

fn highlight_symbol_width(config: &crate::config::AppConfig) -> u16 {
    config
        .general
        .highlight_symbol
        .as_deref()
        .map(|s| s.chars().count() as u16)
        .unwrap_or(0)
}
