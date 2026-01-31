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
        let title = if app.mode == AppMode::SymbolSelection {
            " Symbols "
        } else {
            " Search "
        };
        let search_widget = Paragraph::new(app.search_query.as_str())
            .style(config.input.style())
            .block(config.input.block(general, title));
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

    let padding = if config.inner_box.section.is_visible() {
        config.inner_box.section.border_offset(general) * 2
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
        let block = if config.inner_box.section.is_visible() {
            let title = config.inner_box.authentication_title.as_deref().unwrap_or(" Authentication ");
            config.inner_box.section.block_with_title(general, title)
        } else {
            Block::default().borders(Borders::ALL).title(" Authentication ")
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
        } else if app.mode == AppMode::Calculator {
            let mut list_items = Vec::new();
            if let Some((expr, result)) = &app.calculator_result {
                let mut text = if config.features.replace_calc_symbols {
                    format!("{} = {}", replace_symbols(expr), replace_symbols(result))
                } else {
                    format!("{} = {}", expr, result)
                };
                if config.features.fancy_numbers {
                    text = format_fancy(&text);
                }
                
                let display_text = aligned_text(&text, text_area_width, config.text.alignment());
                list_items.push(ListItem::new(Span::styled(display_text, config.text.style())).style(entry_style));
            }
            
            for entry in &app.math_history.entries {
                let mut text = if config.features.replace_calc_symbols {
                    format!("{} = {}", replace_symbols(&entry.expression), replace_symbols(&entry.result))
                } else {
                    format!("{} = {}", entry.expression, entry.result)
                };
                if config.features.fancy_numbers {
                    text = format_fancy(&text);
                }
                
                let display_text = aligned_text(&text, text_area_width, config.text.alignment());
                list_items.push(ListItem::new(Span::styled(display_text, config.text.style())).style(entry_style));
            }
            list_items
        } else if app.mode == AppMode::SymbolSelection {
            app.filtered_symbols
                .iter()
                .map(|(name, symbol)| {
                    if !config.text.is_visible() {
                        return ListItem::new(Span::raw(""));
                    }
                    let text = format!("{} {}", symbol, name);
                    let display_text = aligned_text(&text, text_area_width, config.text.alignment());
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

        if config.inner_box.section.is_visible() {
            let title = if app.mode == AppMode::AppSelection {
                config.inner_box.applications_title.as_deref().unwrap_or(" Applications ")
            } else if app.mode == AppMode::SymbolSelection {
                " Symbols "
            } else if app.mode == AppMode::Calculator {
                " Solution "
            } else {
                config.inner_box.directories_title.as_deref().unwrap_or(" Directories ")
            };
            list = list.block(config.inner_box.section.block_with_title(general, title));
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

fn replace_symbols(expr: &str) -> String {
    expr.replace("sqrt", "√")
        .replace("integrate", "∫")
        .replace("diff", "∂")
        .replace("limit", "lim")
        .replace("pi", "π")
}

fn to_superscript(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '^' {
            // Check if next is a paren for group exponent e.g. ^(x+1)
            if let Some(&next_c) = chars.peek() {
                if next_c == '(' {
                    chars.next(); // consume '('
                    // consume until ')'
                     while let Some(inner) = chars.next() {
                        if inner == ')' {
                            break;
                        }
                        result.push(char_to_superscript(inner));
                    }
                    continue; // Skip the default push for '^'
                } else if next_c == '-' || next_c.is_digit(10) {
                     // Read immediate number
                     while let Some(&inner) = chars.peek() {
                         if inner.is_digit(10) || inner == '-' || inner == '.' {
                             result.push(char_to_superscript(inner));
                             chars.next();
                         } else {
                             break;
                         }
                     }
                     continue;
                }
            }
        } 
        result.push(c);
         
    }
    

    input.replace(" ^ ", "^")
         .replace("^0", "⁰")
         .replace("^1", "¹")
         .replace("^2", "²")
         .replace("^3", "³")
         .replace("^4", "⁴")
         .replace("^5", "⁵")
         .replace("^6", "⁶")
         .replace("^7", "⁷")
         .replace("^8", "⁸")
         .replace("^9", "⁹")
         .replace("^-", "⁻")
         .replace("^(", "⁽")
         .replace(")", "⁾") 

}

fn format_fancy(expr: &str) -> String {
    let mut s = expr.replace(" ^ ", "^");
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '^' {
            // formatting mode
            let mut nesting = 0;
            // Check for parens
             if let Some(&next) = chars.peek() {
                 if next == '(' {
                     chars.next(); // eat (
                     nesting = 1;
                 }
             }
             
             let had_paren = nesting > 0;
             if had_paren {
                  result.push('⁽');
             }

             while let Some(&peek) = chars.peek() {
                 if had_paren {
                     if peek == ')' {
                         chars.next();
                         result.push('⁾');
                         break;
                     }
                      chars.next();
                      result.push(char_to_superscript(peek));
                 } else {
                     if peek.is_alphanumeric() || peek == '.' || peek == '-' {
                          chars.next();
                          result.push(char_to_superscript(peek));
                     } else {
                         break;
                     }
                 }
             }
        } else {
            result.push(c);
        }
    }
    result
}


fn char_to_superscript(c: char) -> char {
    match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' => '⁻',
        '(' => '⁽',
        ')' => '⁾',
        '.' => '⋅', // kinda?
        'x' => 'ˣ',
        'y' => 'ʸ',
        _ => c
    }
}
