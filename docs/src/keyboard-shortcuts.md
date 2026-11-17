# Keybindings & Gestures

WinScroller 2D intercepts shortcuts with `WH_KEYBOARD_LL` and suppresses accidental Start Menu activations while preserving `Win+L` (Lock), `Win+E`, `Win+R`, and all unbound Windows shortcuts.

## Global Keybindings

### 2D Navigation & Camera

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Left` / `Right` / `Up` / `Down` | `focus` | Focus neighboring cell and scroll camera |
| `Win+Ctrl+Left` / `Right` / `Up` / `Down` | `pan_camera` | Pan camera independently of focus |
| `Win+C` | `center_camera` | Center camera on the focused cell |

### Window Movement & Sizing

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Shift+Left` / `Right` / `Up` / `Down` | `move_window` | Move or swap focused window in the 2D grid |
| `Win+Equal` / `Win+Minus` | `resize_width` | Step focused cell width up/down (`width_steps`) |
| `Win+Shift+Equal` / `Win+Shift+Minus` | `resize_height` | Step focused cell height up/down (`height_steps`) |
| `Win+Alt+R` | `cycle_width_preset` | Cycle through `width_steps` |
| `Win+Alt+Shift+R` | `cycle_height_preset` | Cycle through `height_steps` |

### Cell Stacking & Window Modes

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Comma` | `consume_into_cell` | Consume neighboring right window into focused cell |
| `Win+Period` | `expel_from_cell` | Expel active stacked window out to a neighbor cell |
| `Win+W` | `toggle_cell_stack_mode` | Switch cell between horizontal side-by-side split and tabbed stacking |
| `Win+F` | `toggle_maximize_cell` | Maximize/restore focused cell inside viewport |
| `Win+Shift+F` | `toggle_fullscreen` | Toggle borderless fullscreen |
| `Win+Shift+Space` | `toggle_floating` | Toggle floating vs. tiled mode |
| `Win+O` | `toggle_overview` | Open or close the dynamic 2D **Overview** |
| `Win+Shift+Q` | `close_focused` | Close the focused window |

### Workspaces & Monitors

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Alt+Left` / `Win+Alt+Right` | `focus_monitor` | Focus adjacent monitor |
| `Win+Alt+Shift+Left` / `Right` | `move_to_monitor` | Move focused window to adjacent monitor |
| `Win+PageUp` / `Win+PageDown` | `switch_workspace_relative` | Switch to previous/next workspace |
| `Win+Alt+1` .. `Win+Alt+9` | `switch_workspace` | Switch to workspace `1..9` |
| `Win+Alt+Shift+1` .. `9` | `move_to_workspace` | Move focused window to workspace `1..9` |

### Control & Safety

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Alt+L` | `toggle_workspace_layout` | Toggle 2D lattice tiling vs. floating on current workspace |
| `Win+Alt+S` | `open_settings` | Open the live Settings UI |
| `Win+Shift+R` | `retile` | Retile and resync managed windows |
| `Win+Shift+P` | `toggle_pause` | Pause/resume tiling and hotkeys |
| `Win+Shift+Escape` | `quit` | Exit WinScroller 2D and restore managed windows |

## Overview Mode (`Win+O`) Controls

| Input | Effect |
|-------|--------|
| **Hover card** | Highlight card and reveal red top-right `✕` close button |
| **Click red `✕`** | Close that window directly |
| **Left-click card** | Animate window to front and exit overview |
| **Drag & Drop onto a window** | Snap both windows horizontally (50% / 50%) into one combined view |
| **Drag & Drop onto an empty slot** | Move window into that coordinate as a standalone cell |
| **`Shift` + Drag & Drop** | Swap the two windows' positions |
| **`Ctrl` + Drag & Drop** | Stack dragged window into the target cell |
| **Arrow keys + `Enter`** | Move highlight cursor and activate cell |
| **`Esc` or Right-click** | Dismiss overview |

## Precision Touchpad & Tray

- **3-finger pan**: Smooth 1:1 continuous 2D panning (`layout.three_finger_pan = true`).
- **4-finger swipe up / down**: Open / close `Win+O` overview.
- **Tray double-click**: Toggle `Win+O` overview.
- **Tray right-click**: Access Overview, Settings, Workspace Lattice Mode, Pause/Resume, Retile, Open Config, Reload Config, and Exit.
