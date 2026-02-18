use crate::config::AppConfig;
use super::api::ExtensionMetadata;

#[derive(Debug, Clone)]
pub struct ParsedSudoQuery {
    pub query: String,
    pub sudo_args: Vec<String>,
}

pub struct Sudo;

impl crate::extensions::FlareExtension for Sudo {
    fn metadata(&self, config: &AppConfig) -> crate::extensions::ExtensionMetadata {
        metadata(config)
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with("sudo")
    }

    fn process(&self, _query: &str, _config: &AppConfig, _registry: &crate::extensions::ExtensionRegistry) -> crate::extensions::ExtensionResult {
        // Sudo is still a special case in app.rs for now
        crate::extensions::ExtensionResult::None
    }
}

pub fn metadata(_config: &AppConfig) -> crate::extensions::ExtensionMetadata {
    ExtensionMetadata {
        name: "Sudo".to_string(),
        description: "Run commands with sudo privileges".to_string(),
        trigger: "sudo".to_string(),
    }
}

pub fn parse_query(search_query: &str) -> ParsedSudoQuery {
    if !search_query.starts_with("sudo") {
        return ParsedSudoQuery {
            query: search_query.to_string(),
            sudo_args: Vec::new(),
        };
    }

    let parts: Vec<&str> = search_query.split_whitespace().collect();
    let mut idx = 1usize;
    let mut sudo_args = Vec::new();

    if parts.first() == Some(&"sudo") {
        while idx < parts.len() {
            let part = parts[idx];
            if part.starts_with('-') {
                sudo_args.push(part.to_string());
                if ["-C", "-g", "-h", "-p", "-r", "-t", "-U", "-u"].contains(&part) {
                    if idx + 1 < parts.len() {
                        idx += 1;
                        sudo_args.push(parts[idx].to_string());
                    }
                }
            } else {
                break;
            }
            idx += 1;
        }
    }

    let query = if idx < parts.len() {
        parts[idx..].join(" ")
    } else {
        String::new()
    };

    ParsedSudoQuery {
        query,
        sudo_args,
    }
}
