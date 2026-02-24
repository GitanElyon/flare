# Flare Extension API (Light Reference)

This project uses a runtime extension registry. Every extension implements `FlareExtension` and returns an `ExtensionResult` for the current query.

## Core Trait

Defined in [src/extensions/api.rs](src/extensions/api.rs).

- `metadata(&self, config: &AppConfig) -> ExtensionMetadata`
- `should_handle(&self, query: &str, config: &AppConfig) -> bool`
- `process(&self, query: &str, config: &AppConfig, registry: &ExtensionRegistry) -> ExtensionResult`

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
- `Help(Vec<HelpCommand>)`
  - Help/command menu entries.
- `None`
  - No match or no output.

## List Actions

`ExtensionListAction`:

- `CopyToClipboardAndExit`
- `None`

The app executes this action generically. This keeps module-specific launch logic out of the app event loop.

## Built-in vs External Extensions

- Built-in extensions are registered in [src/extensions/mod.rs](src/extensions/mod.rs).
- External executable extensions are discovered in `~/.config/flare/extensions/` and queried using:
  - `--info` (JSON metadata)
  - `--query <text>` (result payload)

External extension outputs are currently treated as single-result text. If you want typed list/actions for external extensions too, extend the external protocol and parser in the registry.
