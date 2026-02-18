use std::{
    fs,
    path::Path,
};
use crate::config::AppConfig;
use super::api::{ExtensionMetadata, FlareExtension, ExtensionResult};

pub struct Files;

impl FlareExtension for Files {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        should_handle_path_query(query)
    }

    fn process(&self, query: &str, config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        let files = list_files(query, config.features.dirs_first);
        ExtensionResult::Files(files)
    }
}

pub fn metadata(_config: &AppConfig) -> ExtensionMetadata {
    ExtensionMetadata {
        name: "Files".to_string(),
        description: "Browse files and directories".to_string(),
        trigger: "~/ or /".to_string(),
    }
}

pub fn should_handle_path_query(query: &str) -> bool {
    query.starts_with('~') || query.starts_with('/')
}

pub fn expand_tilde(path: &str) -> String {
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

pub fn list_files(query_path: &str, dirs_first: bool) -> Vec<String> {
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

            if let Some(score) = crate::app::fuzzy_score(file_prefix, name) {
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
