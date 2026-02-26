use crate::config::AppConfig;
use crate::extensions::{
    AuthResult, ExtensionListAction, ExtensionListItem, ExtensionRegistry, ExtensionResult,
};
use crate::history::{History, MathHistory};
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
    Authentication,
    ExtensionList,
    SingleResult,
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub exec_args: Vec<String>,
}

pub struct App {
    pub search_query: String,
    pub search_cursor: usize,
    pub entries: Vec<AppEntry>,
    pub filtered_entries: Vec<AppEntry>,
    pub list_state: ListState,
    pub should_quit: bool,
    pub config: AppConfig,
    pub status_message: Option<String>,
    pub launch_args: Option<Vec<String>>,
    pub mode: AppMode,
    pub filtered_files: Vec<String>,
    pub filtered_extension_items: Vec<ExtensionListItem>,
    pub extension_action: ExtensionListAction,
    pub extension_list_title: Option<String>,
    pub history: History,
    pub sudo_password: String,
    pub sudo_password_cursor: usize,
    pub pending_command: Option<(String, Vec<String>, Vec<String>)>,
    pub sudo_log: Vec<String>,
    pub launch_prefix_args: Vec<String>,
    pub single_result: Option<(String, String)>,
    pub single_result_query_prefix: String,
    pub math_history: MathHistory,
    pub flare_ascii: String,
    pub extension_registry: ExtensionRegistry,
}

impl App {
    pub fn new(config: AppConfig, status_message: Option<String>) -> Self {
        let entries = scan_desktop_files(config.features.show_duplicates);
        let history = History::load();
        let math_history = MathHistory::load();

        let extension_registry = ExtensionRegistry::new(&config);

        let flare_ascii = if let Some(path) = &config.flare_ascii.custom_path {
            let expanded_path = crate::extensions::files::expand_tilde(path);
            fs::read_to_string(expanded_path).unwrap_or_else(|_| include_str!("../assets/flare.txt").to_string())
        } else {
            include_str!("../assets/flare.txt").to_string()
        };

        let mut app = Self {
            search_query: String::new(),
            search_cursor: 0,
            filtered_entries: entries.clone(),
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            should_quit: false,
            config,
            status_message,
            launch_args: None,
            mode: AppMode::AppSelection,
            filtered_files: Vec::new(),
            filtered_extension_items: Vec::new(),
            extension_action: ExtensionListAction::None,
            extension_list_title: None,
            history,
            sudo_password: String::new(),
            sudo_password_cursor: 0,
            pending_command: None,
            sudo_log: Vec::new(),
            launch_prefix_args: Vec::new(),
            single_result: None,
            single_result_query_prefix: String::new(),
            math_history,
            flare_ascii,
            extension_registry,
        };

        app.sort_entries();
        app.filtered_entries = app.entries.clone();
        app
    }

    fn char_count(input: &str) -> usize {
        input.chars().count()
    }

    fn byte_index_at_char(input: &str, char_idx: usize) -> usize {
        if char_idx == 0 {
            return 0;
        }

        input
            .char_indices()
            .nth(char_idx)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| input.len())
    }

    pub fn move_search_cursor_left(&mut self) {
        if self.search_cursor > 0 {
            self.search_cursor -= 1;
        }
    }

    pub fn move_search_cursor_right(&mut self) {
        let len = Self::char_count(&self.search_query);
        if self.search_cursor < len {
            self.search_cursor += 1;
        }
    }

    pub fn insert_search_char(&mut self, ch: char) {
        let byte_idx = Self::byte_index_at_char(&self.search_query, self.search_cursor);
        self.search_query.insert(byte_idx, ch);
        self.search_cursor += 1;
        self.update_filter();
    }

    pub fn insert_search_text(&mut self, text: &str) {
        let byte_idx = Self::byte_index_at_char(&self.search_query, self.search_cursor);
        self.search_query.insert_str(byte_idx, text);
        self.search_cursor += Self::char_count(text);
        self.update_filter();
    }

    pub fn backspace_search_char(&mut self) {
        if self.search_cursor == 0 {
            return;
        }

        let end = Self::byte_index_at_char(&self.search_query, self.search_cursor);
        let start = Self::byte_index_at_char(&self.search_query, self.search_cursor - 1);
        self.search_query.replace_range(start..end, "");
        self.search_cursor -= 1;
        self.update_filter();
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.search_cursor = Self::char_count(&self.search_query);
    }

    pub fn move_sudo_cursor_left(&mut self) {
        if self.sudo_password_cursor > 0 {
            self.sudo_password_cursor -= 1;
        }
    }

    pub fn move_sudo_cursor_right(&mut self) {
        let len = Self::char_count(&self.sudo_password);
        if self.sudo_password_cursor < len {
            self.sudo_password_cursor += 1;
        }
    }

    pub fn insert_sudo_char(&mut self, ch: char) {
        let byte_idx = Self::byte_index_at_char(&self.sudo_password, self.sudo_password_cursor);
        self.sudo_password.insert(byte_idx, ch);
        self.sudo_password_cursor += 1;
    }

    pub fn backspace_sudo_char(&mut self) {
        if self.sudo_password_cursor == 0 {
            return;
        }

        let end = Self::byte_index_at_char(&self.sudo_password, self.sudo_password_cursor);
        let start = Self::byte_index_at_char(&self.sudo_password, self.sudo_password_cursor - 1);
        self.sudo_password.replace_range(start..end, "");
        self.sudo_password_cursor -= 1;
    }

    pub fn clear_sudo_password(&mut self) {
        self.sudo_password.clear();
        self.sudo_password_cursor = 0;
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
        self.filtered_extension_items.clear();
        self.extension_action = ExtensionListAction::None;
        self.extension_list_title = None;
        self.launch_prefix_args.clear();
        self.single_result = None;
        self.single_result_query_prefix = String::new();

        // Check extensions from the registry first
        for ext in &self.extension_registry.extensions {
            if ext.should_handle(&self.search_query, &self.config) {
                match ext.process(&self.search_query, &self.config, &self.extension_registry) {
                    ExtensionResult::Single { query, result } => {
                        self.mode = AppMode::SingleResult;
                        self.filtered_entries.clear();
                        self.single_result = Some((query, result));
                        self.single_result_query_prefix = ext.metadata(&self.config).trigger.clone();
                        self.list_state.select(Some(0));
                        return;
                    }
                    ExtensionResult::List { title, items, action } => {
                        self.mode = AppMode::ExtensionList;
                        self.filtered_entries.clear();
                        self.filtered_extension_items = items;
                        self.extension_action = action;
                        self.extension_list_title = Some(title);
                        if self.filtered_extension_items.is_empty() {
                            self.list_state.select(None);
                        } else {
                            self.list_state.select(Some(0));
                        }
                        return;
                    }
                    ExtensionResult::Files(files) => {
                        self.mode = AppMode::FileSelection;
                        self.filtered_files = files;
                        self.filtered_entries.clear();
                        // Files don't return early here because we might also want to search apps
                    }
                    ExtensionResult::None => {}
                }
            }
        }

        let query_slice = {
            let (stripped, prefix_args) = self.extension_registry.preprocess_query(&self.search_query, &self.config);
            self.launch_prefix_args = prefix_args;
            stripped
        };
        
        let query_slice_str = query_slice.trim();

        if self.mode != AppMode::FileSelection && query_slice_str.is_empty() {
            self.filtered_entries = self.entries.clone();
        } else if self.mode != AppMode::FileSelection {
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
                                if !last_arg.starts_with('-') {
                                    let files = self.extension_registry.list_completions(last_arg, &self.config);
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
        
        let count = match self.mode {
            AppMode::AppSelection => self.filtered_entries.len(),
            AppMode::FileSelection => self.filtered_files.len(),
            AppMode::ExtensionList => self.filtered_extension_items.len(),
            AppMode::SingleResult => {
                let mut len = self.math_history.entries.len();
                if self.single_result.is_some() {
                    len += 1;
                }
                len
            }
            _ => 0,
        };

        if count == 0 {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = match self.mode {
            AppMode::AppSelection => self.filtered_entries.len(),
            AppMode::FileSelection => self.filtered_files.len(),
            AppMode::ExtensionList => self.filtered_extension_items.len(),
            AppMode::SingleResult => {
                let mut len = self.math_history.entries.len();
                if self.single_result.is_some() {
                    len += 1;
                }
                len
            }
            _ => 0,
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
        let len = match self.mode {
            AppMode::AppSelection => self.filtered_entries.len(),
            AppMode::FileSelection => self.filtered_files.len(),
            AppMode::ExtensionList => self.filtered_extension_items.len(),
            AppMode::SingleResult => {
                let mut len = self.math_history.entries.len();
                if self.single_result.is_some() {
                    len += 1;
                }
                len
            }
            _ => 0,
        };

        if len > 0 {
            self.list_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        let len = match self.mode {
            AppMode::AppSelection => self.filtered_entries.len(),
            AppMode::FileSelection => self.filtered_files.len(),
            AppMode::ExtensionList => self.filtered_extension_items.len(),
            AppMode::SingleResult => {
                let mut len = self.math_history.entries.len();
                if self.single_result.is_some() {
                    len += 1;
                }
                len
            }
            _ => 0,
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

                    let expanded_path = self.extension_registry.expand_path(&new_path);
                    if Path::new(&expanded_path).is_dir() {
                        new_path.push('/');
                    }

                    if let Some(last_space_idx) = self.search_query.rfind(' ') {
                        let (prefix, _) = self.search_query.split_at(last_space_idx + 1);
                        self.set_search_query(format!("{}{}", prefix, new_path));
                    } else {
                        self.set_search_query(new_path);
                    }
                    self.update_filter();
                }
            }
        }
    }

    pub fn launch_selected(&mut self) {
        if self.mode == AppMode::Authentication {
            self.verify_sudo_and_launch();
            return;
        }

        if self.mode == AppMode::SingleResult {
             if let Some(i) = self.list_state.selected() {
                 let has_result = self.single_result.is_some();

                 if has_result && i == 0 {
                     if let Some((expr, res)) = &self.single_result {
                         // Only save non-empty, non-error results
                         if res != "Error" && !expr.is_empty() && !res.is_empty() {
                             self.math_history.add(expr.clone(), res.clone());
                             let prefix = self.single_result_query_prefix.clone();
                             self.set_search_query(prefix);
                             self.update_filter();
                             self.list_state.select(Some(0)); // Select first history item
                         }
                     }
                 } else {
                     let idx = if has_result { i - 1 } else { i };
                     if let Some(entry) = self.math_history.entries.get(idx) {
                         let res = entry.result.clone();
                         // Append the result to whatever is currently in the search bar
                         self.insert_search_text(&res);
                     }
                 }
             }
             return;
        }

        if self.mode == AppMode::ExtensionList {
            if let Some(i) = self.list_state.selected() {
                if let Some(item) = self.filtered_extension_items.get(i) {
                    match self.extension_action {
                        ExtensionListAction::CopyToClipboardAndExit => {
                            let text = item.value.clone();
                            use std::io::Write;
                            if let Ok(mut child) = Command::new("wl-copy")
                                .stdin(Stdio::piped())
                                .stdout(Stdio::null())
                                .stderr(Stdio::null())
                                .spawn()
                            {
                                if let Some(mut stdin) = child.stdin.take() {
                                    let _ = stdin.write_all(text.as_bytes());
                                }
                                let _ = child.wait();
                            }
                            self.should_quit = true;
                        }
                        ExtensionListAction::SetSearchQuery => {
                            let query = item.value.clone();
                            self.set_search_query(query);
                            self.update_filter();
                        }
                        ExtensionListAction::AppendToQuery => {
                            let value = item.value.clone();
                            self.insert_search_text(&value);
                        }
                        ExtensionListAction::ExecuteAndExit => {
                            let cmd_str = item.value.clone();
                            if !cmd_str.is_empty() {
                                let mut command = Command::new("sh");
                                command
                                    .args(["-c", &cmd_str])
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
                                    Ok(_) => self.should_quit = true,
                                    Err(e) => self.status_message = Some(format!("Failed to run: {}", e)),
                                }
                            }
                        }
                        ExtensionListAction::ExecuteAndRefresh => {
                            let cmd_str = item.value.clone();
                            let saved_index = self.list_state.selected();
                            if !cmd_str.is_empty() {
                                let _ = Command::new("sh")
                                    .args(["-c", &cmd_str])
                                    .stdout(Stdio::null())
                                    .stderr(Stdio::null())
                                    .output();
                            }
                            self.update_filter();
                            // Restore cursor position after refresh (clamped to new list length)
                            if let Some(idx) = saved_index {
                                let new_len = self.filtered_extension_items.len();
                                if new_len > 0 {
                                    self.list_state.select(Some(idx.min(new_len - 1)));
                                }
                            }
                        }
                        ExtensionListAction::None => {}
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
                                .map(|arg| self.extension_registry.expand_path(arg))
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

                    if self.extension_registry.requires_auth(&self.search_query, &self.config) {
                        self.mode = AppMode::Authentication;
                        self.clear_sudo_password();
                        self.sudo_log = vec!["Password: ".to_string()];
                        self.pending_command = Some((cmd.clone(), final_args, self.launch_prefix_args.clone()));
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
        if let Some((cmd, args, prefix_args)) = self.pending_command.clone() {
            match self.extension_registry.authenticate_and_launch(
                &self.sudo_password,
                &cmd,
                &args,
                &prefix_args,
                &self.search_query,
                &self.config,
            ) {
                AuthResult::Success => {
                    self.should_quit = true;
                    self.status_message = None;
                }
                AuthResult::AuthFailed => {
                    self.sudo_log.push("Sorry, try again.".to_string());
                    self.sudo_log.push("Password: ".to_string());
                    self.clear_sudo_password();
                }
                AuthResult::LaunchError(e) => {
                    self.sudo_log.push(format!("Error: {}", e));
                    self.sudo_log.push("Password: ".to_string());
                    self.clear_sudo_password();
                }
            }
        }
    }

    fn open_file(&mut self, path_str: &str) {
        let expanded = self.extension_registry.expand_path(path_str);
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


pub(crate) fn fuzzy_score(query: &str, target: &str) -> Option<i64> {
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
        score -= target_chars.len() as i64 - query_chars.len() as i64;
        return Some(score);
    }
    None
}

#[allow(dead_code)]
fn fuzzy_match(query: &str, target: &str) -> bool {
    fuzzy_score(query, target).is_some()
}

