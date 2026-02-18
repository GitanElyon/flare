use crate::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub name: String,
    pub description: String,
    pub trigger: String,
}

#[derive(Debug, Clone)]
pub enum ExtensionResult {
    Single { query: String, result: String },
    List(Vec<(&'static str, &'static str)>),
    Files(Vec<String>),
    Help(Vec<crate::extensions::help::HelpCommand>),
    None,
}

pub trait FlareExtension: Send + Sync {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata;
    fn should_handle(&self, query: &str, config: &AppConfig) -> bool;
    fn process(&self, query: &str, config: &AppConfig, registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult;
}
