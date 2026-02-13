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
    
    let flare_lines = app.flare_ascii.lines().count() as u16;

    if config.flare_ascii.section.is_visible() {
        let p = &config.flare_ascii.padding;
        constraints.push(Constraint::Length(flare_lines + p.top + p.bottom)); 
    }

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

    if config.flare_ascii.section.is_visible() {
        let chunk = chunks[chunk_index];
        chunk_index += 1;
        
        let p = &config.flare_ascii.padding;
        let inner_area = Rect {
            x: chunk.x + p.left,
            y: chunk.y + p.top,
            width: chunk.width.saturating_sub(p.left + p.right),
            height: chunk.height.saturating_sub(p.top + p.bottom),
        };

        let mut widget = if config.flare_ascii.gradient && !config.flare_ascii.gradient_colors.is_empty() {
             let colors: Vec<Color> = config.flare_ascii.gradient_colors.iter()
                 .filter_map(|s| crate::config::parse_color(s))
                 .collect();
             
             let lines: Vec<Line> = app.flare_ascii.lines().enumerate().map(|(i, line)| {
                 let color = if colors.is_empty() {
                     Color::White
                 } else if colors.len() == 1 {
                     colors[0]
                 } else {
                     // Simple linear interpolation between all colors provided
                     let total_lines = flare_lines.max(1) as f32;
                     let progress = i as f32 / total_lines;
                     let segment_count = (colors.len() - 1) as f32;
                     let segment_progress = progress * segment_count;
                     let segment_index = segment_progress.floor() as usize;
                     let segment_index = segment_index.min(colors.len() - 2);
                     let factor = segment_progress - segment_index as f32;
                     
                     interpolate_color(colors[segment_index], colors[segment_index + 1], factor)
                 };
                 Line::from(Span::styled(line, Style::default().fg(color)))
             }).collect();
             Paragraph::new(lines)
        } else {
             let mut p_widget = Paragraph::new(app.flare_ascii.as_str());
             if let Some(color) = config.flare_ascii.section.fg.as_deref().and_then(crate::config::parse_color) {
                  p_widget = p_widget.style(Style::default().fg(color));
             }
             p_widget
        };

        widget = widget.alignment(config.flare_ascii.alignment.unwrap_or(crate::config::TextAlignment::Center).into());
        f.render_widget(widget, inner_area);
    }

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

    let scroll_area = list_chunk;

    let padding = if config.results.section.is_visible() {
        config.results.section.border_offset(general) * 2
    } else {
        0
    };
    let mut text_area_width = scroll_area.width.saturating_sub(padding);
    text_area_width = text_area_width.saturating_sub(highlight_symbol_width(config));

    let entry_style = Style::default();
    let entry_selected_visible = config.entry_selected.is_visible();
    let highlight_style = if entry_selected_visible {
        config.entry_selected.style()
    } else {
        Style::default()
    };

    if app.mode == AppMode::SudoPassword {
        let block = if config.results.section.is_visible() {
            let title = config.results.authentication_title.as_deref().unwrap_or(" Authentication ");
            config.results.section.block_with_title(general, title)
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

                    let is_fav = app.history.is_favorite_symbol(name);
                    let fav_symbol_cfg = config.general.favorite_symbol.as_deref().unwrap_or("★ ");
                    let empty_prefix = " ".repeat(fav_symbol_cfg.chars().count());
                    let prefix = if is_fav { fav_symbol_cfg } else { &empty_prefix };

                    let text = format!("{}{} {}", prefix, symbol, name);
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

        if config.results.section.is_visible() {
            let title = if app.mode == AppMode::AppSelection {
                config.results.applications_title.as_deref().unwrap_or(" Applications ")
            } else if app.mode == AppMode::SymbolSelection {
                " Symbols "
            } else if app.mode == AppMode::Calculator {
                " Solution "
            } else {
                config.results.directories_title.as_deref().unwrap_or(" Directories ")
            };
            list = list.block(config.results.section.block_with_title(general, title));
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

#[allow(dead_code)]
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
    let s = expr.replace(" ^ ", "^");
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

fn interpolate_color(c1: Color, c2: Color, factor: f32) -> Color {
    let (r1, g1, b1) = color_to_rgb(c1);
    let (r2, g2, b2) = color_to_rgb(c2);
    
    let r = (r1 as f32 + (r2 as f32 - r1 as f32) * factor) as u8;
    let g = (g1 as f32 + (g2 as f32 - g1 as f32) * factor) as u8;
    let b = (b1 as f32 + (b2 as f32 - b1 as f32) * factor) as u8;
    
    Color::Rgb(r, g, b)
}

fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (170, 0, 0),
        Color::Green => (0, 170, 0),
        Color::Yellow => (170, 85, 0),
        Color::Blue => (0, 0, 170),
        Color::Magenta => (170, 0, 170),
        Color::Cyan => (0, 170, 170),
        Color::White => (170, 170, 170),
        Color::Gray => (85, 85, 85),
        Color::DarkGray => (85, 85, 85),
        Color::LightRed => (255, 85, 85),
        Color::LightGreen => (85, 255, 85),
        Color::LightYellow => (255, 255, 85),
        Color::LightBlue => (85, 85, 255),
        Color::LightMagenta => (255, 85, 255),
        Color::LightCyan => (85, 255, 255),
        _ => (255, 255, 255),
    }
}
