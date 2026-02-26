use crate::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionMetadata {
    pub name: String,
    pub description: String,
    pub trigger: String,
    /// An example query to populate the search bar when selected from Help.
    /// Falls back to `trigger` if not set.
    #[serde(default)]
    pub query_example: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ExtensionListItem {
    pub title: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionListAction {
    CopyToClipboardAndExit,
    /// Set the search query to the selected item's value.
    SetSearchQuery,
    /// Append the selected item's value to the current search query.
    AppendToQuery,
    /// Execute the item's value as a shell command via `sh -c`, then exit.
    ExecuteAndExit,
    /// Execute the item's value as a shell command (blocking), then refresh the view.
    ExecuteAndRefresh,
    None,
}

// Notes for extension authors:
// - `ExecuteAndExit` means the UI will spawn the command and then quit immediately.
//   Use this when the intent is to run a side-effecting shell command and close the launcher.
// - `ExecuteAndRefresh` means the UI will run the command (blocking) and then call
//   `update_filter()` / refresh the extension list. Extensions that change system state
//   (for example, volume changes or device switching) should use `ExecuteAndRefresh` so
//   the UI can re-query and display the updated state without losing focus.
// - `CopyToClipboardAndExit`, `SetSearchQuery` and `AppendToQuery` are handled by the
//   host; extension authors can return these to indicate the desired interaction.

#[derive(Debug, Clone)]
pub enum AuthResult {
    Success,
    AuthFailed,
    LaunchError(String),
}

#[derive(Debug, Clone)]
pub enum ExtensionResult {
    Single { query: String, result: String },
    List {
        title: String,
        items: Vec<ExtensionListItem>,
        action: ExtensionListAction,
    },
    Files(Vec<String>),
    None,
}

pub trait FlareExtension: Send + Sync {
    fn metadata(&self, config: &AppConfig) -> ExtensionMetadata;
    fn should_handle(&self, query: &str, config: &AppConfig) -> bool;
    fn process(&self, query: &str, config: &AppConfig, registry: &crate::extensions::ExtensionRegistry) -> ExtensionResult;

    /// Strip a query prefix and return `(stripped_query, prefix_args)`.
    /// Used by extensions like sudo that prepend themselves to other queries.
    /// Return `None` if this extension does not use a query prefix.
    fn strip_prefix(&self, _query: &str, _config: &AppConfig) -> Option<(String, Vec<String>)> {
        None
    }

    /// Return `true` if launching this query requires authentication.
    fn requires_auth(&self, _query: &str, _config: &AppConfig) -> bool {
        false
    }

    /// Expand a path string (e.g. tilde expansion). Return `None` to pass through unchanged.
    fn expand_path(&self, _path: &str) -> Option<String> {
        None
    }

    /// Authenticate and launch a command. Called when `requires_auth` returns `true`.
    fn authenticate_and_launch(
        &self,
        _password: &str,
        _cmd: &str,
        _args: &[String],
        _prefix_args: &[String],
    ) -> AuthResult {
        AuthResult::LaunchError("Authentication not implemented".to_string())
    }
}
