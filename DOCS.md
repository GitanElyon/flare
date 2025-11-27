# Flare Configuration Guide

Flare reads its configuration from `~/.config/flare/config.toml`. The file is created automatically the first time you run the launcher. Edits are hot-loaded on restart.

## File Overview

```toml
[[general]
rounded-corners = true
show-borders = true
highlight-symbol = "» "

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
