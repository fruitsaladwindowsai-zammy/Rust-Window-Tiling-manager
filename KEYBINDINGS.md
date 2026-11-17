# Keybindings & Input Reference

WinScroller 2D uses a low-level Windows keyboard hook (`WH_KEYBOARD_LL`) so `Win+*` shortcuts work reliably without leaking the Windows Start Menu, while preserving reserved system shortcuts such as `Win+L` (Lock), `Win+E` (Explorer), `Win+R` (Run), and other unbound `Win+*` combinations.

> **Customize bindings:** Edit `%APPDATA%\WinScroller2D\winscroller2d.toml` (auto-reloaded on save) or check [CONFIGURATION.md](CONFIGURATION.md).

---

## Default Global Keybindings

### 2D Grid Navigation & Camera

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Left` | `focus` (`left`) | Focus the cell to the left and scroll camera |
| `Win+Right` | `focus` (`right`) | Focus the cell to the right and scroll camera |
| `Win+Up` | `focus` (`up`) | Focus the cell above and scroll camera |
| `Win+Down` | `focus` (`down`) | Focus the cell below and scroll camera |
| `Win+Ctrl+Left` | `pan_camera` (`left`) | Pan camera viewport left independently of focus |
| `Win+Ctrl+Right` | `pan_camera` (`right`) | Pan camera viewport right independently of focus |
| `Win+Ctrl+Up` | `pan_camera` (`up`) | Pan camera viewport up independently of focus |
| `Win+Ctrl+Down` | `pan_camera` (`down`) | Pan camera viewport down independently of focus |
| `Win+C` | `center_camera` | Center the camera on the currently focused cell |

### Moving & Swapping Windows

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Shift+Left` | `move_window` (`left`) | Move focused window left (preserves holes if `preserve_holes = true`, or swaps with occupied cell) |
| `Win+Shift+Right` | `move_window` (`right`) | Move focused window right |
| `Win+Shift+Up` | `move_window` (`up`) | Move focused window up |
| `Win+Shift+Down` | `move_window` (`down`) | Move focused window down |

### Cell Sizing & Presets

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Equal` (`Win+=`) | `resize_width` (`+1`) | Grow focused cell to the next width preset |
| `Win+Minus` (`Win+-`) | `resize_width` (`-1`) | Shrink focused cell to the previous width preset |
| `Win+Shift+Equal` (`Win+Shift+=`) | `resize_height` (`+1`) | Grow focused cell to the next height preset |
| `Win+Shift+Minus` (`Win+Shift+-`) | `resize_height` (`-1`) | Shrink focused cell to the previous height preset |
| `Win+Alt+R` | `cycle_width_preset` | Cycle through configured `width_steps` presets |
| `Win+Alt+Shift+R` | `cycle_height_preset` | Cycle through configured `height_steps` presets |

### Cell Stacking (Consume / Expel / Tabbed)

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Comma` (`Win+,`) | `consume_into_cell` (`right`) | Pull the neighboring right window into the focused cell stack |
| `Win+Period` (`Win+.`) | `expel_from_cell` (`right`) | Expel the active stacked window out to a neighboring cell |
| `Win+W` | `toggle_cell_stack_mode` | Toggle the focused cell between horizontal side-by-side split and tabbed stacking |

### Window & Cell Modes

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+F` | `toggle_maximize_cell` | Toggle maximizing the focused cell within the viewport |
| `Win+Shift+F` | `toggle_fullscreen` | Toggle borderless fullscreen for the focused window |
| `Win+Shift+Space` | `toggle_floating` | Toggle floating vs. tiled mode for the focused window |
| `Win+O` | `toggle_overview` | Open or close the interactive 2D **Win+O** grid overview (snapping, moving, closing) |
| `Win+Shift+Q` | `close_focused` | Close the currently focused window |

### Multi-Monitor & Virtual Desktop / Workspaces

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Alt+Left` | `focus_monitor` (`left`) | Focus the monitor to the left |
| `Win+Alt+Right` | `focus_monitor` (`right`) | Focus the monitor to the right |
| `Win+Alt+Shift+Left` | `move_to_monitor` (`left`) | Move focused window to the monitor on the left |
| `Win+Alt+Shift+Right` | `move_to_monitor` (`right`) | Move focused window to the monitor on the right |
| `Win+PageUp` | `switch_workspace_relative` (`-1`) | Switch to the previous workspace on the current monitor |
| `Win+PageDown` | `switch_workspace_relative` (`+1`) | Switch to the next workspace on the current monitor |
| `Win+Alt+1` .. `Win+Alt+9` | `switch_workspace` (`0..8`) | Switch directly to workspace `1..9` |
| `Win+Alt+Shift+1` .. `Win+Alt+Shift+9` | `move_to_workspace` (`0..8`) | Move focused window to workspace `1..9` |

### Control, Settings & Safety

| Shortcut | Action | Description |
|----------|--------|-------------|
| `Win+Alt+L` | `toggle_workspace_layout` | Toggle tiling vs. floating layout for the current workspace |
| `Win+Alt+S` | `open_settings` | Open the live **WinScroller 2D Settings** overlay window |
| `Win+Shift+R` | `retile` | Retile and resync managed windows to default geometry |
| `Win+Shift+P` | `toggle_pause` | Pause or resume tiling & hotkeys (only `Win+Shift+P` and `Win+Shift+Escape` stay active while paused) |
| `Win+Shift+Escape` | `quit` | Gracefully exit WinScroller 2D and restore managed windows |

---

## Interactive Overview (`Win+O`) Controls

When the **Win+O** overview is open, all windows on the active workspace scale dynamically into the display view with pure live thumbnails, centered app icons at the bottom middle, and smooth spring animations:

| Input | Behavior |
|-------|----------|
| **Hover** over a window card | Highlights the card and smoothly animates the red close button (white `✕`) into the top-right corner |
| **Click red `✕` button** | Closes that application directly from the overview |
| **Left-click** a window card | Animates the highlighted window to the front and exits overview |
| **Drag & Drop** onto another window | **Snaps** the two windows horizontally (50% / 50% width, 100% height) into one combined view |
| **Drag & Drop** onto an empty slot | Moves the window into that new coordinate as a standalone cell |
| **`Shift` + Drag & Drop** onto a window | Swaps the two windows' positions |
| **`Ctrl` + Drag & Drop** onto a window | Stacks the dragged window into the target cell |
| **Snapped views in overview** | Snapped views display both windows side-by-side as sub-cards with live thumbnails, icons, and individual close buttons |
| **Left-click** an empty slot | Jumps camera/focus to that empty grid coordinate and closes overview |
| `Left` / `Right` / `Up` / `Down` | Move overview highlight cursor across grid coordinates |
| `Enter` | Focus the highlighted cell and animate it to the front |
| `Esc` or **Right-click** | Dismiss overview without changing focus |

---

## Clean Borders & Clean Exit
- **Clean Display Borders:** 0px edge peek margin (`peek_x = 0, peek_y = 0`) guarantees that non-visible workspace columns and rows are cleanly cloaked with zero title bar slivers or edge artifacts in screen corners.
- **Clean Exit Centering:** When exiting WinScroller 2D (`Win+Shift+Escape` or system tray exit), all managed windows are cleanly uncloaked, unclipped, and centered in the monitor work area.

---

## Precision Touchpad Gestures

When `layout.three_finger_pan = true` (default):

| Gesture | Behavior |
|---------|----------|
| **3-finger pan** | Smooth 1:1 continuous 2D panning across the infinite grid (snaps or focuses neighbor on lift via `pan_commit`) |
| **4-finger swipe up** | Open **Win+O** overview |
| **4-finger swipe down** | Close **Win+O** overview |

---

## System Tray Controls

| Action | Behavior |
|--------|----------|
| **Double-click tray icon** | Toggle **2D Grid Overview (`Win+O`)** |
| **Right-click tray icon** | Open context menu: Overview, Settings (`Win+Alt+S`), Workspace 2D Lattice Mode (`Win+Alt+L`), Pause/Resume Tiling (`Win+Shift+P`), Retile/Sync Windows, Open Config, Reload Config, Exit & Restore Windows |

---

## Customizing Bindings in `winscroller2d.toml`

Keybindings are configured as `[[bindings]]` array-of-tables entries:

```toml
[[bindings]]
keys = "Win+Right"
action = "focus"
direction = "right"

[[bindings]]
keys = "Win+Shift+Q"
action = "close_focused"

[[bindings]]
keys = "Win+Alt+1"
action = "switch_workspace"
index = 0
```

### Supported Modifiers & Key Tokens

- **Modifiers:** `Win` (`Super`, `Meta`, `Mod`), `Ctrl` (`Control`), `Shift`, `Alt`
- **Letters & Digits:** `A`–`Z`, `0`–`9`
- **Function Keys:** `F1`–`F12`
- **Navigation & Editing:** `Left`, `Right`, `Up`, `Down`, `Space`, `Enter` (`Return`), `Escape` (`Esc`), `Tab`, `Backspace` (`Back`), `Delete` (`Del`), `Insert` (`Ins`), `Home`, `End`, `PageUp` (`PgUp`, `Prior`), `PageDown` (`PgDn`, `Next`)
- **Punctuation / Symbols:** `Equal` (`Equals`, `Plus`, `=`, `+`, plus numpad `+`), `Minus` (`Dash`, `-`, plus numpad `-`), `Comma` (`,`), `Period` (`Dot`, `.`), `BracketLeft` (`LBracket`, `[`), `Backslash` (`\`), `BracketRight` (`RBracket`, `]`)
- **Reserved:** `Win+L` (Windows Lock screen) is never intercepted.

### Available `action` Values

- Directional (`direction = "left" | "right" | "up" | "down"`): `focus`, `move_window`, `pan_camera`, `focus_monitor`, `move_to_monitor`
- Optional direction: `consume_into_cell`, `expel_from_cell`
- Delta (`delta = 1 | -1`): `resize_width`, `resize_height`, `switch_workspace_relative`
- Index (0-based `index = 0..`): `switch_workspace`, `move_to_workspace`
- No extra fields: `center_camera`, `cycle_width_preset`, `cycle_height_preset`, `toggle_cell_stack_mode`, `toggle_maximize_cell`, `toggle_fullscreen`, `toggle_floating`, `toggle_overview`, `close_focused`, `toggle_workspace_layout`, `open_settings`, `toggle_pause`, `retile`, `reload_config`, `quit`
