use crate::{
    config::AppConfig,
    history::History,
};
use arboard::Clipboard;
use std::process::{Command, Stdio};
use std::sync::LazyLock;
use super::api::{ExtensionMetadata, FlareExtension, ExtensionResult};

pub struct Symbols;

impl FlareExtension for Symbols {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        crate::extensions::symbols::should_handle(query, config)
    }

    fn process(&self, _query: &str, _config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        // We'll pass an empty history for the generic process for now, 
        // as history is managed by the App struct.
        // In a more mature plugin system, history might be a shared resource.
        ExtensionResult::List(Vec::new())
    }
}

pub static SYMBOLS: LazyLock<Vec<(&'static str, &'static str)>> = LazyLock::new(|| {
    let json = include_str!("../../assets/symbols.json");
    let data: Vec<(String, String)> = serde_json::from_str(json).expect("Failed to parse symbols.json");
    data.into_iter()
        .map(|(a, b)| (
            Box::leak(a.into_boxed_str()) as &'static str,
            Box::leak(b.into_boxed_str()) as &'static str
        ))
        .collect()
});

pub fn metadata(config: &AppConfig) -> ExtensionMetadata {
    ExtensionMetadata {
        name: "Symbols".to_string(),
        description: "Search and copy Nerd Font icons/symbols".to_string(),
        trigger: config.features.symbol_search_trigger.clone(),
    }
}

pub fn should_handle(query: &str, config: &AppConfig) -> bool {
    query.starts_with(&config.features.symbol_search_trigger)
}

pub fn filter_symbols(
    query: &str,
    config: &AppConfig,
    history: &History,
) -> Vec<(&'static str, &'static str)> {
    let query = query
        .strip_prefix(&config.features.symbol_search_trigger)
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if query.is_empty() {
        let mut symbols: Vec<(&'static str, &'static str)> =
            SYMBOLS.iter().cloned().collect();
        symbols.sort_by(|(name_a, _), (name_b, _)| {
            let fav_a = history.is_favorite_symbol(name_a);
            let fav_b = history.is_favorite_symbol(name_b);
            if fav_a != fav_b {
                return fav_b.cmp(&fav_a);
            }
            name_a.cmp(name_b)
        });
        return symbols;
    }

    let mut scored_symbols: Vec<(i64, (&'static str, &'static str))> = SYMBOLS
        .iter()
        .filter_map(|&(name, symbol)| {
            let name_lower = name.to_lowercase();
            if name_lower.contains(&query) {
                let score = if name_lower.starts_with(&query) { 100 } else { 0 };
                Some((score, (name, symbol)))
            } else {
                None
            }
        })
        .collect();

    scored_symbols.sort_by(|(score_a, (name_a, _)), (score_b, (name_b, _))| {
        let fav_a = history.is_favorite_symbol(name_a);
        let fav_b = history.is_favorite_symbol(name_b);
        if fav_a != fav_b {
            return fav_b.cmp(&fav_a);
        }
        score_b.cmp(score_a).then_with(|| name_a.cmp(name_b))
    });

    scored_symbols.into_iter().map(|(_, symbol)| symbol).collect()
}

pub fn copy_to_clipboard(symbol: &str) {
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(symbol.to_string());
    }
}

pub fn copy_to_clipboard_with_notify(symbol: &str) {
    copy_to_clipboard(symbol);
    let _ = Command::new("notify-send")
        .arg("Flare")
        .arg(format!("Copied {} to clipboard", symbol))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}
