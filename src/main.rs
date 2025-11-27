use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    path::Path,
    process::Command,
};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
struct AppEntry {
    name: String,
    exec: String,
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
                // Clean up exec command (remove %u, %F placeholders common in desktop files)
                let cmd_str = entry
                    .exec
                    .split_whitespace()
                    .filter(|s| !s.starts_with('%'))
                    .collect::<Vec<_>>()
                    .join(" ");

                // Fork/spawn the process
                let _ = Command::new("sh")
                    .arg("-c")
                    .arg(format!("{} &", cmd_str)) // Run in background
                    .spawn();

                self.should_quit = true;
            }
        }
    }
}

fn scan_desktop_files() -> Vec<AppEntry> {
    let mut entries = Vec::new();
    let dirs = vec![
        "/usr/share/applications",
        "/usr/local/share/applications",
        // Add user local dir: ~/.local/share/applications
    ];

    let mut user_dir = dirs.clone();
    if let Ok(home) = std::env::var("HOME") {
        user_dir.push(&format!("{}/.local/share/applications", home));
    }
    // Fix: user_dir contains strings that need to be owned or static,
    // simpler to just iterate paths directly.
    let paths_to_scan = if let Ok(home) = std::env::var("HOME") {
        vec![
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
            format!("{}/.local/share/applications", home),
        ]
    } else {
        vec![
            "/usr/share/applications".to_string(),
            "/usr/local/share/applications".to_string(),
        ]
    };

    for dir in paths_to_scan {
        if !Path::new(&dir).exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            if entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "desktop")
            {
                if let Some(app) = parse_desktop_file(entry.path()) {
                    entries.push(app);
                }
            }
        }
    }

    // Sort and deduplicate by name
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries.dedup_by(|a, b| a.name == b.name);
    entries
}

fn parse_desktop_file(path: &Path) -> Option<AppEntry> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);

    let mut name = None;
    let mut exec = None;
    let mut is_desktop_entry = false;
    let mut no_display = false;

    for line in reader.lines().filter_map(|l| l.ok()) {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            is_desktop_entry = true;
            continue;
        }

        // Only parse the main section
        if line.starts_with('[') && line != "[Desktop Entry]" {
            is_desktop_entry = false;
        }

        if !is_desktop_entry {
            continue;
        }

        if line.starts_with("Name=") && name.is_none() {
            name = Some(line.trim_start_matches("Name=").to_string());
        } else if line.starts_with("Exec=") && exec.is_none() {
            exec = Some(line.trim_start_matches("Exec=").to_string());
        } else if line == "NoDisplay=true" {
            no_display = true;
        }
    }

    if no_display {
        return None;
    }

    match (name, exec) {
        (Some(name), Some(exec)) => Some(AppEntry { name, exec }),
        _ => None,
    }
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
