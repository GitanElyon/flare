# Flare Configuration Guide

Flare reads its configuration from `~/.config/flare/config.toml`. The file is created automatically the first time you run the launcher. Edits are hot-loaded on restart.

## Getting Started

1. Launch Flare once so the default config is written.
2. Open `~/.config/flare/config.toml` in your editor of choice.
3. Tweak the sections you care about (colors, borders, titles, etc.).
4. Restart Flare to see the new look. Keep the app running in one terminal while you edit in another to iterate quickly.

You can also copy configs between machines—Flare only cares that the TOML structure matches the sections described below.

## File Overview

```toml
[general]
rounded-corners = true
show-borders = true
highlight-symbol = "» "

[features]
enable-file-explorer = true
enable-launch-args = true
enable-auto-complete = true
dirs-first = true
show-duplicates = false
recent-first = true

[window]
visible = false          # alias: visable = true
bg = "#0f0f17ff"        # rrggbbAA (alpha optional)

[outer-box]
visible = false
title = " Flare "
title-alignment = "center"
border-color = "#cdd6f4"

[input]
visible = true
title = " Search "
title-alignment = "center"
border-color = "#cba6f7"

[scroll]
visible = false
border-color = "#585b70"

[inner-box]
visible = true
title = " Applications "
title-alignment = "center"
border-color = "#89b4fa"

[entry]
visible = false
bg = "#1e1e2e"

[entry-selected]
visible = true
fg = "#151525"
bg = "#6d7694"

[text]
visible = true
fg = "#f2f5f7"
alignment = "left"      # left | center | right
```

Every section shares a common set of optional keys:

| Key               | Type               | Description |
|-------------------|--------------------|-------------|
| `visible` / `visable` | bool          | Toggle rendering for that box. Useful for minimalist layouts or when embedding inside another launcher frame. |
| `fg` / `bg`       | color string       | Foreground/background color. Accepts named colors (`blue`, `light-red`, …) or hex values `#RRGGBB` and `#RRGGBBAA`. The extra `AA` channel controls opacity; Flare blends it against the terminal background. |
| `border-color`    | color string       | Border color with the same syntax as `fg/bg`. |
| `borders`         | bool               | Override the global `general.show-borders` toggle for an individual section. |
| `rounded`         | bool               | Override the global `general.rounded-corners` toggle. |
| `title`           | string             | Optional text shown in the block header. |
| `title-alignment` | enum (`left`, `center`, `right`) | How the block title (e.g. “Flare”, “Search”, “Applications”) is aligned within the border. |

Additional section-specific options:

- `general.highlight-symbol`: string drawn in front of the selected entry. Set to an empty string (or disable `entry-selected.visible`) to hide it.
- `text.alignment`: aligns entry labels within the list (`left`, `center`, `right`).

### General Section

The `[general]` block controls defaults for the rest of the UI:

- `rounded-corners`: switches every visible border between plain and rounded corners. Individual sections can opt out via `rounded = false`.
- `show-borders`: quick way to remove all borders. Override per section with `borders = true/false` when you want one box framed but another bare.
- `highlight-symbol`: string prepended to the focused entry. Multi-character strings work fine—emoji too, if your font supports them.

### Features Section

The `[features]` block allows you to toggle specific functionalities:

- `enable-file-explorer`: Enables file browsing. When typing a path starting with `~/` or `/`, Flare switches to file-only mode. Also enables path completion for launch arguments.
- `enable-launch-args`: Enables passing arguments to applications (e.g., `nvim file.txt`).
- `enable-auto-complete`: Enables tab auto-completion for file paths.
- `dirs-first`: When listing files, show directories before files. Defaults to `true`.
- `show-duplicates`: Shows duplicate entries when the same application appears in multiple locations (e.g., both `/usr/share/applications` and `~/.local/share/applications`). Set to `true` to show all instances, or `false` to show only the first occurrence. Defaults to `false`.
- `recent-first`: Sorts applications by usage frequency. Defaults to `true`.

### Color Syntax

- Hex colors use `#RRGGBB` or `#RRGGBBAA`. The optional `AA` alpha channel lets you get subtler shades against the terminal background.
- Named colors accept the same set Ratatui exposes (`blue`, `light-red`, `gray`, etc.). Unknown names are ignored, so double-check spelling if a color does not change.
- When alpha is present, Flare pre-multiplies it before drawing, so `#1e1e2e80` produces a translucent version of the same hue.

## Visual Structure

Flare mirrors common wofi/rofi selectors. Sections map to UI elements as follows:

| Section      | Applies to |
|--------------|------------|
| `window`     | Whole terminal window background |
| `outer-box`  | Frame that wraps the UI |
| `input`      | Search field area |
| `scroll`     | Scrollable viewport containing the entry list |
| `inner-box`  | Box around the list itself |
| `entry`      | Individual list rows |
| `entry-selected` | Highlight style for the active row |
| `text`       | Program name span inside each row |

Each section can be hidden (`visible = false`) to remove it entirely—for example, disable `outer-box` and `window` to embed Flare inside another tiling window or hide `input` to create a command-palette style overlay.

## Error Handling

If `config.toml` is missing or invalid, Flare falls back to the built-in defaults, writes a fresh template to `~/.config/flare/config.toml`, prints a warning before launching the TUI, and shows the warning inside the interface so you know the config needs attention.
