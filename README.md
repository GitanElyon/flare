# Overview

Flare is a lightweight, terminal-based application launcher for Linux. Built with Rust and Ratatui, it combines the visual list style of tools like `rofi` or `wofi` with the simplicity and speed of `dmenu`.

# Features

- **Fast Scanning**: Automatically detects applications from standard `.desktop` file locations (`/usr/share/applications`, `~/.local/share/applications`).
- **TUI Interface**: Clean, terminal-based user interface.
- **Instant Filtering**: Real-time search filtering as you type.
- **Keyboard Centric**: Designed for efficiency with intuitive keybindings.

# Installation

Ensure you have Rust and Cargo installed.

```bash
git clone https://github.com/yourusername/flare.git
cd flare
cargo build --release
```

The binary will be located at `target/release/flare`. You can copy this to somewhere in your `$PATH` (e.g., `/usr/local/bin`).

I will streamline this process before the first offial release.

# Usage

Run `flare` from your terminal. It works best when bound to a global hotkey in your window manager (e.g., `Super+D` or `Alt+Space`).

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

