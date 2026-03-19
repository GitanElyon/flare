# Flare Core API

This document describes the base Flare launcher internals (`core`) and how plugin packs integrate with it.

Plugin scripts and community catalog are documented in `plugins/` and in:

- https://github.com/gitanelyon/awesome-flare

## Core modules

- `src/main.rs`
  - Terminal lifecycle, event loop, key handling.
- `src/app.rs`
  - App state machine, filtering, file explorer, launch argument handling, command spawning.
- `src/ui.rs`
  - Rendering and list presentation.
- `src/config.rs`
  - `config.toml` loading and defaults.
- `src/history.rs`
  - App usage/favorites history persistence.

## Configuration surface

Flare reads:

- `~/.config/flare/config.toml` (UI + launcher behavior)
- `~/.config/flare/scripts/alias.toml` (optional script/trigger mapping)

Important toggles in `features`:

- `enable-file-explorer` (default: `true`)
- `enable-launch-args` (default: `true`)
- `enable-auto-complete` (default: `true`)
- `dirs-first` (default: `true`)

## File explorer behavior

Implemented in `App::update_filter` + `App::list_completions`:

- Path queries beginning with `/`, `~/`, `./`, or `../` enter file-selection mode.
- Tab completion in file-selection mode inserts selected paths.
- If selected path is executable, Flare runs it directly; otherwise it opens via `xdg-open`.

## Plugin pack integration

`core` is the host launcher.

- Plugin scripts are expected under `~/.config/flare/scripts/`.
- Script aliases are loaded from `~/.config/flare/scripts/alias.toml`.

For plugin implementation details and curated plugins, use the `awesome-flare` repo.
