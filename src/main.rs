mod app;
mod config;
mod extensions;
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
use std::env;
use std::fs;
use dirs::config_dir;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--gen-config" => {
                if let Some(mut path) = config_dir() {
                    path.push("flare");
                    if fs::create_dir_all(&path).is_err() {
                        eprintln!("Error: Unable to create configuration directory: {:?}", path);
                        std::process::exit(1);
                    }
                    path.push("config.toml");

                    if path.exists() {
                        eprintln!("Error: Configuration file already exists at {:?}", path);
                        std::process::exit(1);
                    }

                    // We generate the TOML directly from the default struct 
                    // which is now defined in assets/defaults.rs
                    let default_config_struct = AppConfig::default();
                    match toml::to_string_pretty(&default_config_struct) {
                        Ok(serialized) => {
                            match fs::write(&path, serialized) {
                                Ok(_) => {
                                    println!("Successfully generated default configuration at {:?}", path);
                                    std::process::exit(0);
                                }
                                Err(e) => {
                                    eprintln!("Error writing configuration file: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error serializing default configuration: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("Error: Could not determine configuration directory.");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                println!("Flare - An Application Launcher");
                println!("Usage: flare [OPTIONS]");
                println!("");
                println!("Options:");
                println!("  --gen-config    Generate a default config file at ~/.config/flare/config.toml");
                println!("                  (Fails if file already exists)");
                println!("  -h, --help      Print this help message");
                std::process::exit(0);
            }
            _ => {
            }
        }
    }

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
                if app.mode == app::AppMode::Authentication {
                    match key.code {
                        KeyCode::Esc => {
                            app.mode = app::AppMode::AppSelection;
                            app.clear_sudo_password();
                            app.pending_command = None;
                            app.sudo_log.clear();
                        }
                        KeyCode::Enter => app.launch_selected(),
                        KeyCode::Backspace => app.backspace_sudo_char(),
                        KeyCode::Left => app.move_sudo_cursor_left(),
                        KeyCode::Right => app.move_sudo_cursor_right(),
                        KeyCode::Char(c) => app.insert_sudo_char(c),
                        _ => {}
                    }
                } else {
                    if matches_key(&key, app.config.general.jump_to_top_key.as_deref().unwrap_or("alt+up")) {
                        app.select_first();
                        continue;
                    }
                    if matches_key(&key, app.config.general.jump_to_bottom_key.as_deref().unwrap_or("alt+down")) {
                        app.select_last();
                        continue;
                    }

                    match key.code {
                        KeyCode::Esc => app.should_quit = true,
                        KeyCode::Enter => app.launch_selected(),
                        KeyCode::Up => app.move_selection(-1),
                        KeyCode::Down => app.move_selection(1),
                        KeyCode::Left => app.move_search_cursor_left(),
                        KeyCode::Right => app.move_search_cursor_right(),
                        _ if matches_key(&key, app.config.general.favorite_key.as_deref().unwrap_or("alt+f")) => {
                            app.toggle_favorite();
                        }
                        KeyCode::Backspace => app.backspace_search_char(),
                        KeyCode::Char(c) => app.insert_search_char(c),
                        KeyCode::Tab => app.auto_complete(),
                        _ => {}
                    }
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
