use crate::{
    config::AppConfig,
    history::History,
};
use std::sync::LazyLock;
use super::api::{
    ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension,
};

pub struct Symbols;

impl FlareExtension for Symbols {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        crate::extensions::symbols::should_handle(query, config)
    }

    fn process(&self, query: &str, config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        let history = History::load();
        let symbols = filter_symbols(query, config, &history)
            .into_iter()
            .map(|(name, symbol, is_favorite)| {
                let prefix = if is_favorite { "★ " } else { "" };
                ExtensionListItem { action: None,
                    title: format!("{}{} {}", prefix, symbol, name),
                    value: symbol,
                }
            })
            .collect();

        ExtensionResult::List {
            title: "Symbols".to_string(),
            items: symbols,
            action: ExtensionListAction::CopyToClipboardAndExit,
        }
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
        query_example: None,
    }
}

pub fn should_handle(query: &str, config: &AppConfig) -> bool {
    query.starts_with(&config.features.symbol_search_trigger)
}

pub fn filter_symbols(
    query: &str,
    config: &AppConfig,
    history: &History,
) -> Vec<(String, String, bool)> {
    let query = query
        .strip_prefix(&config.features.symbol_search_trigger)
        .unwrap_or("")
        .trim()
        .to_lowercase();

    if query.is_empty() {
        let mut symbols: Vec<(String, String, bool)> = SYMBOLS
            .iter()
            .map(|(a, b)| {
                let name = a.to_string();
                let favorite = history.is_favorite_symbol(&name);
                (name, b.to_string(), favorite)
            })
            .collect();
        symbols.sort_by(|(name_a, _, fav_a), (name_b, _, fav_b)| {
            if fav_a != fav_b {
                return fav_b.cmp(&fav_a);
            }
            name_a.cmp(name_b)
        });
        return symbols;
    }

    let mut scored_symbols: Vec<(i64, (String, String, bool))> = SYMBOLS
        .iter()
        .filter_map(|&(name, symbol)| {
            let name_lower = name.to_lowercase();
            if name_lower.contains(&query) {
                let score = if name_lower.starts_with(&query) { 100 } else { 0 };
                let name = name.to_string();
                Some((score, (name.clone(), symbol.to_string(), history.is_favorite_symbol(&name))))
            } else {
                None
            }
        })
        .collect();

    scored_symbols.sort_by(|(score_a, (name_a, _, fav_a)), (score_b, (name_b, _, fav_b))| {
        if fav_a != fav_b {
            return fav_b.cmp(&fav_a);
        }
        score_b.cmp(score_a).then_with(|| name_a.cmp(name_b))
    });

    scored_symbols.into_iter().map(|(_, symbol)| symbol).collect()
}
