use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use dirs::config_dir;
use freedesktop_desktop_entry::{Iter, default_paths, get_languages_from_env};
use ratatui::{
    prelude::*,
    text::Span,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    process::{Command, Stdio},
};

#[derive(Debug, Clone)]
struct AppEntry {
    name: String,
    exec_args: Vec<String>,
}

struct App {
    search_query: String,
    entries: Vec<AppEntry>,
    filtered_entries: Vec<AppEntry>,
    list_state: ListState,
    should_quit: bool,
    config: AppConfig,
}

impl App {
    fn new() -> Self {
        let config = AppConfig::load();
        let entries = scan_desktop_files();
        Self {
            search_query: String::new(),
            filtered_entries: entries.clone(),
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            should_quit: false,
            config,
        }
    }

    fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_entries = self.entries.clone();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_entries = self
                .entries
                .iter()
                .filter(|e| e.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
        // Reset selection if out of bounds or empty
        if self.filtered_entries.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered_entries.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                let len = self.filtered_entries.len() as i32;
                let new_i = (i as i32 + delta).rem_euclid(len);
                new_i as usize
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn launch_selected(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if let Some(entry) = self.filtered_entries.get(i) {
                if let Some((cmd, args)) = entry.exec_args.split_first() {
                    let spawn_result = Command::new(cmd)
                        .args(args)
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn();

                    if spawn_result.is_ok() {
                        self.should_quit = true;
                    }
                }
            }
        }
    }
}

fn scan_desktop_files() -> Vec<AppEntry> {
    let locales = get_languages_from_env();
    let locale_slice = locales.as_slice();

    let mut entries: Vec<AppEntry> = Iter::new(default_paths())
        .entries(Some(locale_slice))
        .filter(|entry| !entry.no_display() && !entry.hidden())
        .filter_map(|entry| {
            let exec_args = entry.parse_exec().ok()?;
            let name = entry
                .full_name(locale_slice)
                .or_else(|| entry.name(locale_slice))
                .map(|cow| cow.into_owned())
                .unwrap_or_else(|| entry.appid.clone());

            Some(AppEntry { name, exec_args })
        })
        .collect();

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.dedup_by(|a, b| a.name == b.name);
    entries
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::Enter => app.launch_selected(),
                    KeyCode::Up => app.move_selection(-1),
                    KeyCode::Down => app.move_selection(1),
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.update_filter();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.update_filter();
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn ui(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let config = &app.config;
    let general = &config.general;

    f.render_widget(Clear, area);
    let window_block = config.window.block(general, "");
    f.render_widget(window_block, area);

    let outer_block = config.outer_box.block(general, "");
    let inner_area = outer_block.inner(area);
    f.render_widget(outer_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(1),    // List
        ])
        .split(inner_area);

    let search_widget = Paragraph::new(app.search_query.as_str())
        .style(config.input.style())
        .block(config.input.block(general, " Search "));
    f.render_widget(search_widget, chunks[0]);

    let cursor_offset = config.input.border_offset(general);
    let cursor_x = (chunks[0].x + cursor_offset + app.search_query.len() as u16)
        .min(chunks[0].x + chunks[0].width.saturating_sub(1));
    let cursor_y =
        (chunks[0].y + cursor_offset).min(chunks[0].y + chunks[0].height.saturating_sub(1));
    f.set_cursor_position((cursor_x, cursor_y));

    let items: Vec<ListItem> = app
        .filtered_entries
        .iter()
        .map(|entry| {
            let text_style = config.text.style();
            ListItem::new(Span::styled(entry.name.clone(), text_style)).style(config.entry.style())
        })
        .collect();

    let scroll_block = config.scroll.block(general, "");
    let scroll_area = scroll_block.inner(chunks[1]);
    f.render_widget(scroll_block, chunks[1]);

    let list = List::new(items)
        .block(config.inner_box.block(general, " Applications "))
        .highlight_style(config.entry_selected.style())
        .highlight_symbol(config.general.highlight_symbol.as_deref().unwrap_or(">> "));

    f.render_stateful_widget(list, scroll_area, &mut app.list_state);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct AppConfig {
    general: GeneralConfig,
    window: SectionConfig,
    outer_box: SectionConfig,
    input: SectionConfig,
    scroll: SectionConfig,
    inner_box: SectionConfig,
    entry: SectionConfig,
    entry_selected: SectionConfig,
    text: SectionConfig,
}

impl AppConfig {
    fn load() -> Self {
        let default = Self::default();
        match config_dir() {
            Some(mut dir) => {
                dir.push("flare");
                if fs::create_dir_all(&dir).is_err() {
                    return default;
                }
                let config_path = dir.join("config.toml");
                if config_path.exists() {
                    if let Ok(contents) = fs::read_to_string(&config_path) {
                        if let Ok(parsed) = toml::from_str::<AppConfig>(&contents) {
                            return parsed;
                        }
                    }
                } else if let Ok(serialized) = toml::to_string_pretty(&default) {
                    let _ = fs::write(&config_path, serialized);
                }
                default
            }
            None => default,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            window: SectionConfig {
                bg: Some(String::from("#000000")),
                ..SectionConfig::default()
            },
            outer_box: SectionConfig {
                title: Some(String::from(" Flare ")),
                border_color: Some(String::from("#cdd6f4")),
                ..SectionConfig::default()
            },
            input: SectionConfig {
                title: Some(String::from(" Search ")),
                border_color: Some(String::from("#cba6f7")),
                ..SectionConfig::default()
            },
            scroll: SectionConfig {
                border_color: Some(String::from("#585b70")),
                ..SectionConfig::default()
            },
            inner_box: SectionConfig {
                title: Some(String::from(" Applications ")),
                border_color: Some(String::from("#89b4fa")),
                ..SectionConfig::default()
            },
            entry: SectionConfig {
                bg: Some(String::from("#1e1e2e")),
                ..SectionConfig::default()
            },
            entry_selected: SectionConfig {
                fg: Some(String::from("#1e1e2e")),
                bg: Some(String::from("#cdd6f4")),
                ..SectionConfig::default()
            },
            text: SectionConfig {
                fg: Some(String::from("#f2f5f7")),
                ..SectionConfig::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct GeneralConfig {
    rounded_corners: bool,
    show_borders: bool,
    highlight_symbol: Option<String>,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            rounded_corners: true,
            show_borders: true,
            highlight_symbol: Some(String::from(">> ")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
struct SectionConfig {
    title: Option<String>,
    fg: Option<String>,
    bg: Option<String>,
    border_color: Option<String>,
    rounded: Option<bool>,
    borders: Option<bool>,
}

impl SectionConfig {
    fn style(&self) -> Style {
        let mut style = Style::default();
        if let Some(color) = self.fg.as_deref().and_then(|value| parse_color(value)) {
            style = style.fg(color);
        }
        if let Some(color) = self.bg.as_deref().and_then(|value| parse_color(value)) {
            style = style.bg(color);
        }
        style
    }

    fn border_offset(&self, general: &GeneralConfig) -> u16 {
        if self.draws_borders(general) { 1 } else { 0 }
    }

    fn draws_borders(&self, general: &GeneralConfig) -> bool {
        self.borders.unwrap_or(general.show_borders)
    }

    fn block<'a>(&self, general: &GeneralConfig, fallback_title: &'a str) -> Block<'a> {
        let mut block = Block::default().title(
            self.title
                .clone()
                .unwrap_or_else(|| fallback_title.to_string()),
        );

        if self.draws_borders(general) {
            block = block.borders(Borders::ALL);
            let rounded = self.rounded.unwrap_or(general.rounded_corners);
            block = block.border_type(if rounded {
                BorderType::Rounded
            } else {
                BorderType::Plain
            });

            if let Some(color) = self
                .border_color
                .as_deref()
                .and_then(|value| parse_color(value))
            {
                block = block.border_style(Style::default().fg(color));
            }
        }

        block.style(self.style())
    }
}

impl Default for SectionConfig {
    fn default() -> Self {
        Self {
            title: None,
            fg: None,
            bg: None,
            border_color: None,
            rounded: None,
            borders: None,
        }
    }
}

fn parse_color(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }

    match trimmed.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "dark-grey" => Some(Color::DarkGray),
        "lightred" | "light-red" => Some(Color::LightRed),
        "lightgreen" | "light-green" => Some(Color::LightGreen),
        "lightblue" | "light-blue" => Some(Color::LightBlue),
        "lightmagenta" | "light-magenta" => Some(Color::LightMagenta),
        "lightcyan" | "light-cyan" => Some(Color::LightCyan),
        "lightyellow" | "light-yellow" => Some(Color::LightYellow),
        _ => None,
    }
}
