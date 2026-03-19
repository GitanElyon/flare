# Flare

Flare is a terminal-first Linux application launcher built with Rust + Ratatui.

## Highlights

- Fast `.desktop` app scanning and fuzzy search.
- Usage/favorites-based ordering.
- Launch arguments support.
- File explorer mode enabled by default.
- Keyboard-first navigation and customization.

## Plugin model

Flare core is the host runtime. Plugins are script-based and live in `~/.config/flare/scripts/`. Plugins can define custom triggers, query handling, and output formatting via a simple line-oriented protocol. 

Scripts can be executable files (any language) or extension-based scripts run through supported interpreters (`.sh`, `.bash`, `.zsh`, `.fish`, `.py`, `.pl`, `.rb`, `.js`, `.lua`).

The plugin ecosystem is cataloged in `awesome-flare`:
- https://github.com/gitanelyon/awesome-flare

## Install

```bash
git clone https://github.com/GitanElyon/flare.git
cd flare/core
cargo install --locked --path .
```

Nix users can install via:
```bash
nix profile install "github:GitanElyon/flare"
```

## Run

```bash
flare
```

## Keybindings

- `Up`/`Down`: move selection
- `Left`/`Right`: move cursor in input
- `Tab`: autocomplete path
- `Enter`: launch/open selected item
- `Esc`: quit
- `Alt+f`: toggle favorite

## Config files

- `~/.config/flare/config.toml`
- `~/.config/flare/scripts/alias.toml` (optional script trigger aliases)

See [DOCS.md](DOCS.md) for full configuration details.
