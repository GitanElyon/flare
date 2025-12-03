mod app;
mod config;
mod history;
mod ui;

use crate::{app::App, config::AppConfig, ui::draw};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io;

fn main() -> Result<()> {
    let load_result = AppConfig::load();
    if let Some(warning) = &load_result.warning {
        eprintln!("{warning}");
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(load_result.config, load_result.warning);

    loop {
        terminal.draw(|f| draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Esc => app.should_quit = true,
                    KeyCode::Enter => app.launch_selected(),
                    KeyCode::Up => app.move_selection(-1),
                    KeyCode::Down => app.move_selection(1),
                    KeyCode::Left => app.select_first(),
                    KeyCode::Right => app.select_last(),
                    _ if matches_key(&key, app.config.general.favorite_key.as_deref().unwrap_or("alt+f")) => {
                        app.toggle_favorite();
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.update_filter();
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.update_filter();
                    }
                    KeyCode::Tab => app.auto_complete(),
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

fn matches_key(key: &event::KeyEvent, config_str: &str) -> bool {
    let config_str = config_str.to_lowercase();
    let parts: Vec<&str> = config_str.split('+').collect();
    
    let mut required_modifiers = KeyModifiers::empty();
    let mut required_code = None;

    for part in parts {
        match part {
            "ctrl" | "control" => required_modifiers.insert(KeyModifiers::CONTROL),
            "alt" | "option" => required_modifiers.insert(KeyModifiers::ALT),
            "shift" => required_modifiers.insert(KeyModifiers::SHIFT),
            "super" | "cmd" | "win" | "meta" => required_modifiers.insert(KeyModifiers::SUPER),
            "enter" | "return" => required_code = Some(KeyCode::Enter),
            "esc" | "escape" => required_code = Some(KeyCode::Esc),
            "backspace" => required_code = Some(KeyCode::Backspace),
            "tab" => required_code = Some(KeyCode::Tab),
            "space" => required_code = Some(KeyCode::Char(' ')),
            "up" => required_code = Some(KeyCode::Up),
            "down" => required_code = Some(KeyCode::Down),
            "left" => required_code = Some(KeyCode::Left),
            "right" => required_code = Some(KeyCode::Right),
            s if s.len() == 1 => required_code = Some(KeyCode::Char(s.chars().next().unwrap())),
            s if s.starts_with('f') && s.len() > 1 => {
                 if let Ok(n) = s[1..].parse::<u8>() {
                     required_code = Some(KeyCode::F(n));
                 }
            }
            _ => {}
        }
    }

    if let Some(code) = required_code {
        if key.code != code {
            return false;
        }
    }
    
    key.modifiers.contains(required_modifiers)
}
