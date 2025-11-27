use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use freedesktop_desktop_entry::{Iter, default_paths, get_languages_from_env};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{
    io,
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
}

impl App {
    fn new() -> Self {
        let entries = scan_desktop_files();
        Self {
            search_query: String::new(),
            filtered_entries: entries.clone(),
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            should_quit: false,
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(1),    // List
        ])
        .split(f.area());

    let search_block = Paragraph::new(app.search_query.as_str())
        .block(Block::default().borders(Borders::ALL).title(" Search "));
    f.render_widget(search_block, chunks[0]);

    // Set cursor in search bar
    f.set_cursor_position((
        chunks[0].x + 1 + app.search_query.len() as u16,
        chunks[0].y + 1,
    ));

    let items: Vec<ListItem> = app
        .filtered_entries
        .iter()
        .map(|entry| ListItem::new(entry.name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Applications "),
        )
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black))
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, chunks[1], &mut app.list_state);
}
