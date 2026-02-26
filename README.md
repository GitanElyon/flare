# Overview

Flare is a customizable, lightweight, terminal-based application launcher for Linux. Built with Rust and Ratatui, it combines the visual list style of tools like `rofi` or `wofi` with the simplicity and speed of `dmenu`.

# Features

- **Fast scanning**: Automatically detects applications from standard `.desktop` file locations (`/usr/share/applications`, `~/.local/share/applications`).
- **Smart ordering**: Sorts applications by usage frequency, keeping your most used apps at the top.
- **TUI interface**: Clean, terminal-based user interface.
- **Instant filtering**: Real-time search filtering as you type.
- **Plugin-based extensions**: Enable features from `~/.config/flare/extention_config.toml`.
- **File Explorer plugin**: Browse and select files directly. Start with `~/` or `/` to search files exclusively, or type a path after an app name to pass it as an argument.
- **Symbol Search plugin**: Type `.` to search through Nerd Font symbols. Select one to copy it to the clipboard.
- **Launch Arguments**: Pass arguments to applications (e.g., `nvim ~/file.txt`).
- **Calculator plugin**: Type `=` to evaluate expressions and use history.
- **Clipboard History plugin**: Type `+` to browse your recent clipboard entries. Select one to copy it back to the clipboard. Flare can read from installed clipboard history tools (for example `wl-clipboard-history` or `cliphist`) and falls back to its local history file when those are unavailable.
 - **Sudo plugin**: Launch applications with elevated privileges (e.g., `sudo gparted`). Includes a secure, terminal-style password prompt (implemented as an extension that requests authentication).
 - **Help plugin**: Type `-` to list available extension commands; the help view is now provided by the help extension as a regular extension list (no special-case in the app core).
- **Keyboard-centric**: Designed for efficiency with intuitive keybindings.
- **Highly customizable**: Extensive configuration options for appearance and behavior.

# Runtime Extensions & Customization

Flare is designed to be fast and lightweight while being extensible at runtime. Instead of requiring compile-time feature flags, Flare supports a runtime plugin system:

 - Built-in extensions (Calculator, Symbols, Files, Sudo, Help, Clipboard, Runner, Volume) are shipped with the binary and activate based on `~/.config/flare/extention_config.toml`.
- External extensions can be added without recompiling: drop an executable into `~/.config/flare/extensions/` and Flare will detect it on startup.

A plugin binary should implement two simple interfaces the launcher expects:

- `--info`
	* Should print a JSON object with the plugin metadata: `name`, `description`, and `trigger`.
	* Example output:

```json
{ "name": "My Plugin", "description": "Does something cool", "trigger": "!" }
```

- `--query <text>`
	* Called when the user types a query beginning with the plugin's trigger (or any other trigger semantics the plugin chooses).
	* Should write its result(s) to stdout. For simple single-result plugins, just print the answer. For richer integrations you can return a newline-separated list or any format your companion code expects.

Example minimal Bash plugin (save as `~/.config/flare/extensions/hello` and make executable):

```bash
#!/usr/bin/env bash
if [ "$1" = "--info" ]; then
	echo '{"name":"Hello","description":"Greets the user","trigger":"!"}'
	exit 0
fi

if [ "$1" = "--query" ]; then
	shift
	echo "Hello, $*"
	exit 0
fi

echo ""
```

After you place the executable, restart Flare; the plugin will show up in the runtime extension registry and respond to its trigger. You can test it manually:

```bash
~/.config/flare/extensions/hello --info
~/.config/flare/extensions/hello --query world
```

Notes:
- Flare provides internal utilities (history, clipboard helpers, file expansion) used by built-in extensions. External plugins are standalone processes and communicate via `--info` / `--query`.
- The `--info` JSON object may include an optional `query_example` field in addition to `name`, `description`, and `trigger`. `query_example` helps populate the search bar when an extension's command is selected from a list.

Built-in runtime extensions (recent additions)

- Runner (`>`): prefix a query with `>` to run an arbitrary shell command (for example `> echo hello`). Selecting the item executes the command via `sh -c` and Flare exits.
- Volume (`v!`): control system audio. Type `v!` to open the volume menu, or `v! -h` to show available commands. Supported operations include `v! +N`, `v! -N`, `v! N`, `v! mute`, and `v! devices`. Flare auto-detects available backends (`wpctl` for PipeWire, `pactl` for PulseAudio, or `amixer` for ALSA) and issues the corresponding commands.
- The host understands several extension result shapes: single-result text, file lists, and structured lists where each item can include a display title and a value. If you need list+action semantics for an external plugin, return a structured output and update the registry parser accordingly.
- Keep your plugin fast and stdout-friendly; Flare runs the plugin synchronously while evaluating the query.

# Installation

Ensure you have a recent stable Rust toolchain (1.77+) installed via [rustup](https://rustup.rs/).

### Quick install (recommended)

```bash
git clone https://github.com/GitanElyon/flare.git
cd flare
cargo install --locked --path .
```

This puts the `flare` binary in `~/.cargo/bin`, which should be on your `$PATH` to run it, and already will be if you installed Rust via rustup. Update to the latest commit any time with:

```bash
cd /path/to/flare
git pull
cargo install --locked --path .
```

### Manual build

```bash
git clone https://github.com/GitanElyon/flare.git
cd flare
cargo build --release
sudo install -Dm755 target/release/flare /usr/local/bin/flare
```

Use the manual path if you prefer to inspect the build artifacts yourself or package Flare for a distribution.

### Nix/NixOS

If you are running Nix enabled, you can run Flare directly:

```bash
nix run github:GitanElyon/flare
```

To install Flare to your profile:

```bash
nix profile install github:GitanElyon/flare
```

To update an existing installation:

```bash
nix profile upgrade flare
```

**Note for developers:** If you are developing Flare locally and using Nix Flakes, you **must** `git add` your changes before running `nix run .` or `nix profile install .`. Flakes only recognize files that are tracked by Git.

# Usage

You can run `flare` from your terminal, or set it up as a hotkey application launcher.

Flare can easily be used as an application launcher in place of `rofi` or `wofi`. To set it up, bind your desired hotkey to open a floating terminal running the `flare` command.

You can also use Flare to browse files or pass arguments to applications:
- **Launch with arguments**: Type the app name followed by arguments (e.g., `neovim ~/Documents/note.txt`).
- **Sudo Launch**: Type `sudo` before an application name to launch it with elevated privileges. You will be prompted for your password within Flare. Arguments like `sudo -E` are supported.
- **File Explorer**: Type a path starting with `~/` or `/` (e.g., `~/Projects/` or `/etc/`) to browse directories exclusively. Select a file and press Enter to open it with the default application (via `xdg-open`) or execute it if it's a binary.

Example for Hyprland config:

```conf
bind = $mod, space, exec, [float] $terminal -e flare
```

## Keybindings

| Key | Action |
| --- | --- |
| **Type** | Filter the application list |
| **Up / Down** | Navigate the list |
| **Left / Right** | Move cursor in input (edit text in-place) |
| **Alt + Up / Alt + Down** | Jump to top / bottom of list |
| **Alt + f** | Toggle favorite status |
| **Tab** | Auto-complete file paths |
| **Enter** | Launch selected application |
| **Esc** | Quit Flare |
| **Backspace** | Delete character from search |

# Configuration

As detailed in the [Flare Configuration Guide](./DOCS.md), Flare reads its configuration from `~/.config/flare/config.toml`. The file is created automatically the first time you run the launcher. Edits are hot-loaded on restart.

By default, Flare starts in app-launcher-only mode. Add plugins in `~/.config/flare/extention_config.toml` under `enabled = [...]` to activate extra modes.

[Currently]( https://github.com/pop-os/freedesktop-desktop-entry/blob/main/src/lib.rs#L656 ), Flare scans the following standard XDG directories:
- `/usr/share/applications`
- `/usr/local/share/applications`
- `~/.local/share/applications`

# License

Flare has an MIT license. Feel free to submit a PR.

