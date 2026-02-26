# Flare Extension API (Light Reference)

This project uses a runtime extension registry. Every extension implements `FlareExtension` and returns an `ExtensionResult` for the current query.

## Core Trait

Defined in [src/extensions/api.rs](src/extensions/api.rs).

- `metadata(&self, config: &AppConfig) -> ExtensionMetadata`
- `should_handle(&self, query: &str, config: &AppConfig) -> bool`
- `process(&self, query: &str, config: &AppConfig, registry: &ExtensionRegistry) -> ExtensionResult`

Flare also exposes several optional/default methods that extensions may implement to integrate more tightly with the host:

- `strip_prefix(&self, query: &str, config: &AppConfig) -> Option<(String, Vec<String>)>` — return a `(stripped_query, prefix_args)` pair when an extension claims a prefix (for example `sudo ...`), otherwise `None`.
- `requires_auth(&self, query: &str, config: &AppConfig) -> bool` — return `true` when a query requires authentication; the host will enter an authentication mode and may delegate launching back to the extension.
- `expand_path(&self, path: &str) -> Option<String>` — optional path-expansion helper (tilde expansion, etc.); return `None` to use the host default.
- `authenticate_and_launch(&self, password: &str, cmd: &str, args: &[String], prefix_args: &[String]) -> AuthResult` — called by the host to authenticate and launch privileged commands; returns `AuthResult::Success`, `AuthResult::AuthFailed` or `AuthResult::LaunchError(String)`.

## Result Types

`ExtensionResult` currently supports:

- `Single { query, result }`
  - Used for single-value views (for example calculator output).
- `List { title, items, action }`
  - Generic extension list view.
  - `items` is `Vec<ExtensionListItem { title, value }>`.
  - `action` tells the app what to do when the user presses Enter.
- `Files(Vec<String>)`
  - File-selection mode list.
- `None`
  - No match or no output.

Note: help/command lists are implemented as regular extensions that return a `List` (there is no special `Help` variant any more).

## List Actions

`ExtensionListAction`:

- `CopyToClipboardAndExit`
- `SetSearchQuery` — set the search query to the selected item's `value` and rerun filtering
- `AppendToQuery` — append the selected item's `value` to the current search query
- `None`

The app executes the `action` returned by an extension generically; this keeps module-specific launch logic out of the app event loop and lets extensions express intent declaratively.

## Built-in vs External Extensions

- Built-in extensions are registered in [src/extensions/mod.rs](src/extensions/mod.rs).
- External executable extensions are discovered in `~/.config/flare/extensions/` and queried using:
  - `--info` (JSON metadata)
  - `--query <text>` (result payload)
- The `--info` JSON should at minimum include `name`, `description`, and `trigger`. It may also include an optional `query_example` field that helps populate the search bar when the extension is chosen from a list.

External extension outputs are treated as single-result text by default. If you want typed list/actions for external extensions, return a small structured payload from `--query` and update the registry parser accordingly (the registry supports `Files` and `List` result shapes for built-in extensions).
