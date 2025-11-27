# Overview

Flare is a customizable, lightweight, terminal-based application launcher for Linux. Built with Rust and Ratatui, it combines the visual list style of tools like `rofi` or `wofi` with the simplicity and speed of `dmenu`.

# Features

- **Fast Scanning**: Automatically detects applications from standard `.desktop` file locations (`/usr/share/applications`, `~/.local/share/applications`).
- **TUI Interface**: Clean, terminal-based user interface.
- **Instant Filtering**: Real-time search filtering as you type.
- **Keyboard Centric**: Designed for efficiency with intuitive keybindings.
- **Highly Customizable**: Extensive configuration options for appearance and behavior.

# Installation

Ensure you have a recent stable Rust toolchain (1.77+) installed via [rustup](https://rustup.rs/).

### Quick install (recommended)

```bash
git clone https://github.com/yourusername/flare.git
cd flare
cargo install --locked --path .
```

This places the `flare` binary in `~/.cargo/bin`, which is already on your `$PATH` if you installed Rust via rustup. Update to the latest commit any time with:

```bash
cd /path/to/flare
git pull
cargo install --locked --path .
```

### Manual build

```bash
git clone https://github.com/yourusername/flare.git
cd flare
cargo build --release
sudo install -Dm755 target/release/flare /usr/local/bin/flare
```

Use the manual path if you prefer to inspect the build artifacts yourself or package Flare for a distribution.

# Usage

You can either run `flare` from your terminal, or set it up as a hotkey application launcher.

Flare can easily be used as an application launcher in place of `rofi` or `wofi`. To set it up, bind your desired hotkey to open a floating terminal running the `flare` command.

Example in my hyprland config:

```bash
bind = $mod, space, exec, [float] $terminal -e flare
```

## Keybindings

| Key | Action |
| --- | --- |
| **Type** | Filter the application list |
| **Up / Down** | Navigate the list |
| **Enter** | Launch selected application |
| **Esc** | Quit Flare |
| **Backspace** | Delete character from search |

# Configuration

Currently, Flare scans the following standard XDG directories:
- `/usr/share/applications`
- `/usr/local/share/applications`
- `~/.local/share/applications`

# License

Flare has an MIT license, so feel free to submit a PR.

