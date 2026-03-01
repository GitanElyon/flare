use crate::{
    config::AppConfig,
    history::ClipboardHistory,
};
use arboard::Clipboard;
use std::process::Command;
use super::api::{
    ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension,
};

pub struct ClipboardExt;

impl FlareExtension for ClipboardExt {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "Clipboard".to_string(),
            description: "View and search clipboard history (+)".to_string(),
            trigger: config.features.clipboard_search_trigger.clone(),
            query_example: None,
        }
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        query.starts_with(&config.features.clipboard_search_trigger)
    }

    fn process(&self, query: &str, config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        let entries = resolve_clipboard_entries(config.features.clipboard_prefer_external_history_tools);
        let items = filter_clipboard(query, config, &entries)
            .into_iter()
            .map(|text| {
                ExtensionListItem { action: None,
                    title: single_line_preview(&text),
                    value: text,
                }
            })
            .collect();

        ExtensionResult::List {
            title: "Clipboard History".to_string(),
            items,
            action: ExtensionListAction::CopyToClipboardAndExit,
        }
    }
}

pub fn filter_clipboard(
    query: &str,
    config: &AppConfig,
    entries: &[String],
) -> Vec<String> {
    let query = query
        .strip_prefix(&config.features.clipboard_search_trigger)
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if query.is_empty() {
        return entries.to_vec();
    }

    entries
        .iter()
        .filter(|text| text.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

fn resolve_clipboard_entries(prefer_external_tools: bool) -> Vec<String> {
    if prefer_external_tools {
        if let Some(entries) = load_external_history() {
            if !entries.is_empty() {
                return dedupe_limit(entries, 200);
            }
        }
    }

    let mut history = ClipboardHistory::load();
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(text) = clipboard.get_text() {
            history.add(text);
        }
    }

    dedupe_limit(history.entries, 200)
}

fn load_external_history() -> Option<Vec<String>> {
    let attempts: [(&str, &[&str]); 5] = [
        ("wl-clipboard-history", &["list"]),
        ("cliphist", &["list"]),
        ("xclip", &["-o"]),
        ("xsel", &["--clipboard", "--output"]),
        ("copyq", &["tab", "clipboard", "read"]),
    ];

    for (bin, args) in attempts {
        if !command_exists(bin) {
            continue;
        }

        if let Ok(output) = Command::new(bin).args(args).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<String> = stdout
                    .lines()
                    .map(|line| normalize_line(line))
                    .filter(|line| !line.is_empty())
                    .collect();
                if !lines.is_empty() {
                    return Some(lines);
                }
            }
        }
    }

    None
}

fn command_exists(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn normalize_line(line: &str) -> String {
    let trimmed = line.trim();
    if let Some((_, value)) = trimmed.split_once('\t') {
        return value.trim().to_string();
    }
    trimmed.to_string()
}

fn dedupe_limit(entries: Vec<String>, max: usize) -> Vec<String> {
    let mut output = Vec::new();
    for text in entries {
        let text = text.trim().to_string();
        if text.is_empty() {
            continue;
        }
        if !output.iter().any(|existing| existing == &text) {
            output.push(text);
        }
        if output.len() >= max {
            break;
        }
    }
    output
}

fn single_line_preview(text: &str) -> String {
    text.replace('\n', " ").chars().take(120).collect::<String>()
}

