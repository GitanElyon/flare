pub mod api;
pub mod calculator;
pub mod clipboard;
pub mod files;
pub mod help;
pub mod sudo;
pub mod symbols;

pub use api::{
    AuthResult, ExtensionListAction, ExtensionListItem, ExtensionMetadata, ExtensionResult,
    FlareExtension,
};
use crate::config::AppConfig;
use std::fs;
use std::process::Command;

pub struct ExtensionRegistry {
    pub extensions: Vec<Box<dyn FlareExtension>>,
}

pub struct ExternalExtension {
    pub path: String,
    pub cached_metadata: ExtensionMetadata,
}

impl FlareExtension for ExternalExtension {
    fn metadata(&self, _config: &AppConfig) -> ExtensionMetadata {
        self.cached_metadata.clone()
    }

    fn should_handle(&self, query: &str, _config: &AppConfig) -> bool {
        query.starts_with(&self.cached_metadata.trigger)
    }

    fn process(&self, query: &str, _config: &AppConfig, _registry: &ExtensionRegistry) -> ExtensionResult {
        let output = Command::new(&self.path)
            .arg("--query")
            .arg(query)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
                ExtensionResult::Single { 
                    query: query.to_string(), 
                    result 
                }
            }
            _ => ExtensionResult::None,
        }
    }
}

impl ExtensionRegistry {
    pub fn new(config: &AppConfig) -> Self {
        let mut extensions: Vec<Box<dyn FlareExtension>> = Vec::new();

        if config.extensions.is_enabled("help") {
            extensions.push(Box::new(help::HelpExtension));
        }

        if config.extensions.is_enabled("calculator") {
            extensions.push(Box::new(calculator::Calculator));
        }

        if config.extensions.is_enabled("symbols") {
            extensions.push(Box::new(symbols::Symbols));
        }

        if config.extensions.is_enabled("clipboard") {
            extensions.push(Box::new(clipboard::ClipboardExt));
        }

        if config.extensions.is_enabled("files") {
            extensions.push(Box::new(files::Files));
        }

        if config.extensions.is_enabled("sudo") {
            extensions.push(Box::new(sudo::Sudo));
        }

        // Load external plugins
        if let Some(mut path) = dirs::config_dir() {
            path.push("flare");
            path.push("extensions");
            
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let path_str = path.to_string_lossy().to_string();
                        // Try to get metadata
                        let output = Command::new(&path_str).arg("--info").output();
                        if let Ok(out) = output {
                            if out.status.success() {
                                if let Ok(metadata) = serde_json::from_slice::<ExtensionMetadata>(&out.stdout) {
                                    extensions.push(Box::new(ExternalExtension {
                                        path: path_str,
                                        cached_metadata: metadata,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        Self { extensions }
    }

    /// Expand a path by delegating to any extension that handles path expansion.
    pub fn expand_path(&self, path: &str) -> String {
        for ext in &self.extensions {
            if let Some(expanded) = ext.expand_path(path) {
                return expanded;
            }
        }
        path.to_string()
    }

    /// List file/directory completions for a query path by delegating to extensions.
    pub fn list_completions(&self, query: &str, config: &AppConfig) -> Vec<String> {
        for ext in &self.extensions {
            if ext.should_handle(query, config) {
                if let ExtensionResult::Files(files) = ext.process(query, config, self) {
                    return files;
                }
            }
        }
        Vec::new()
    }

    /// Strip any query prefix (e.g. "sudo") and return `(stripped_query, prefix_args)`.
    pub fn preprocess_query(&self, query: &str, config: &AppConfig) -> (String, Vec<String>) {
        for ext in &self.extensions {
            if let Some((stripped, prefix_args)) = ext.strip_prefix(query, config) {
                return (stripped, prefix_args);
            }
        }
        (query.to_string(), Vec::new())
    }

    /// Returns `true` if any loaded extension requires authentication for the given query.
    pub fn requires_auth(&self, query: &str, config: &AppConfig) -> bool {
        self.extensions.iter().any(|ext| ext.requires_auth(query, config))
    }

    /// Authenticate and launch via the first extension that claims the query.
    pub fn authenticate_and_launch(
        &self,
        password: &str,
        cmd: &str,
        args: &[String],
        prefix_args: &[String],
        query: &str,
        config: &AppConfig,
    ) -> api::AuthResult {
        for ext in &self.extensions {
            if ext.requires_auth(query, config) {
                return ext.authenticate_and_launch(password, cmd, args, prefix_args);
            }
        }
        api::AuthResult::LaunchError("No authentication extension available".to_string())
    }
}
