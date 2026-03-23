# Flare Core Docs

`core` is the base Flare launcher runtime.

Plugin packs (scripts, aliases, community catalog) are documented in:

- https://github.com/gitanelyon/awesome-flare

## Config files

Flare reads launcher settings from:

- `~/.config/flare/config.toml`
  - UI + launcher behavior.

Script integration uses:

- `~/.config/flare/scripts/`
  - executable `*.sh` scripts discovered dynamically.
- `~/.config/flare/alias.toml`
  - optional trigger aliases for script names.

`config.toml` is created automatically on first run.

## Important defaults

From `[features]` in `config.toml`:

- `enable-file-explorer = true`
- `enable-launch-args = true`
- `enable-auto-complete = true`
- `dirs-first = true`
- `show-duplicates = false`
- `recent-first = true`

## File explorer behavior

With file explorer enabled (default), typing a path query enters file-selection mode:

- Absolute path: `/...`
- Home path: `~/...`
- Relative path: `./...` or `../...`

Behavior:

- `Tab` autocompletes selected path.
- `Enter` on directories keeps browsing.
- `Enter` on files opens via `xdg-open`.
- Executable files can be executed directly.

## Keybindings

- `Up/Down`: move selection
- `Left/Right`: move input cursor
- `Tab`: autocomplete
- `Alt+f`: favorite/unfavorite app
- `Alt+Up`: jump to first item
- `Alt+Down`: jump to last item
- `Enter`: launch/open selected item
- `Esc`: quit

## Plugin integration notes

- Flare core is host/runtime.
- Script plugins live in `~/.config/flare/scripts/`.
- Protocol, directives, and setup guidance are in `https://github.com/gitanelyon/awesome-flare`.

## XDG app scan paths

Flare discovers `.desktop` entries from standard XDG locations including:

- `/usr/share/applications`
- `/usr/local/share/applications`
- `~/.local/share/applications`