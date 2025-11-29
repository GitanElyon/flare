use crate::config::AppConfig;
use freedesktop_desktop_entry::{Iter, default_paths, get_languages_from_env};
use ratatui::widgets::ListState;
use std::{
    fs,
    io,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    path::Path,
};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    AppSelection,
    FileSelection,
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
}

impl App {
    pub fn new(config: AppConfig, status_message: Option<String>) -> Self {
        let entries = scan_desktop_files();
        Self {
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
        }
    }

    pub fn update_filter(&mut self) {
        self.launch_args = None;
        self.mode = AppMode::AppSelection;
        self.filtered_files.clear();

        if self.search_query.is_empty() {
            self.filtered_entries = self.entries.clone();
        } else {
            let query = self.search_query.to_lowercase();
            let matches: Vec<AppEntry> = self
                .entries
                .iter()
                .filter(|e| fuzzy_match(&query, &e.name.to_lowercase()))
                .cloned()
                .collect();

            if !matches.is_empty() {
                self.filtered_entries = matches;
            } else {
                let words: Vec<&str> = self.search_query.split_whitespace().collect();
                let mut found = false;

                for i in (1..words.len()).rev() {
                    let sub_query = words[0..i].join(" ");
                    let sub_query_lower = sub_query.to_lowercase();

                    let sub_matches: Vec<AppEntry> = self
                        .entries
                        .iter()
                        .filter(|e| fuzzy_match(&sub_query_lower, &e.name.to_lowercase()))
                        .cloned()
                        .collect();

                    if !sub_matches.is_empty() {
                        self.filtered_entries = sub_matches;
                        let args: Vec<String> = words[i..].iter().map(|s| s.to_string()).collect();
                        if let Some(last_arg) = args.last() {
                            self.filtered_files = list_files(last_arg);
                            self.mode = AppMode::FileSelection;
                        }
                        self.launch_args = Some(args);
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

    pub fn auto_complete(&mut self) {
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
                        self.update_filter();
                    }
                }
            }
        }
    }

    pub fn launch_selected(&mut self) {
        if let Some(i) = self.list_state.selected() {
            let app_entry = if self.mode == AppMode::FileSelection {
                self.filtered_entries.first()
            } else {
                self.filtered_entries.get(i)
            };

            if let Some(entry) = app_entry {
                if let Some((cmd, args)) = entry.exec_args.split_first() {
                    let mut command = Command::new(cmd);

                    let mut final_args = Vec::new();

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

                    command
                        .args(final_args)
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
                                Some(format!("Failed to launch {}: {}", entry.name, err));
                        }
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

fn fuzzy_match(query: &str, target: &str) -> bool {
    let mut query_chars = query.chars();
    let mut matcher = query_chars.next();

    if matcher.is_none() {
        return true;
    }

    for t in target.chars() {
        if let Some(q) = matcher {
            if t == q {
                matcher = query_chars.next();
                if matcher.is_none() {
                    return true;
                }
            }
        }
    }
    false
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

fn list_files(query_path: &str) -> Vec<String> {
    let expanded = expand_tilde(query_path);
    let path = Path::new(&expanded);
    
    let (dir, file_prefix) = if query_path.ends_with('/') {
        (path, "")
    } else {
        (path.parent().unwrap_or(Path::new("")), path.file_name().and_then(|s| s.to_str()).unwrap_or(""))
    };

    let search_dir = if dir.as_os_str().is_empty() {
        if query_path.starts_with('/') { Path::new("/") } else { Path::new(".") }
    } else {
        dir
    };

    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(search_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') && !file_prefix.starts_with('.') {
                continue;
            }
            if fuzzy_match(file_prefix, name) {
                let mut path_str = path.to_string_lossy().to_string();
                if query_path.starts_with('~') {
                    if let Some(home) = dirs::home_dir() {
                        let home_str = home.to_string_lossy();
                        if path_str.starts_with(home_str.as_ref()) {
                            path_str = format!("~{}", &path_str[home_str.len()..]);
                        }
                    }
                }
                files.push(path_str);
            }
        }
    }
    files.sort();
    files
}
