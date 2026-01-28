use crate::config::AppConfig;
use crate::history::History;
use arboard::Clipboard;
use freedesktop_desktop_entry::{Iter, default_paths, get_languages_from_env};
use ratatui::widgets::ListState;
use std::{
    fs,
    io,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    process::{Command, Stdio},
    path::Path,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    AppSelection,
    FileSelection,
    SudoPassword,
    SymbolSelection,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec_args: Vec<String>,
}

pub struct App {
    pub search_query: String,
    pub entries: Vec<AppEntry>,
    pub filtered_entries: Vec<AppEntry>,
    pub list_state: ListState,
    pub should_quit: bool,
    pub config: AppConfig,
    pub status_message: Option<String>,
    pub launch_args: Option<Vec<String>>,
    pub mode: AppMode,
    pub filtered_files: Vec<String>,
    pub filtered_symbols: Vec<(&'static str, &'static str)>,
    pub history: History,
    pub sudo_password: String,
    pub pending_command: Option<(String, Vec<String>, Vec<String>)>,
    pub sudo_log: Vec<String>,
    pub sudo_args: Vec<String>,
}

impl App {
    pub fn new(config: AppConfig, status_message: Option<String>) -> Self {
        let entries = scan_desktop_files(config.features.show_duplicates);
        let history = History::load();

        let mut app = Self {
            search_query: String::new(),
            filtered_entries: entries.clone(),
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            should_quit: false,
            config,
            status_message,
            launch_args: None,
            mode: AppMode::AppSelection,
            filtered_files: Vec::new(),
            filtered_symbols: Vec::new(),
            history,
            sudo_password: String::new(),
            pending_command: None,
            sudo_log: Vec::new(),
            sudo_args: Vec::new(),
        };

        app.sort_entries();
        app.filtered_entries = app.entries.clone();
        app
    }

    pub fn sort_entries(&mut self) {
        let history = &self.history;
        let recent_first = self.config.features.recent_first;

        self.entries.sort_by(|a, b| {
            let fav_a = history.is_favorite(&a.name);
            let fav_b = history.is_favorite(&b.name);
            if fav_a != fav_b {
                return fav_b.cmp(&fav_a);
            }

            if recent_first {
                let count_a = history.get_count(&a.name);
                let count_b = history.get_count(&b.name);
                if count_a != count_b {
                    return count_b.cmp(&count_a);
                }
            }

            a.name.to_lowercase().cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name))
        });
    }

    pub fn toggle_favorite(&mut self) {
        if self.mode == AppMode::AppSelection {
            if let Some(i) = self.list_state.selected() {
                if let Some(entry) = self.filtered_entries.get(i).cloned() {
                    self.history.toggle_favorite(&entry.name);
                    self.sort_entries();
                    self.update_filter();
                }
            }
        }
    }


    pub fn update_filter(&mut self) {
        self.launch_args = None;
        self.mode = AppMode::AppSelection;
        self.filtered_files.clear();
        self.filtered_symbols.clear();
        self.sudo_args.clear();

        if self.search_query.starts_with(&self.config.features.symbol_search_trigger) {
            self.mode = AppMode::SymbolSelection;
            self.filtered_entries.clear();
            
            let query = self.search_query
                .strip_prefix(&self.config.features.symbol_search_trigger)
                .unwrap_or("")
                .trim()
                .to_lowercase();

            if query.is_empty() {
                self.filtered_symbols = crate::symbols::SYMBOLS.iter().cloned().collect();
            } else {
                let mut matches: Vec<(i64, (&'static str, &'static str))> = crate::symbols::SYMBOLS
                    .iter()
                    .filter_map(|(name, val)| {
                        fuzzy_score(&query, name).map(|score| (score, (*name, *val)))
                    })
                    .collect();

                matches.sort_by(|a, b| b.0.cmp(&a.0));

                self.filtered_symbols = matches.into_iter().map(|(_, item)| item).collect();
            }
            // Ensure selection is valid
            if self.filtered_symbols.is_empty() {
                self.list_state.select(None);
            } else {
                self.list_state.select(Some(0));
            }
            return;
        }

        let query_slice = if self.search_query.starts_with("sudo") {
            let parts: Vec<&str> = self.search_query.split_whitespace().collect();
            let mut idx = 1; // skip "sudo"
            let mut is_sudo = false;

            if parts.first() == Some(&"sudo") {
                is_sudo = true;
                while idx < parts.len() {
                    let part = parts[idx];
                    if part.starts_with('-') {
                        self.sudo_args.push(part.to_string());
                        // check for flags that take arguments
                        // -C fd, -g group, -h host, -p prompt, -r role, -t type, -U user, -u user
                        // also handle bundled flags like -Ab (no arg) vs -u user
                        // simplified check: if it's exactly one of these flags, take next arg
                        if ["-C", "-g", "-h", "-p", "-r", "-t", "-U", "-u"].contains(&part) {
                            if idx + 1 < parts.len() {
                                idx += 1;
                                self.sudo_args.push(parts[idx].to_string());
                            }
                        }
                    } else {
                        break;
                    }
                    idx += 1;
                }
            }

            if is_sudo {
                // reconstruct the query from the remaining parts
                // we need to find where the command starts in the original string to preserve spaces if possible,
                // or just join the parts. Joining parts is safer for now.
                if idx < parts.len() {
                    // this is a bit inefficient but works
                    parts[idx..].join(" ")
                } else {
                    String::new()
                }
            } else {
                self.search_query.clone()
            }
        } else {
            self.search_query.clone()
        };
        
        let query_slice_str = query_slice.trim();

        if self.config.features.enable_file_explorer
            && (query_slice_str.starts_with('~') || query_slice_str.starts_with('/'))
        {
            self.mode = AppMode::FileSelection;
            self.filtered_files = list_files(query_slice_str, self.config.features.dirs_first);
            self.filtered_entries.clear();
        } else if query_slice_str.is_empty() {
            self.filtered_entries = self.entries.clone();
        } else {
            let query = query_slice_str.to_lowercase();
            let mut matches: Vec<(i64, AppEntry)> = self
                .entries
                .iter()
                .filter_map(|e| {
                    fuzzy_score(&query, &e.name).map(|score| (score, e.clone()))
                })
                .collect();

            matches.sort_by(|a, b| b.0.cmp(&a.0));

            let matches: Vec<AppEntry> = matches.into_iter().map(|(_, e)| e).collect();

            if !matches.is_empty() {
                self.filtered_entries = matches;
            } else {
                let words: Vec<&str> = query_slice_str.split_whitespace().collect();
                let mut found = false;

                for i in (1..words.len()).rev() {
                    let sub_query = words[0..i].join(" ");
                    let sub_query_lower = sub_query.to_lowercase();

                    let mut sub_matches: Vec<(i64, AppEntry)> = self
                        .entries
                        .iter()
                        .filter_map(|e| {
                            fuzzy_score(&sub_query_lower, &e.name).map(|score| (score, e.clone()))
                        })
                        .collect();

                    sub_matches.sort_by(|a, b| b.0.cmp(&a.0));

                    let sub_matches: Vec<AppEntry> = sub_matches.into_iter().map(|(_, e)| e).collect();

                    if !sub_matches.is_empty() {
                        self.filtered_entries = sub_matches;
                        
                        if self.config.features.enable_launch_args {
                            let args: Vec<String> = words[i..].iter().map(|s| s.to_string()).collect();
                            if let Some(last_arg) = args.last() {
                                if self.config.features.enable_file_explorer && !last_arg.starts_with('-') {
                                    let files = list_files(last_arg, self.config.features.dirs_first);
                                    if !files.is_empty() {
                                        self.filtered_files = files;
                                        self.mode = AppMode::FileSelection;
                                    }
                                }
                            }
                            self.launch_args = Some(args);
                        }
                        
                        found = true;
                        break;
                    }
                }

                if !found {
                    self.filtered_entries = Vec::new();
                }
            }
        }
        
        let count = if self.mode == AppMode::AppSelection {
            self.filtered_entries.len()
        } else {
            self.filtered_files.len()
        };

        if count == 0 {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = if self.mode == AppMode::AppSelection {
            self.filtered_entries.len()
        } else {
            self.filtered_files.len()
        };

        if len == 0 {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                let new_i = (i as i32 + delta).rem_euclid(len as i32);
                new_i as usize
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn select_first(&mut self) {
        let len = if self.mode == AppMode::AppSelection {
            self.filtered_entries.len()
        } else {
            self.filtered_files.len()
        };

        if len > 0 {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        let len = if self.mode == AppMode::AppSelection {
            self.filtered_entries.len()
        } else {
            self.filtered_files.len()
        };

        if len > 0 {
            self.list_state.select(Some(len - 1));
        }
    }

    pub fn auto_complete(&mut self) {
        if !self.config.features.enable_auto_complete {
            return;
        }
        if self.mode == AppMode::FileSelection {
            if let Some(i) = self.list_state.selected() {
                if let Some(selected_file) = self.filtered_files.get(i) {
                    let mut new_path = selected_file.clone();
                    
                    let expanded_path = expand_tilde(&new_path);
                    if Path::new(&expanded_path).is_dir() {
                        new_path.push('/');
                    }

                    if let Some(last_space_idx) = self.search_query.rfind(' ') {
                        let (prefix, _) = self.search_query.split_at(last_space_idx + 1);
                        self.search_query = format!("{}{}", prefix, new_path);
                    } else {
                        self.search_query = new_path;
                    }
                    self.update_filter();
                }
            }
        }
    }

    pub fn launch_selected(&mut self) {
        if self.mode == AppMode::SudoPassword {
            self.verify_sudo_and_launch();
            return;
        }

        if self.mode == AppMode::SymbolSelection {
            if let Some(i) = self.list_state.selected() {
                if let Some((_, symbol)) = self.filtered_symbols.get(i) {
                    let mut copied = false;
                    if let Some(cmd) = &self.config.general.clipboard_command {
                        let parts: Vec<&str> = cmd.split_whitespace().collect();
                        if let Some((prog, args)) = parts.split_first() {
                            let child = Command::new(prog)
                                .args(args)
                                .stdin(Stdio::piped())
                                .spawn();
                            
                            if let Ok(mut child) = child {
                                use std::io::Write;
                                if let Some(mut stdin) = child.stdin.take() {
                                    let _ = stdin.write_all(symbol.as_bytes());
                                }
                                let _ = child.wait();
                                copied = true;
                                self.should_quit = true;
                            } else {
                                self.status_message = Some(format!("Failed to run clipboard command: {}", cmd));
                            }
                        }
                    }

                    if !copied {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if let Err(e) = clipboard.set_text(symbol.to_string()) {
                                self.status_message = Some(format!("Clipboard error: {}", e));
                            } else {
                                self.should_quit = true;
                            }
                        } else {
                            self.status_message = Some("Failed to initialize clipboard".to_string());
                        }
                    }
                }
            }
            return;
        }

        if let Some(i) = self.list_state.selected() {
            if self.mode == AppMode::FileSelection && self.filtered_entries.is_empty() {
                if let Some(selected_file) = self.filtered_files.get(i).cloned() {
                    self.open_file(&selected_file);
                }
                return;
            }

            let app_entry = if self.mode == AppMode::FileSelection {
                self.filtered_entries.first().cloned()
            } else {
                self.filtered_entries.get(i).cloned()
            };

            if let Some(entry) = app_entry {
                self.history.increment(&entry.name);
                if let Some((cmd, args)) = entry.exec_args.split_first() {
                    let mut final_args = Vec::new();

                    if self.config.features.enable_launch_args {
                        if let Some(launch_args) = &self.launch_args {
                            let mut current_launch_args = launch_args.clone();
                            
                            if self.mode == AppMode::FileSelection {
                                if let Some(selected_file) = self.filtered_files.get(i) {
                                    if let Some(last) = current_launch_args.last_mut() {
                                        *last = selected_file.clone();
                                    }
                                }
                            }

                            let expanded_launch_args: Vec<String> = current_launch_args
                                .iter()
                                .map(|arg| expand_tilde(arg))
                                .collect();

                            let mut replaced = false;
                            for arg in args {
                                if ["%f", "%F", "%u", "%U"].contains(&arg.as_str()) {
                                    final_args.extend(expanded_launch_args.clone());
                                    replaced = true;
                                } else {
                                    final_args.push(arg.clone());
                                }
                            }

                            if !replaced {
                                final_args.extend(expanded_launch_args);
                            }
                        } else {
                            for arg in args {
                                if !["%f", "%F", "%u", "%U"].contains(&arg.as_str()) {
                                    final_args.push(arg.clone());
                                }
                            }
                        }
                    } else {
                        for arg in args {
                            if !["%f", "%F", "%u", "%U"].contains(&arg.as_str()) {
                                final_args.push(arg.clone());
                            }
                        }
                    }

                    if self.search_query.starts_with("sudo") {
                        self.mode = AppMode::SudoPassword;
                        self.sudo_password.clear();
                        self.sudo_log = vec!["Password: ".to_string()];
                        self.pending_command = Some((cmd.clone(), final_args, self.sudo_args.clone()));
                        return;
                    }

                    self.spawn_command(cmd, final_args, &entry.name);
                }
            }
        }
    }

    fn spawn_command(&mut self, cmd: &str, args: Vec<String>, entry_name: &str) {
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                Ok(()) as io::Result<()>
            });
        }

        match command.spawn() {
            Ok(_) => {
                self.should_quit = true;
                self.status_message = None;
            }
            Err(err) => {
                self.status_message =
                    Some(format!("Failed to launch {}: {}", entry_name, err));
            }
        }
    }

    fn verify_sudo_and_launch(&mut self) {
        if let Some((cmd, args, sudo_args)) = &self.pending_command {
            // filter sudo args for validation (only allow safe args)
            let validation_args: Vec<String> = sudo_args.iter()
                .filter(|arg| ["-u", "-g", "-h", "-p", "-n", "-k", "-S"].contains(&arg.as_str()) || !arg.starts_with('-'))
                .cloned()
                .collect();

            let child = Command::new("sudo")
                .args(validation_args)
                .arg("-v")
                .arg("-S")
                .arg("-k")
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();

            match child {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        if let Err(_) = writeln!(stdin, "{}", self.sudo_password) {
                             self.sudo_log.push("Failed to write password".to_string());
                             self.sudo_log.push("Password: ".to_string());
                             self.sudo_password.clear();
                             return;
                        }
                    }
                    
                    match child.wait() {
                        Ok(status) => {
                            if status.success() {
                                let mut command = Command::new("sudo");
                                command.args(sudo_args);
                                command.arg("-b"); // run in background
                                command.arg("-S");
                                command.arg(cmd);
                                command.args(args);
                                
                                command.stdin(Stdio::piped())
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null());
                                    
                                unsafe {
                                    command.pre_exec(|| {
                                        libc::setsid();
                                        libc::signal(libc::SIGHUP, libc::SIG_IGN);
                                        Ok(()) as io::Result<()>
                                    });
                                }
                                
                                match command.spawn() {
                                    Ok(mut child) => {
                                         if let Some(mut stdin) = child.stdin.take() {
                                            use std::io::Write;
                                            let _ = writeln!(stdin, "{}", self.sudo_password);
                                        }
                                        self.should_quit = true;
                                        self.status_message = None;
                                    }
                                    Err(err) => {
                                        self.status_message = Some(format!("Failed to launch sudo: {}", err));
                                    }
                                }
                            } else {
                                self.sudo_log.push("Sorry, try again.".to_string());
                                self.sudo_log.push("Password: ".to_string());
                                self.sudo_password.clear();
                            }
                        }
                        Err(e) => {
                             self.sudo_log.push(format!("Sudo check failed: {}", e));
                             self.sudo_log.push("Password: ".to_string());
                             self.sudo_password.clear();
                        }
                    }
                }
                Err(e) => {
                    self.sudo_log.push(format!("Failed to run sudo: {}", e));
                    self.sudo_log.push("Password: ".to_string());
                    self.sudo_password.clear();
                }
            }
        }
    }

    fn open_file(&mut self, path_str: &str) {
        let expanded = expand_tilde(path_str);
        let path = Path::new(&expanded);

        let is_executable = if let Ok(metadata) = fs::metadata(path) {
            metadata.permissions().mode() & 0o111 != 0
        } else {
            false
        };

        let mut command = if is_executable && !path.is_dir() {
            Command::new(path)
        } else {
            let mut cmd = Command::new("xdg-open");
            cmd.arg(path);
            cmd
        };

        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                libc::signal(libc::SIGHUP, libc::SIG_IGN);
                Ok(()) as io::Result<()>
            });
        }

        match command.spawn() {
            Ok(_) => {
                self.should_quit = true;
                self.status_message = None;
            }
            Err(err) => {
                self.status_message = Some(format!("Failed to open {}: {}", path_str, err));
            }
        }
    }
}

fn scan_desktop_files(show_duplicates: bool) -> Vec<AppEntry> {
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

    entries.sort_by(|a, b| {
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
            .then_with(|| a.name.cmp(&b.name))
    });
    
    if !show_duplicates {
        entries.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
    }
    
    entries
}


fn fuzzy_score(query: &str, target: &str) -> Option<i64> {
    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();

    if query_chars.is_empty() {
        return Some(0);
    }

    let mut score = 0;
    let mut pattern_idx = 0;
    let mut prev_match_idx = -100;

    for (idx, &t_char) in target_chars.iter().enumerate() {
        if pattern_idx < query_chars.len() {
            let q_char = query_chars[pattern_idx];
            if t_char.eq_ignore_ascii_case(&q_char) {
                let mut char_score = 10;

                if idx as i64 == prev_match_idx + 1 {
                    char_score += 40;
                }

                if idx == 0
                    || target_chars[idx - 1].is_whitespace()
                    || ['_', '-', '.', '/'].contains(&target_chars[idx - 1])
                {
                    char_score += 20;
                }

                if t_char.is_uppercase() {
                    char_score += 10;
                }

                score += char_score;
                prev_match_idx = idx as i64;
                pattern_idx += 1;
            }
        }
    }

    if pattern_idx == query_chars.len() {
        score -= (target_chars.len() as i64 - query_chars.len() as i64);
        return Some(score);
    }
    None
}

fn fuzzy_match(query: &str, target: &str) -> bool {
    fuzzy_score(query, target).is_some()
}

fn expand_tilde(path: &str) -> String {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home.to_string_lossy().to_string();
        }
    }
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.to_string_lossy(), stripped);
        }
    }
    path.to_string()
}

fn list_files(query_path: &str, dirs_first: bool) -> Vec<String> {
    let expanded = expand_tilde(query_path);
    let path = Path::new(&expanded);

    let (dir, file_prefix) = if query_path.ends_with('/') {
        (path, "")
    } else {
        (
            path.parent().unwrap_or(Path::new("")),
            path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
        )
    };

    let search_dir = if dir.as_os_str().is_empty() {
        if query_path.starts_with('/') {
            Path::new("/")
        } else {
            Path::new(".")
        }
    } else {
        dir
    };

    let mut entries_vec: Vec<(String, bool, i64)> = Vec::new();
    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);

            if name.starts_with('.') && !file_prefix.starts_with('.') {
                continue;
            }
            if let Some(score) = fuzzy_score(file_prefix, name) {
                let mut path_str = path.to_string_lossy().to_string();
                if query_path.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        let home_str = home.to_string_lossy();
                        if path_str.starts_with(home_str.as_ref()) {
                            path_str = format!("~{}", &path_str[home_str.len()..]);
                        }
                    }
                }
                entries_vec.push((path_str, is_dir, score));
            }
        }
    }

    if dirs_first {
        entries_vec.sort_by(
            |(a_path, a_is_dir, a_score), (b_path, b_is_dir, b_score)| {
                if *a_is_dir != *b_is_dir {
                    b_is_dir.cmp(a_is_dir)
                } else {
                    b_score.cmp(a_score).then_with(|| a_path.cmp(b_path))
                }
            },
        );
    } else {
        entries_vec.sort_by(|(a_path, _, a_score), (b_path, _, b_score)| {
            b_score.cmp(a_score).then_with(|| a_path.cmp(b_path))
        });
    }

    entries_vec.into_iter().map(|(p, _, _)| p).collect()
}
