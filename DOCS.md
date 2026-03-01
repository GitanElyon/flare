# Flare Configuration Guide

Flare reads core UI/launcher configuration from `~/.config/flare/config.toml` and extension-specific configuration from `~/.config/flare/extention_config.toml`. Both files are created automatically.

## Getting Started

1. Launch Flare once so the default config is written.
2. Open `~/.config/flare/config.toml` for core UI/launcher settings.
3. Open `~/.config/flare/extention_config.toml` for extension toggles and extension-specific options.
4. Tweak the sections you care about (colors, borders, titles, etc.).
5. Restart Flare to see the new look. Keep the app running in one terminal while you edit in another to iterate quickly.

You can also copy configs between machines—Flare only cares that the TOML structure matches the sections described below.

> Tip: color fields accept either a single string (`"#cba6f7"`) or an array (`["#6464ff", "#c864ff"]`). Arrays with 2+ colors become gradients automatically.

## File Overview

```toml
[general]
rounded-corners = true
show-borders = true
highlight-symbol = "» "
favorite-symbol = "★ "
favorite-key = "alt+f"
jump-to-top-key = "alt+up"
jump-to-bottom-key = "alt+down"
clipboard-command = "wl-copy" # Optional: command to use for copying symbols (e.g., "wl-copy", "xclip -selection clipboard")

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
border-color = ["#cdd6f4"]

[input]
visible = true
title = " Search "
title-alignment = "center"
border-color = ["#6464ff", "#c864ff"] # 1 color = solid border, 2+ colors = gradient border
border-angle = 90

[flare-ascii]
visible = true
gradient-colors = ["#6464ff", "#c864ff"] # List of hex colors for the gradient
gradient-angle = 90 # 0-360 degrees (0 = left->right, 90 = top->bottom)
alignment = "center"
custom-path = "/home/user/.config/flare/flare.txt" # Optional: path to a custom ASCII art file

[flare-ascii.padding]
top = 0
bottom = 0
left = 0
right = 0

[list]
visible = true
title = " Applications "
apps-title = " Applications "
files-title = " Directories "
sudo-title = " Authentication "
title-alignment = "center"
border-color = ["#6464ff", "#c864ff"]
border-angle = 90

# Legacy alias: [results] still works

[entry]
fg = ["#f2f5f7"] # non-selected text color (use 2+ colors for gradient text)
bg = []           # optional non-selected background (use 2+ colors for gradient fill)
gradient-angle = 90

[entry-selected]
visible = true
fg = ["#151525"]
bg = ["#6464ff", "#c864ff"] # selected row highlight fill
gradient-angle = 90
full-width-highlight = true

[text]
visible = true
fg = "#f2f5f7"
alignment = "left"      # left | center | right
```

Every section shares a common set of optional keys:

| Key               | Type               | Description |
|-------------------|--------------------|-------------|
| `visible` / `visable` | bool          | Toggle rendering for that box. Useful for minimalist layouts or when embedding inside another launcher frame. |
| `fg` / `bg`       | color string or color array | Foreground/background colors. A single value gives a solid color; 2+ values create a gradient (where supported). Accepts named colors (`blue`, `light-red`, …) or hex `#RRGGBB` and `#RRGGBBAA`. |
| `gradient-angle`  | integer (`0..360`) | Angle used for `fg/bg` gradients. `0` = left→right, `90` = top→bottom, `180` = right→left, `270` = bottom→top. |
| `border-color`    | color string or color array | Border colors. One color draws a solid border; multiple colors automatically draw a gradient border. |
| `border-angle`    | integer (`0..360`) | Angle of border gradient travel when `border-color` has multiple colors. |
| `borders`         | bool               | Override the global `general.show-borders` toggle for an individual section. |
| `rounded`         | bool               | Override the global `general.rounded-corners` toggle. |
| `title`           | string             | Optional text shown in the block header. |
| `title-alignment` | enum (`left`, `center`, `right`) | How the block title (e.g. “Flare”, “Search”, “Applications”) is aligned within the border. |

Additional section-specific options:

- `general.highlight-symbol`: string drawn in front of the selected entry. Set to an empty string (or disable `entry-selected.visible`) to hide it.
- `text.alignment`: aligns entry labels within the list (`left`, `center`, `right`).
- `entry.fg` / `entry.bg`: Colors for non-selected rows. Arrays allow gradients. These override `text.fg`/`text.bg` for normal rows.
- `flare-ascii.gradient-colors`: A list of color strings (e.g. `["#ff0000", "#00ff00"]`) to use for the ASCII color. One color gives solid text; multiple colors interpolate along `gradient-angle`.
- `flare-ascii.gradient-angle`: Angle of the ASCII gradient in degrees (`0..360`).
- `flare-ascii.alignment`: Alignment of the ASCII art (`left`, `center`, `right`).
- `flare-ascii.custom-path`: Absolute path to a file containing custom ASCII art to display.
- `flare-ascii.padding`: Sub-section with `top`, `bottom`, `left`, `right` (integers) to add space around the art.
- `entry-selected.full-width-highlight`: When `true`, selected-row highlighting fills the full width of the list box.
- `list.apps-title`: Title shown when browsing applications. Defaults to " Applications ".
- `list.files-title`: Title shown when browsing files/directories. Defaults to " Directories ".
- `list.sudo-title`: Title shown when prompting for sudo password. Defaults to " Authentication ".
- `general.clipboard-command`: Optional shell command to use for copying symbols. If not set, Flare uses an internal clipboard library. Example: `"wl-copy"` or `"xclip -selection clipboard"`.

## NixOS Installation

### Using Flakes
Add Flare to your `flake.nix` inputs:

```nix
inputs.flare.url = "github:GitanElyon/flare";
```

Then add it to your `environment.systemPackages`:

```nix
environment.systemPackages = [
  inputs.flare.packages.${pkgs.system}.default
];
```

### Without Flakes
You can use `fetchTarball` to include Flare in your `configuration.nix`:

```nix
let
  flare = import (fetchTarball "https://github.com/GitanElyon/flare/archive/main.tar.gz") {};
in {
  environment.systemPackages = [ flare ];
}
```

### General Section

The `[general]` block controls defaults for the rest of the UI:

- `rounded-corners`: switches every visible border between plain and rounded corners. Individual sections can opt out via `rounded = false`.
- `show-borders`: quick way to remove all borders. Override per section with `borders = true/false` when you want one box framed but another bare.
- `highlight-symbol`: string prepended to the focused entry. Multi-character strings work fine—emoji too, if your font supports them.
- `favorite-symbol`: string displayed next to favorite applications and Nerd Font symbols. Defaults to "★ ".
- `favorite-key`: keybinding to toggle favorite status. Supports modifiers (ctrl, alt, shift) and keys (a-z, f1-f12, enter, etc.). Defaults to "alt+f".
- `jump-to-top-key`: keybinding to jump directly to the first result row. Defaults to `alt+up`.
- `jump-to-bottom-key`: keybinding to jump directly to the last result row. Defaults to `alt+down`.

Input editing notes:
- `Left` / `Right` now move the text cursor in the input field so you can edit in-place (including calculator expressions).
- `Up` / `Down` still move through results one row at a time.
- Use `jump-to-top-key` / `jump-to-bottom-key` for instant top/bottom jumps.

### Features Section

The `[features]` block allows you to toggle specific functionalities:

- `enable-launch-args`: Enables passing arguments to applications (e.g., `nvim file.txt`).
- `enable-auto-complete`: Enables tab auto-completion for file paths.
- `dirs-first`: When listing files, show directories before files. Defaults to `true`.
- `show-duplicates`: Shows duplicate entries when the same application appears in multiple locations (e.g., both `/usr/share/applications` and `~/.local/share/applications`). Set to `true` to show all instances, or `false` to show only the first occurrence. Defaults to `false`.
- `recent-first`: Sorts applications by usage frequency. Defaults to `true`.

### Extension Config File (`extention_config.toml`)

Extension toggles and extension-specific options live in `~/.config/flare/extention_config.toml`.

```toml
enabled = ["calculator", "symbols", "files", "sudo", "help", "clipboard"]

[calculator]
trigger = "="
replace-symbols = false
fancy-numbers = false

[symbols]
trigger = "."

[help]
trigger = "-"

[clipboard]
trigger = "+"
prefer-external-history-tools = true
```

- `enabled`: List of extension IDs. If empty, Flare behaves as an app launcher only.
- `calculator.trigger`: Trigger for calculator mode. Defaults to `=`.
- `calculator.replace-symbols`: Replace calculator symbols in rendered output.
- `calculator.fancy-numbers`: Use fancy rendered numeric glyph formatting.
- `symbols.trigger`: Trigger for symbol picker mode. Defaults to `.`.
- `help.trigger`: Trigger for command/help list. Defaults to `-`.
- `clipboard.trigger`: Trigger for clipboard history mode. Defaults to `+`.
- `query_example` (optional): Extensions can provide an example query string in their metadata to help populate the search bar when users select the extension from a list (for example the help extension provides sample queries like `sudo ` or `~/`).
- `clipboard.prefer-external-history-tools`: Prefer `wl-clipboard-history`, `cliphist`, or `copyq` before falling back to Flare local clipboard history.

Supported IDs:
- `calculator`
- `symbols`
- `clipboard`
- `files`
- `sudo`
- `help`
 - `runner`
 - `volume`
 - `bluetooth`

Aliases are accepted for compatibility (`calc`, `icon-picker`, `directory-browser`, `bt`, etc.).

### Runner (extension id: `runner`)

- **Trigger:** `>` — prefix a query with `>` to run an arbitrary shell command. Example: `> echo hello`.
- Pressing Enter executes the command via `sh -c` and the launcher exits (this extension returns an `ExecuteAndExit` action).

### Volume (extension id: `volume`)

- **Trigger:** `v!` — open the volume control menu.
- **Help:** `v! -h` shows the available volume commands.
- **Commands:** `v! +N` (increase by N%), `v! -N` (decrease by N%), `v! N` (set level to N%), `v! mute` (toggle mute), `v! devices` (list/switch outputs).
- Flare attempts to detect available audio tooling in this order: `wpctl` (PipeWire), `pactl` (PulseAudio), `amixer` (ALSA). For each backend Flare issues appropriate commands so the same `v!` verbs work across systems.

### Bluetooth (extension id: `bluetooth`)

- **Trigger:** `b!` — open the bluetooth manager.
- **Help:** `b! -h` shows the available bluetooth commands.
- **Commands:** `b! power on/off`, `b! scan on/off`, or interact with a specific device via `b! <MAC>`.
- Allows querying device connection and pair statuses, running operations like Connect, Pair, Disconnect, Trust, and Remove from an easy to use sub-menu. Powered by `bluetoothctl` under the hood.

Clipboard extension notes:
- Flare will try installed clipboard history tools first (`wl-clipboard-history`, `cliphist`, `copyq`).
- If none are available, it falls back to Flare's local history file at `~/.config/flare/clip_history.toml`.

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
| `flare-ascii` | ASCII art header |
| `input`      | Search field area |
| `list`     | Box around the list itself |
| `entry`       | Non-selected rows in the results list |
| `entry-selected` | Highlight style for the active row |
| `text`       | Program name span inside each row |

Each section can be hidden (`visible = false`) to remove it entirely—for example, disable `outer-box` and `window` to embed Flare inside another tiling window or hide `input` to create a command-palette style overlay.

## Calculator

Flare includes a built-in symbolic calculator. Trigger it by typing `=` at the start of your query.

### Supported Operations

- **Basic Math**: `+`, `-`, `*`, `/`, `^` (power)
- **Functions**:
  - `sqrt(x)`: Square root
  - `log(x)`: Logarithm (base 10)
  - `log(x, base)`: Logarithm with specified base
  - `ln(x)`: Natural logarithm
  - `abs(x)`: Absolute value
  - `sin(x)`, `cos(x)`, `tan(x)`: Trigonometric functions
- **Calculus**:
  - `diff(expr, var)`: Differentiate expression with respect to variable.
    - Example: `= diff(x^2, x)` -> `2 * x`
  - `integrate(expr, var)`: Indefinite integral.
    - Example: `= integrate(2*x, x)` -> `x^2`
  - `integrate(expr, var, a, b)`: Definite integral from a to b.
    - Example: `= integrate(2*x, x, 0, 5)` -> `25`
  - `limit(expr, var, to)`: Limit of expression as var approaches value.

### Configuration

- `features.replace-calc-symbols`: When enabled, replaces function names with mathematical symbols in the history view (e.g., `sqrt` -> `√`, `integrate` -> `∫`).
- `features.fancy-numbers`: When enabled, formats exponents as superscripts (e.g., `x^2` -> `x²`).

## Error Handling

If `config.toml` is missing or invalid, Flare falls back to the built-in defaults, writes a fresh template to `~/.config/flare/config.toml`, prints a warning before launching the TUI, and shows the warning inside the interface so you know the config needs attention.