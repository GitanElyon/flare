# Overview

Flare is a customizable, lightweight, terminal-based application launcher for Linux. Built with Rust and Ratatui, it combines the visual list style of tools like `rofi` or `wofi` with the simplicity and speed of `dmenu`.

# Features

- **Fast scanning**: Automatically detects applications from standard `.desktop` file locations (`/usr/share/applications`, `~/.local/share/applications`).
- **Smart ordering**: Sorts applications by usage frequency, keeping your most used apps at the top.
- **TUI interface**: Clean, terminal-based user interface.
- **Instant filtering**: Real-time search filtering as you type.
- **File Explorer**: Browse and select files directly. Start with `~/` or `/` to search files exclusively, or type a path after an app name to pass it as an argument.
- **Launch Arguments**: Pass arguments to applications (e.g., `nvim ~/file.txt`).
- **Keyboard-centric**: Designed for efficiency with intuitive keybindings.
- **Highly customizable**: Extensive configuration options for appearance and behavior.

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

# Usage

You can run `flare` from your terminal, or set it up as a hotkey application launcher.

Flare can easily be used as an application launcher in place of `rofi` or `wofi`. To set it up, bind your desired hotkey to open a floating terminal running the `flare` command.

You can also use Flare to browse files or pass arguments to applications:
- **Launch with arguments**: Type the app name followed by arguments (e.g., `neovim ~/Documents/note.txt`).
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
| **Left / Right** | Jump to top / bottom of list |
| **Tab** | Auto-complete file paths |
| **Enter** | Launch selected application |
| **Esc** | Quit Flare |
| **Backspace** | Delete character from search |

# Configuration

As detailed in the [Flare Configuration Guide](./DOCS.md), Flare reads its configuration from `~/.config/flare/config.toml`. The file is created automatically the first time you run the launcher. Edits are hot-loaded on restart.

[Currently]( https://github.com/pop-os/freedesktop-desktop-entry/blob/main/src/lib.rs#L656 ), Flare scans the following standard XDG directories:
- `/usr/share/applications`
- `/usr/local/share/applications`
- `~/.local/share/applications`

# License

Flare has an MIT license. Feel free to submit a PR.

