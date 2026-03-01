use crate::config::AppConfig;
use super::api::{ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension};

pub struct Runner;

impl FlareExtension for Runner {
    fn metadata(&self, _config: &AppConfig) -> ExtensionMetadata {
        ExtensionMetadata {
            name: "Runner".to_string(),
            description: "Execute a shell command".to_string(),
            trigger: ">".to_string(),
            query_example: Some("> echo Hello".to_string()),
        }
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with('>')
    }

    fn process(&self, query: &str, _config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        let cmd = query.strip_prefix('>').unwrap_or("").trim().to_string();

        let (items, action) = if cmd.is_empty() {
            (
                vec![ExtensionListItem { action: None,
                    title: "  Type a command to run\u{2026}".to_string(),
                    value: String::new(),
                }],
                ExtensionListAction::None,
            )
        } else {
            (
                vec![ExtensionListItem { action: None,
                    title: format!("  \u{25b6}  Run: {}", cmd),
                    value: cmd,
                }],
                ExtensionListAction::ExecuteAndExit,
            )
        };

        ExtensionResult::List {
            title: " Run Command ".to_string(),
            items,
            action,
        }
    }
}
