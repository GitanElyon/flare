use crate::config::AppConfig;
use super::api::{ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult, FlareExtension};

pub struct HelpExtension;

impl FlareExtension for HelpExtension {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, config: &AppConfig) -> bool {
        should_handle(query, config)
    }

    fn process(&self, _query: &str, config: &AppConfig, registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult {
        let items = available_commands(config, registry)
            .into_iter()
            .map(|cmd| ExtensionListItem { action: None,
                title: format!("{:10}  {:10}  {}", cmd.trigger, cmd.name, cmd.description),
                value: cmd.query_example,
            })
            .collect();
        ExtensionResult::List {
            title: " Commands ".to_string(),
            items,
            action: ExtensionListAction::SetSearchQuery,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HelpCommand {
    pub name: String,
    pub trigger: String,
    pub description: String,
    pub query_example: String,
}

pub fn metadata(config: &AppConfig) -> ExtensionMetadata {
    ExtensionMetadata {
        name: "Help".to_string(),
        description: "Show this help menu".to_string(),
        trigger: trigger(config).to_string(),
        query_example: None,
    }
}

pub fn trigger(config: &AppConfig) -> &str {
    &config.features.help_search_trigger
}

pub fn should_handle(query: &str, config: &AppConfig) -> bool {
    query.trim() == trigger(config)
}

pub fn available_commands(config: &AppConfig, registry: &crate::extensions::ExtensionRegistry) -> Vec<HelpCommand> {
    let mut commands = Vec::new();

    for ext in &registry.extensions {
        let meta = ext.metadata(config);
        let query_example = meta.query_example.unwrap_or_else(|| meta.trigger.clone());
        commands.push(HelpCommand {
            name: meta.name,
            trigger: meta.trigger,
            description: meta.description,
            query_example,
        });
    }

    commands
}
